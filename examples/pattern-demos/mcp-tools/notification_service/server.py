#!/usr/bin/env python3
"""MCP Notification Service - Async notification patterns demo.

This MCP server provides notification capabilities with mock channel delivery,
demonstrating async patterns including queuing and dead-letter handling.

Tools:
    - send_notification: Queue a notification for delivery
    - get_notifications: Retrieve notifications for a user
    - mark_read: Mark notifications as read

Channels (mock implementations):
    - email: Mock email delivery with configurable failure rates
    - slack: Mock Slack webhook delivery
    - in_app: In-app notification storage
"""

from __future__ import annotations

import asyncio
import logging
import random
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any

from mcp.server.fastmcp import FastMCP

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class NotificationStatus(str, Enum):
    """Status of a notification."""

    PENDING = "pending"
    QUEUED = "queued"
    DELIVERED = "delivered"
    FAILED = "failed"
    READ = "read"


class Channel(str, Enum):
    """Supported notification channels."""

    EMAIL = "email"
    SLACK = "slack"
    IN_APP = "in_app"


class Priority(str, Enum):
    """Notification priority levels."""

    LOW = "low"
    NORMAL = "normal"
    HIGH = "high"
    URGENT = "urgent"


@dataclass
class Notification:
    """A notification message."""

    id: str
    user_id: str
    channel: Channel
    subject: str
    body: str
    priority: Priority = Priority.NORMAL
    status: NotificationStatus = NotificationStatus.PENDING
    created_at: datetime = field(default_factory=lambda: datetime.now(timezone.utc))
    delivered_at: datetime | None = None
    read_at: datetime | None = None
    retry_count: int = 0
    max_retries: int = 3
    last_error: str | None = None
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        return {
            "id": self.id,
            "user_id": self.user_id,
            "channel": self.channel.value,
            "subject": self.subject,
            "body": self.body,
            "priority": self.priority.value,
            "status": self.status.value,
            "created_at": self.created_at.isoformat(),
            "delivered_at": self.delivered_at.isoformat() if self.delivered_at else None,
            "read_at": self.read_at.isoformat() if self.read_at else None,
            "retry_count": self.retry_count,
            "last_error": self.last_error,
            "metadata": self.metadata,
        }


@dataclass
class DeadLetterEntry:
    """Entry in the dead-letter queue."""

    notification: Notification
    reason: str
    failed_at: datetime = field(default_factory=lambda: datetime.now(timezone.utc))

    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        return {
            "notification": self.notification.to_dict(),
            "reason": self.reason,
            "failed_at": self.failed_at.isoformat(),
        }


class NotificationStore:
    """In-memory notification storage with queue support."""

    def __init__(self) -> None:
        self.notifications: dict[str, Notification] = {}
        self.delivery_queue: asyncio.Queue[Notification] = asyncio.Queue()
        self.dead_letter_queue: list[DeadLetterEntry] = []
        self._processing = False

    def add(self, notification: Notification) -> None:
        """Add a notification to the store."""
        self.notifications[notification.id] = notification

    def get(self, notification_id: str) -> Notification | None:
        """Get a notification by ID."""
        return self.notifications.get(notification_id)

    def get_for_user(
        self,
        user_id: str,
        channel: Channel | None = None,
        status: NotificationStatus | None = None,
        limit: int = 50,
    ) -> list[Notification]:
        """Get notifications for a user with optional filters."""
        results = []
        for n in self.notifications.values():
            if n.user_id != user_id:
                continue
            if channel and n.channel != channel:
                continue
            if status and n.status != status:
                continue
            results.append(n)
        # Sort by created_at descending (newest first)
        results.sort(key=lambda x: x.created_at, reverse=True)
        return results[:limit]

    async def enqueue(self, notification: Notification) -> None:
        """Add notification to delivery queue."""
        notification.status = NotificationStatus.QUEUED
        await self.delivery_queue.put(notification)

    def add_to_dead_letter(self, notification: Notification, reason: str) -> None:
        """Add failed notification to dead-letter queue."""
        entry = DeadLetterEntry(notification=notification, reason=reason)
        self.dead_letter_queue.append(entry)
        notification.status = NotificationStatus.FAILED
        logger.warning(f"Notification {notification.id} moved to dead-letter queue: {reason}")


class MockChannelHandler:
    """Mock channel delivery handlers."""

    # Configurable failure rates for testing
    EMAIL_FAILURE_RATE = 0.1  # 10% of emails fail
    SLACK_FAILURE_RATE = 0.05  # 5% of slack messages fail
    INAPP_FAILURE_RATE = 0.0  # In-app never fails

    @classmethod
    async def deliver(cls, notification: Notification) -> tuple[bool, str | None]:
        """Deliver notification via appropriate channel.

        Returns:
            Tuple of (success, error_message)
        """
        # Simulate network latency
        await asyncio.sleep(random.uniform(0.05, 0.2))

        if notification.channel == Channel.EMAIL:
            return await cls._deliver_email(notification)
        elif notification.channel == Channel.SLACK:
            return await cls._deliver_slack(notification)
        elif notification.channel == Channel.IN_APP:
            return await cls._deliver_in_app(notification)
        else:
            return False, f"Unknown channel: {notification.channel}"

    @classmethod
    async def _deliver_email(cls, notification: Notification) -> tuple[bool, str | None]:
        """Mock email delivery."""
        if random.random() < cls.EMAIL_FAILURE_RATE:
            return False, "SMTP connection timeout (mock failure)"

        logger.info(
            f"[EMAIL] Sent to user {notification.user_id}: "
            f"Subject='{notification.subject}'"
        )
        return True, None

    @classmethod
    async def _deliver_slack(cls, notification: Notification) -> tuple[bool, str | None]:
        """Mock Slack webhook delivery."""
        if random.random() < cls.SLACK_FAILURE_RATE:
            return False, "Slack API rate limited (mock failure)"

        logger.info(
            f"[SLACK] Posted for user {notification.user_id}: "
            f"'{notification.subject}'"
        )
        return True, None

    @classmethod
    async def _deliver_in_app(cls, notification: Notification) -> tuple[bool, str | None]:
        """Mock in-app notification storage."""
        if random.random() < cls.INAPP_FAILURE_RATE:
            return False, "Database connection error (mock failure)"

        logger.info(
            f"[IN_APP] Stored for user {notification.user_id}: "
            f"'{notification.subject}'"
        )
        return True, None


class NotificationProcessor:
    """Background processor for notification delivery queue."""

    def __init__(self, store: NotificationStore) -> None:
        self.store = store
        self._task: asyncio.Task | None = None

    async def start(self) -> None:
        """Start the background processor."""
        if self._task is None:
            self._task = asyncio.create_task(self._process_loop())
            logger.info("Notification processor started")

    async def stop(self) -> None:
        """Stop the background processor."""
        if self._task:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass
            self._task = None
            logger.info("Notification processor stopped")

    async def _process_loop(self) -> None:
        """Main processing loop."""
        while True:
            try:
                notification = await self.store.delivery_queue.get()
                await self._process_notification(notification)
            except asyncio.CancelledError:
                raise
            except Exception as e:
                logger.error(f"Error in processor loop: {e}")

    async def _process_notification(self, notification: Notification) -> None:
        """Process a single notification."""
        success, error = await MockChannelHandler.deliver(notification)

        if success:
            notification.status = NotificationStatus.DELIVERED
            notification.delivered_at = datetime.now(timezone.utc)
            logger.info(f"Notification {notification.id} delivered successfully")
        else:
            notification.retry_count += 1
            notification.last_error = error

            if notification.retry_count < notification.max_retries:
                # Re-queue for retry with exponential backoff
                delay = 2 ** notification.retry_count
                logger.info(
                    f"Notification {notification.id} failed, retry {notification.retry_count}/{notification.max_retries} in {delay}s"
                )
                await asyncio.sleep(delay)
                await self.store.enqueue(notification)
            else:
                # Move to dead-letter queue
                self.store.add_to_dead_letter(
                    notification,
                    f"Max retries ({notification.max_retries}) exceeded. Last error: {error}",
                )


# Initialize the MCP server
mcp = FastMCP(
    name="notification-service",
    instructions="""Notification Service MCP Server

This server provides notification capabilities with support for multiple channels
(email, slack, in-app) and async delivery patterns. Notifications are queued and
processed asynchronously with retry logic and dead-letter handling.

Available tools:
- send_notification: Send a notification to a user
- get_notifications: Get notifications for a user
- mark_read: Mark a notification as read
- get_dead_letter_queue: View failed notifications""",
)

# Initialize stores and processor
store = NotificationStore()
processor = NotificationProcessor(store)


@mcp.tool()
async def send_notification(
    user_id: str,
    subject: str,
    body: str,
    channel: str = "in_app",
    priority: str = "normal",
    metadata: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Send a notification to a user.

    Args:
        user_id: The user to notify
        subject: Notification subject/title
        body: Notification body/content
        channel: Delivery channel - "email", "slack", or "in_app" (default: "in_app")
        priority: Priority level - "low", "normal", "high", or "urgent" (default: "normal")
        metadata: Optional additional metadata

    Returns:
        Notification details including ID and status
    """
    # Validate channel
    try:
        ch = Channel(channel.lower())
    except ValueError:
        return {
            "success": False,
            "error": f"Invalid channel '{channel}'. Must be one of: email, slack, in_app",
        }

    # Validate priority
    try:
        prio = Priority(priority.lower())
    except ValueError:
        return {
            "success": False,
            "error": f"Invalid priority '{priority}'. Must be one of: low, normal, high, urgent",
        }

    # Create notification
    notification = Notification(
        id=str(uuid.uuid4()),
        user_id=user_id,
        channel=ch,
        subject=subject,
        body=body,
        priority=prio,
        metadata=metadata or {},
    )

    # Store and queue for delivery
    store.add(notification)
    await store.enqueue(notification)

    # Ensure processor is running
    await processor.start()

    return {
        "success": True,
        "notification": notification.to_dict(),
        "message": f"Notification queued for delivery via {channel}",
    }


@mcp.tool()
async def get_notifications(
    user_id: str,
    channel: str | None = None,
    status: str | None = None,
    limit: int = 50,
) -> dict[str, Any]:
    """Get notifications for a user.

    Args:
        user_id: The user ID to get notifications for
        channel: Optional filter by channel ("email", "slack", "in_app")
        status: Optional filter by status ("pending", "queued", "delivered", "failed", "read")
        limit: Maximum number of notifications to return (default: 50)

    Returns:
        List of notifications matching the criteria
    """
    # Validate channel if provided
    ch = None
    if channel:
        try:
            ch = Channel(channel.lower())
        except ValueError:
            return {
                "success": False,
                "error": f"Invalid channel '{channel}'. Must be one of: email, slack, in_app",
            }

    # Validate status if provided
    st = None
    if status:
        try:
            st = NotificationStatus(status.lower())
        except ValueError:
            return {
                "success": False,
                "error": f"Invalid status '{status}'. Must be one of: pending, queued, delivered, failed, read",
            }

    notifications = store.get_for_user(user_id, channel=ch, status=st, limit=limit)

    return {
        "success": True,
        "user_id": user_id,
        "count": len(notifications),
        "notifications": [n.to_dict() for n in notifications],
    }


@mcp.tool()
async def mark_read(notification_id: str) -> dict[str, Any]:
    """Mark a notification as read.

    Args:
        notification_id: The notification ID to mark as read

    Returns:
        Updated notification details
    """
    notification = store.get(notification_id)
    if not notification:
        return {
            "success": False,
            "error": f"Notification '{notification_id}' not found",
        }

    notification.status = NotificationStatus.READ
    notification.read_at = datetime.now(timezone.utc)

    return {
        "success": True,
        "notification": notification.to_dict(),
        "message": "Notification marked as read",
    }


@mcp.tool()
async def get_dead_letter_queue(limit: int = 50) -> dict[str, Any]:
    """Get notifications from the dead-letter queue.

    These are notifications that failed delivery after all retry attempts.
    Useful for debugging and manual intervention.

    Args:
        limit: Maximum number of entries to return (default: 50)

    Returns:
        List of dead-letter queue entries with failure details
    """
    entries = store.dead_letter_queue[-limit:]

    return {
        "success": True,
        "count": len(entries),
        "total_in_queue": len(store.dead_letter_queue),
        "entries": [e.to_dict() for e in entries],
    }


@mcp.tool()
async def retry_dead_letter(notification_id: str) -> dict[str, Any]:
    """Retry a notification from the dead-letter queue.

    Args:
        notification_id: The notification ID to retry

    Returns:
        Status of the retry operation
    """
    # Find in dead-letter queue
    for i, entry in enumerate(store.dead_letter_queue):
        if entry.notification.id == notification_id:
            notification = entry.notification
            # Reset retry count and re-queue
            notification.retry_count = 0
            notification.last_error = None
            notification.status = NotificationStatus.PENDING
            await store.enqueue(notification)

            # Remove from dead-letter queue
            store.dead_letter_queue.pop(i)

            return {
                "success": True,
                "notification": notification.to_dict(),
                "message": "Notification re-queued for delivery",
            }

    return {
        "success": False,
        "error": f"Notification '{notification_id}' not found in dead-letter queue",
    }


@mcp.tool()
async def configure_failure_rates(
    email: float | None = None,
    slack: float | None = None,
    in_app: float | None = None,
) -> dict[str, Any]:
    """Configure mock failure rates for testing.

    This is useful for demonstrating retry and dead-letter patterns.

    Args:
        email: Email failure rate (0.0 to 1.0)
        slack: Slack failure rate (0.0 to 1.0)
        in_app: In-app failure rate (0.0 to 1.0)

    Returns:
        Updated failure rate configuration
    """
    if email is not None:
        if not 0.0 <= email <= 1.0:
            return {"success": False, "error": "email rate must be between 0.0 and 1.0"}
        MockChannelHandler.EMAIL_FAILURE_RATE = email

    if slack is not None:
        if not 0.0 <= slack <= 1.0:
            return {"success": False, "error": "slack rate must be between 0.0 and 1.0"}
        MockChannelHandler.SLACK_FAILURE_RATE = slack

    if in_app is not None:
        if not 0.0 <= in_app <= 1.0:
            return {"success": False, "error": "in_app rate must be between 0.0 and 1.0"}
        MockChannelHandler.INAPP_FAILURE_RATE = in_app

    return {
        "success": True,
        "failure_rates": {
            "email": MockChannelHandler.EMAIL_FAILURE_RATE,
            "slack": MockChannelHandler.SLACK_FAILURE_RATE,
            "in_app": MockChannelHandler.INAPP_FAILURE_RATE,
        },
    }


def main() -> None:
    """Run the notification service MCP server."""
    import sys

    # Default to stdio transport
    transport = "stdio"
    if "--sse" in sys.argv:
        transport = "sse"
    elif "--http" in sys.argv:
        transport = "streamable-http"

    logger.info(f"Starting notification service with {transport} transport")
    mcp.run(transport=transport)


if __name__ == "__main__":
    main()
