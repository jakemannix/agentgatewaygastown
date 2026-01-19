#!/usr/bin/env python3
"""MCP Notification Service - Async notification patterns with SQLite persistence.

This MCP server provides notification capabilities with mock channel delivery,
demonstrating async patterns including queuing, retry logic, and dead-letter handling.

Data is persisted to SQLite for durability across restarts.

Tools:
    - send_notification: Queue a notification for delivery
    - get_notifications: Retrieve notifications for a user
    - mark_read: Mark notifications as read
    - get_dead_letter_queue: View failed notifications
    - retry_dead_letter: Retry a failed notification
    - configure_failure_rates: Adjust mock failure rates for testing

Channels (mock implementations):
    - email: Mock email delivery with configurable failure rates
    - slack: Mock Slack webhook delivery
    - in_app: In-app notification storage
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import random
import sqlite3
import uuid
from contextlib import contextmanager
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any

from mcp.server.fastmcp import FastMCP

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)
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


class NotificationDatabase:
    """SQLite-backed notification storage."""

    def __init__(self, db_path: str = ":memory:"):
        """Initialize the database.

        Args:
            db_path: Path to SQLite database file, or ':memory:' for in-memory.
        """
        self.db_path = db_path
        self._init_db()
        logger.info(f"Notification database initialized at {db_path}")

    @contextmanager
    def _get_conn(self):
        """Get a database connection."""
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        try:
            yield conn
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            conn.close()

    def _init_db(self) -> None:
        """Initialize the database schema."""
        with self._get_conn() as conn:
            cursor = conn.cursor()

            # Notifications table
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS notifications (
                    id TEXT PRIMARY KEY,
                    user_id TEXT NOT NULL,
                    channel TEXT NOT NULL,
                    subject TEXT NOT NULL,
                    body TEXT NOT NULL,
                    priority TEXT NOT NULL DEFAULT 'normal',
                    status TEXT NOT NULL DEFAULT 'pending',
                    created_at TEXT NOT NULL,
                    delivered_at TEXT,
                    read_at TEXT,
                    retry_count INTEGER DEFAULT 0,
                    max_retries INTEGER DEFAULT 3,
                    last_error TEXT,
                    metadata TEXT DEFAULT '{}'
                )
            """)

            # Dead letter queue table
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS dead_letter_queue (
                    id TEXT PRIMARY KEY,
                    notification_id TEXT NOT NULL,
                    reason TEXT NOT NULL,
                    failed_at TEXT NOT NULL,
                    FOREIGN KEY (notification_id) REFERENCES notifications(id)
                )
            """)

            # Indexes
            cursor.execute("""
                CREATE INDEX IF NOT EXISTS idx_notifications_user_id
                ON notifications(user_id)
            """)
            cursor.execute("""
                CREATE INDEX IF NOT EXISTS idx_notifications_status
                ON notifications(status)
            """)

    def _row_to_notification(self, row: sqlite3.Row) -> Notification:
        """Convert a database row to a Notification."""
        return Notification(
            id=row["id"],
            user_id=row["user_id"],
            channel=Channel(row["channel"]),
            subject=row["subject"],
            body=row["body"],
            priority=Priority(row["priority"]),
            status=NotificationStatus(row["status"]),
            created_at=datetime.fromisoformat(row["created_at"]),
            delivered_at=datetime.fromisoformat(row["delivered_at"]) if row["delivered_at"] else None,
            read_at=datetime.fromisoformat(row["read_at"]) if row["read_at"] else None,
            retry_count=row["retry_count"],
            max_retries=row["max_retries"],
            last_error=row["last_error"],
            metadata=json.loads(row["metadata"]) if row["metadata"] else {},
        )

    def create_notification(self, notification: Notification) -> Notification:
        """Create a new notification."""
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute(
                """
                INSERT INTO notifications
                (id, user_id, channel, subject, body, priority, status,
                 created_at, retry_count, max_retries, metadata)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    notification.id,
                    notification.user_id,
                    notification.channel.value,
                    notification.subject,
                    notification.body,
                    notification.priority.value,
                    notification.status.value,
                    notification.created_at.isoformat(),
                    notification.retry_count,
                    notification.max_retries,
                    json.dumps(notification.metadata),
                ),
            )
        return notification

    def get_notification(self, notification_id: str) -> Notification | None:
        """Get a notification by ID."""
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT * FROM notifications WHERE id = ?", (notification_id,))
            row = cursor.fetchone()
            return self._row_to_notification(row) if row else None

    def update_notification(self, notification: Notification) -> None:
        """Update a notification."""
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute(
                """
                UPDATE notifications
                SET status = ?, delivered_at = ?, read_at = ?,
                    retry_count = ?, last_error = ?
                WHERE id = ?
                """,
                (
                    notification.status.value,
                    notification.delivered_at.isoformat() if notification.delivered_at else None,
                    notification.read_at.isoformat() if notification.read_at else None,
                    notification.retry_count,
                    notification.last_error,
                    notification.id,
                ),
            )

    def get_notifications_for_user(
        self,
        user_id: str,
        channel: Channel | None = None,
        status: NotificationStatus | None = None,
        limit: int = 50,
    ) -> list[Notification]:
        """Get notifications for a user with optional filters."""
        with self._get_conn() as conn:
            cursor = conn.cursor()

            query = "SELECT * FROM notifications WHERE user_id = ?"
            params: list[Any] = [user_id]

            if channel:
                query += " AND channel = ?"
                params.append(channel.value)
            if status:
                query += " AND status = ?"
                params.append(status.value)

            query += " ORDER BY created_at DESC LIMIT ?"
            params.append(limit)

            cursor.execute(query, params)
            return [self._row_to_notification(row) for row in cursor.fetchall()]

    def get_pending_notifications(self) -> list[Notification]:
        """Get all pending/queued notifications for processing."""
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute(
                """
                SELECT * FROM notifications
                WHERE status IN ('pending', 'queued')
                ORDER BY priority DESC, created_at ASC
                """
            )
            return [self._row_to_notification(row) for row in cursor.fetchall()]

    def add_to_dead_letter(self, notification_id: str, reason: str) -> None:
        """Add a notification to the dead-letter queue."""
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute(
                """
                INSERT INTO dead_letter_queue (id, notification_id, reason, failed_at)
                VALUES (?, ?, ?, ?)
                """,
                (
                    str(uuid.uuid4()),
                    notification_id,
                    reason,
                    datetime.now(timezone.utc).isoformat(),
                ),
            )

    def get_dead_letter_queue(self, limit: int = 50) -> list[dict[str, Any]]:
        """Get entries from the dead-letter queue."""
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute(
                """
                SELECT dlq.*, n.*
                FROM dead_letter_queue dlq
                JOIN notifications n ON dlq.notification_id = n.id
                ORDER BY dlq.failed_at DESC
                LIMIT ?
                """,
                (limit,),
            )
            results = []
            for row in cursor.fetchall():
                notification = self._row_to_notification(row)
                results.append({
                    "dlq_id": row["id"],
                    "notification": notification.to_dict(),
                    "reason": row["reason"],
                    "failed_at": row["failed_at"],
                })
            return results

    def remove_from_dead_letter(self, notification_id: str) -> bool:
        """Remove a notification from the dead-letter queue."""
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute(
                "DELETE FROM dead_letter_queue WHERE notification_id = ?",
                (notification_id,),
            )
            return cursor.rowcount > 0

    def count_dead_letter(self) -> int:
        """Count entries in the dead-letter queue."""
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT COUNT(*) FROM dead_letter_queue")
            return cursor.fetchone()[0]


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
    """Background processor for notification delivery."""

    def __init__(self, db: NotificationDatabase) -> None:
        self.db = db
        self._task: asyncio.Task | None = None
        self._running = False

    async def start(self) -> None:
        """Start the background processor."""
        if self._task is None:
            self._running = True
            self._task = asyncio.create_task(self._process_loop())
            logger.info("Notification processor started")

    async def stop(self) -> None:
        """Stop the background processor."""
        self._running = False
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
        while self._running:
            try:
                # Get pending notifications from database
                pending = self.db.get_pending_notifications()
                for notification in pending:
                    if not self._running:
                        break
                    await self._process_notification(notification)

                # Sleep before next check
                await asyncio.sleep(1.0)
            except asyncio.CancelledError:
                raise
            except Exception as e:
                logger.error(f"Error in processor loop: {e}")
                await asyncio.sleep(5.0)  # Back off on error

    async def _process_notification(self, notification: Notification) -> None:
        """Process a single notification."""
        # Mark as queued
        notification.status = NotificationStatus.QUEUED
        self.db.update_notification(notification)

        # Attempt delivery
        success, error = await MockChannelHandler.deliver(notification)

        if success:
            notification.status = NotificationStatus.DELIVERED
            notification.delivered_at = datetime.now(timezone.utc)
            self.db.update_notification(notification)
            logger.info(f"Notification {notification.id} delivered successfully")
        else:
            notification.retry_count += 1
            notification.last_error = error

            if notification.retry_count < notification.max_retries:
                # Mark back as pending for retry
                notification.status = NotificationStatus.PENDING
                self.db.update_notification(notification)
                logger.info(
                    f"Notification {notification.id} failed, retry "
                    f"{notification.retry_count}/{notification.max_retries}"
                )
            else:
                # Move to dead-letter queue
                notification.status = NotificationStatus.FAILED
                self.db.update_notification(notification)
                self.db.add_to_dead_letter(
                    notification.id,
                    f"Max retries ({notification.max_retries}) exceeded. Last error: {error}",
                )
                logger.warning(
                    f"Notification {notification.id} moved to dead-letter queue"
                )


# =============================================================================
# MCP Server Setup
# =============================================================================

# Initialize FastMCP server
mcp = FastMCP(
    name="notification-service",
    instructions="""Notification Service MCP Server

This server provides notification capabilities with support for multiple channels
(email, slack, in-app) and async delivery patterns. Notifications are persisted
to SQLite and processed asynchronously with retry logic and dead-letter handling.

Available tools:
- send_notification: Send a notification to a user
- get_notifications: Get notifications for a user
- mark_read: Mark a notification as read
- get_dead_letter_queue: View failed notifications
- retry_dead_letter: Retry a failed notification
- configure_failure_rates: Adjust mock failure rates for testing""",
)

# Database and processor (initialized on startup)
DB_PATH = os.environ.get("NOTIFICATION_SERVICE_DB", ":memory:")
_db: NotificationDatabase | None = None
_processor: NotificationProcessor | None = None


def _get_db() -> NotificationDatabase:
    """Get or initialize the database."""
    global _db
    if _db is None:
        _db = NotificationDatabase(DB_PATH)
    return _db


def _get_processor() -> NotificationProcessor:
    """Get or initialize the processor."""
    global _processor
    if _processor is None:
        _processor = NotificationProcessor(_get_db())
    return _processor


# =============================================================================
# MCP Tools
# =============================================================================


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

    # Save to database
    db = _get_db()
    db.create_notification(notification)

    # Ensure processor is running
    processor = _get_processor()
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

    db = _get_db()
    notifications = db.get_notifications_for_user(user_id, channel=ch, status=st, limit=limit)

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
    db = _get_db()
    notification = db.get_notification(notification_id)

    if not notification:
        return {
            "success": False,
            "error": f"Notification '{notification_id}' not found",
        }

    notification.status = NotificationStatus.READ
    notification.read_at = datetime.now(timezone.utc)
    db.update_notification(notification)

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
    db = _get_db()
    entries = db.get_dead_letter_queue(limit=limit)
    total = db.count_dead_letter()

    return {
        "success": True,
        "count": len(entries),
        "total_in_queue": total,
        "entries": entries,
    }


@mcp.tool()
async def retry_dead_letter(notification_id: str) -> dict[str, Any]:
    """Retry a notification from the dead-letter queue.

    Args:
        notification_id: The notification ID to retry

    Returns:
        Status of the retry operation
    """
    db = _get_db()

    # Get the notification
    notification = db.get_notification(notification_id)
    if not notification:
        return {
            "success": False,
            "error": f"Notification '{notification_id}' not found",
        }

    # Check if it's in the dead-letter queue
    if notification.status != NotificationStatus.FAILED:
        return {
            "success": False,
            "error": f"Notification '{notification_id}' is not in failed state",
        }

    # Remove from dead-letter queue and reset for retry
    db.remove_from_dead_letter(notification_id)
    notification.status = NotificationStatus.PENDING
    notification.retry_count = 0
    notification.last_error = None
    db.update_notification(notification)

    return {
        "success": True,
        "notification": notification.to_dict(),
        "message": "Notification re-queued for delivery",
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


@mcp.resource("schema://notifications")
def get_schema() -> str:
    """Get the notification service database schema."""
    return """
-- Notifications table: stores all notification messages
CREATE TABLE notifications (
    id TEXT PRIMARY KEY,           -- Unique notification identifier
    user_id TEXT NOT NULL,         -- Target user
    channel TEXT NOT NULL,         -- Delivery channel (email, slack, in_app)
    subject TEXT NOT NULL,         -- Notification subject/title
    body TEXT NOT NULL,            -- Notification body content
    priority TEXT NOT NULL,        -- Priority level (low, normal, high, urgent)
    status TEXT NOT NULL,          -- Status (pending, queued, delivered, failed, read)
    created_at TEXT NOT NULL,      -- ISO timestamp of creation
    delivered_at TEXT,             -- ISO timestamp of delivery
    read_at TEXT,                  -- ISO timestamp when read
    retry_count INTEGER DEFAULT 0, -- Number of delivery attempts
    max_retries INTEGER DEFAULT 3, -- Maximum retry attempts
    last_error TEXT,               -- Last error message
    metadata TEXT DEFAULT '{}'     -- JSON metadata
);

-- Dead letter queue: failed notifications
CREATE TABLE dead_letter_queue (
    id TEXT PRIMARY KEY,           -- DLQ entry ID
    notification_id TEXT NOT NULL, -- Reference to notification
    reason TEXT NOT NULL,          -- Failure reason
    failed_at TEXT NOT NULL,       -- ISO timestamp of final failure
    FOREIGN KEY (notification_id) REFERENCES notifications(id)
);

-- Indexes for efficient querying
CREATE INDEX idx_notifications_user_id ON notifications(user_id);
CREATE INDEX idx_notifications_status ON notifications(status);
"""


def main() -> None:
    """Run the notification service MCP server."""
    import argparse

    parser = argparse.ArgumentParser(description="Notification Service MCP Server")
    parser.add_argument(
        "--transport",
        choices=["stdio", "sse", "streamable-http"],
        default="stdio",
        help="Transport type (default: stdio)",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=8004,
        help="Port for HTTP transport (default: 8004)",
    )
    parser.add_argument(
        "--db",
        default=":memory:",
        help="SQLite database path (default: in-memory)",
    )
    # Legacy flag support
    parser.add_argument("--http", action="store_true", help="Use streamable-http transport")
    parser.add_argument("--sse", action="store_true", help="Use SSE transport")

    args = parser.parse_args()

    # Set database path
    global DB_PATH
    DB_PATH = args.db
    os.environ["NOTIFICATION_SERVICE_DB"] = args.db

    # Handle legacy flags
    transport = args.transport
    if args.http:
        transport = "streamable-http"
    elif args.sse:
        transport = "sse"

    logger.info(f"Starting notification service with {transport} transport")
    logger.info(f"Database: {args.db}")

    if transport == "streamable-http":
        logger.info(f"Listening on port {args.port}")
        mcp.run(transport="streamable-http", port=args.port)
    else:
        mcp.run(transport=transport)


if __name__ == "__main__":
    main()
