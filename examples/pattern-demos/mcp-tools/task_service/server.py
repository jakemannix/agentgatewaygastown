#!/usr/bin/env python3
"""MCP Task Service - SQLite-backed task management server.

Provides CRUD operations for tasks with state transitions and history tracking.
Useful for demonstrating saga patterns (multi-step task workflows).
"""

from __future__ import annotations

import json
import logging
import sqlite3
from contextlib import contextmanager
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Any

from mcp.server.fastmcp import FastMCP

# Configure logging (avoid print() for stdio servers)
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    handlers=[logging.FileHandler("/tmp/task_service.log")],
)
logger = logging.getLogger(__name__)

# Initialize FastMCP server
mcp = FastMCP("task-service")

# Database path (can be overridden via environment variable)
DB_PATH = Path(__file__).parent / "tasks.db"


class TaskStatus(str, Enum):
    """Valid task statuses with allowed transitions."""

    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    BLOCKED = "blocked"
    COMPLETED = "completed"
    CANCELLED = "cancelled"

    @classmethod
    def valid_transitions(cls) -> dict[str, list[str]]:
        """Define valid state transitions."""
        return {
            cls.PENDING: [cls.IN_PROGRESS, cls.CANCELLED],
            cls.IN_PROGRESS: [cls.PENDING, cls.BLOCKED, cls.COMPLETED, cls.CANCELLED],
            cls.BLOCKED: [cls.IN_PROGRESS, cls.CANCELLED],
            cls.COMPLETED: [],  # Terminal state
            cls.CANCELLED: [],  # Terminal state
        }

    def can_transition_to(self, new_status: TaskStatus) -> bool:
        """Check if transition to new_status is valid."""
        valid = self.valid_transitions().get(self, [])
        return new_status in valid


class TaskPriority(str, Enum):
    """Task priority levels."""

    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"


def get_db_path() -> Path:
    """Get the database path."""
    import os

    return Path(os.environ.get("TASK_SERVICE_DB", DB_PATH))


@contextmanager
def get_db():
    """Context manager for database connections."""
    conn = sqlite3.connect(get_db_path())
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    try:
        yield conn
        conn.commit()
    except Exception:
        conn.rollback()
        raise
    finally:
        conn.close()


def init_db():
    """Initialize the database schema."""
    with get_db() as conn:
        conn.executescript(
            """
            -- Tasks table
            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                priority TEXT NOT NULL DEFAULT 'medium',
                assignee TEXT,
                due_date TEXT,
                metadata TEXT,  -- JSON for extensible metadata
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Task history for state transitions
            CREATE TABLE IF NOT EXISTS task_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                old_status TEXT,
                new_status TEXT NOT NULL,
                changed_by TEXT,
                reason TEXT,
                changed_at TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );

            -- Indexes for common queries
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
            CREATE INDEX IF NOT EXISTS idx_tasks_assignee ON tasks(assignee);
            CREATE INDEX IF NOT EXISTS idx_task_history_task_id ON task_history(task_id);
        """
        )
    logger.info("Database initialized at %s", get_db_path())


def now_iso() -> str:
    """Get current UTC time in ISO format."""
    return datetime.now(timezone.utc).isoformat()


def task_to_dict(row: sqlite3.Row) -> dict[str, Any]:
    """Convert a database row to a dictionary."""
    d = dict(row)
    if d.get("metadata"):
        d["metadata"] = json.loads(d["metadata"])
    return d


def record_history(
    conn: sqlite3.Connection,
    task_id: int,
    old_status: str | None,
    new_status: str,
    changed_by: str | None = None,
    reason: str | None = None,
) -> None:
    """Record a state transition in history."""
    conn.execute(
        """
        INSERT INTO task_history (task_id, old_status, new_status, changed_by, reason, changed_at)
        VALUES (?, ?, ?, ?, ?, ?)
        """,
        (task_id, old_status, new_status, changed_by, reason, now_iso()),
    )


# Initialize database on module load
init_db()


# ============================================================================
# MCP Tools - CRUD Operations
# ============================================================================


@mcp.tool()
async def create_task(
    title: str,
    description: str | None = None,
    priority: str = "medium",
    assignee: str | None = None,
    due_date: str | None = None,
    metadata: dict | None = None,
) -> str:
    """Create a new task.

    Args:
        title: Task title (required)
        description: Detailed task description
        priority: Task priority (low, medium, high, critical). Default: medium
        assignee: Person or system assigned to the task
        due_date: Due date in ISO format (YYYY-MM-DD or full ISO datetime)
        metadata: Additional key-value metadata as JSON object

    Returns:
        JSON with the created task details including its ID
    """
    # Validate priority
    try:
        TaskPriority(priority)
    except ValueError:
        return json.dumps(
            {
                "error": f"Invalid priority: {priority}. Valid values: {[p.value for p in TaskPriority]}"
            }
        )

    now = now_iso()
    metadata_json = json.dumps(metadata) if metadata else None

    with get_db() as conn:
        cursor = conn.execute(
            """
            INSERT INTO tasks (title, description, status, priority, assignee, due_date, metadata, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                title,
                description,
                TaskStatus.PENDING.value,
                priority,
                assignee,
                due_date,
                metadata_json,
                now,
                now,
            ),
        )
        task_id = cursor.lastrowid

        # Record initial state in history
        record_history(conn, task_id, None, TaskStatus.PENDING.value, reason="Task created")

        # Fetch the created task
        row = conn.execute("SELECT * FROM tasks WHERE id = ?", (task_id,)).fetchone()

    logger.info("Created task %d: %s", task_id, title)
    return json.dumps({"task": task_to_dict(row)}, indent=2)


@mcp.tool()
async def get_task(task_id: int) -> str:
    """Get a task by ID.

    Args:
        task_id: The unique task identifier

    Returns:
        JSON with task details or error if not found
    """
    with get_db() as conn:
        row = conn.execute("SELECT * FROM tasks WHERE id = ?", (task_id,)).fetchone()

    if not row:
        return json.dumps({"error": f"Task {task_id} not found"})

    return json.dumps({"task": task_to_dict(row)}, indent=2)


@mcp.tool()
async def update_task(
    task_id: int,
    title: str | None = None,
    description: str | None = None,
    priority: str | None = None,
    assignee: str | None = None,
    due_date: str | None = None,
    metadata: dict | None = None,
) -> str:
    """Update task fields (does not change status - use transition_task for that).

    Args:
        task_id: The unique task identifier
        title: New task title
        description: New task description
        priority: New priority (low, medium, high, critical)
        assignee: New assignee
        due_date: New due date in ISO format
        metadata: New metadata (replaces existing metadata)

    Returns:
        JSON with updated task details or error
    """
    # Build update fields dynamically
    updates = []
    values = []

    if title is not None:
        updates.append("title = ?")
        values.append(title)
    if description is not None:
        updates.append("description = ?")
        values.append(description)
    if priority is not None:
        try:
            TaskPriority(priority)
        except ValueError:
            return json.dumps(
                {
                    "error": f"Invalid priority: {priority}. Valid values: {[p.value for p in TaskPriority]}"
                }
            )
        updates.append("priority = ?")
        values.append(priority)
    if assignee is not None:
        updates.append("assignee = ?")
        values.append(assignee)
    if due_date is not None:
        updates.append("due_date = ?")
        values.append(due_date)
    if metadata is not None:
        updates.append("metadata = ?")
        values.append(json.dumps(metadata))

    if not updates:
        return json.dumps({"error": "No fields to update"})

    updates.append("updated_at = ?")
    values.append(now_iso())
    values.append(task_id)

    with get_db() as conn:
        # Check if task exists
        existing = conn.execute("SELECT id FROM tasks WHERE id = ?", (task_id,)).fetchone()
        if not existing:
            return json.dumps({"error": f"Task {task_id} not found"})

        conn.execute(
            f"UPDATE tasks SET {', '.join(updates)} WHERE id = ?",
            values,
        )

        row = conn.execute("SELECT * FROM tasks WHERE id = ?", (task_id,)).fetchone()

    logger.info("Updated task %d", task_id)
    return json.dumps({"task": task_to_dict(row)}, indent=2)


@mcp.tool()
async def delete_task(task_id: int) -> str:
    """Delete a task and its history.

    Args:
        task_id: The unique task identifier

    Returns:
        JSON confirmation or error if not found
    """
    with get_db() as conn:
        existing = conn.execute("SELECT id, title FROM tasks WHERE id = ?", (task_id,)).fetchone()
        if not existing:
            return json.dumps({"error": f"Task {task_id} not found"})

        conn.execute("DELETE FROM tasks WHERE id = ?", (task_id,))

    logger.info("Deleted task %d: %s", task_id, existing["title"])
    return json.dumps({"deleted": True, "task_id": task_id, "title": existing["title"]})


# ============================================================================
# MCP Tools - State Transitions
# ============================================================================


@mcp.tool()
async def transition_task(
    task_id: int,
    new_status: str,
    changed_by: str | None = None,
    reason: str | None = None,
) -> str:
    """Transition a task to a new status with validation and history tracking.

    Valid transitions:
    - pending -> in_progress, cancelled
    - in_progress -> pending, blocked, completed, cancelled
    - blocked -> in_progress, cancelled
    - completed -> (terminal state, no transitions)
    - cancelled -> (terminal state, no transitions)

    Args:
        task_id: The unique task identifier
        new_status: Target status (pending, in_progress, blocked, completed, cancelled)
        changed_by: Who is making the change (for audit trail)
        reason: Reason for the transition (for audit trail)

    Returns:
        JSON with updated task and transition details, or error
    """
    try:
        target_status = TaskStatus(new_status)
    except ValueError:
        return json.dumps(
            {
                "error": f"Invalid status: {new_status}. Valid values: {[s.value for s in TaskStatus]}"
            }
        )

    with get_db() as conn:
        row = conn.execute("SELECT * FROM tasks WHERE id = ?", (task_id,)).fetchone()
        if not row:
            return json.dumps({"error": f"Task {task_id} not found"})

        current_status = TaskStatus(row["status"])

        # Check if transition is valid
        if not current_status.can_transition_to(target_status):
            valid = TaskStatus.valid_transitions().get(current_status, [])
            return json.dumps(
                {
                    "error": f"Invalid transition from {current_status.value} to {new_status}",
                    "valid_transitions": [s.value for s in valid],
                }
            )

        # Update status
        now = now_iso()
        conn.execute(
            "UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?",
            (new_status, now, task_id),
        )

        # Record in history
        record_history(conn, task_id, current_status.value, new_status, changed_by, reason)

        # Fetch updated task
        updated = conn.execute("SELECT * FROM tasks WHERE id = ?", (task_id,)).fetchone()

    logger.info(
        "Transitioned task %d: %s -> %s (by: %s, reason: %s)",
        task_id,
        current_status.value,
        new_status,
        changed_by,
        reason,
    )
    return json.dumps(
        {
            "task": task_to_dict(updated),
            "transition": {
                "from": current_status.value,
                "to": new_status,
                "changed_by": changed_by,
                "reason": reason,
            },
        },
        indent=2,
    )


@mcp.tool()
async def complete_task(task_id: int, changed_by: str | None = None, reason: str | None = None) -> str:
    """Convenience method to mark a task as completed.

    Args:
        task_id: The unique task identifier
        changed_by: Who is completing the task
        reason: Completion notes

    Returns:
        JSON with updated task details or error
    """
    return await transition_task(task_id, TaskStatus.COMPLETED.value, changed_by, reason)


# ============================================================================
# MCP Tools - List and Filter
# ============================================================================


@mcp.tool()
async def list_tasks(
    status: str | None = None,
    priority: str | None = None,
    assignee: str | None = None,
    limit: int = 100,
    offset: int = 0,
) -> str:
    """List tasks with optional filters.

    Args:
        status: Filter by status (pending, in_progress, blocked, completed, cancelled)
        priority: Filter by priority (low, medium, high, critical)
        assignee: Filter by assignee (exact match)
        limit: Maximum number of tasks to return (default: 100)
        offset: Number of tasks to skip for pagination (default: 0)

    Returns:
        JSON with list of tasks and total count
    """
    conditions = []
    values = []

    if status:
        try:
            TaskStatus(status)
        except ValueError:
            return json.dumps(
                {
                    "error": f"Invalid status: {status}. Valid values: {[s.value for s in TaskStatus]}"
                }
            )
        conditions.append("status = ?")
        values.append(status)

    if priority:
        try:
            TaskPriority(priority)
        except ValueError:
            return json.dumps(
                {
                    "error": f"Invalid priority: {priority}. Valid values: {[p.value for p in TaskPriority]}"
                }
            )
        conditions.append("priority = ?")
        values.append(priority)

    if assignee:
        conditions.append("assignee = ?")
        values.append(assignee)

    where_clause = f"WHERE {' AND '.join(conditions)}" if conditions else ""

    with get_db() as conn:
        # Get total count
        count_query = f"SELECT COUNT(*) FROM tasks {where_clause}"
        total = conn.execute(count_query, values).fetchone()[0]

        # Get tasks
        query = f"""
            SELECT * FROM tasks {where_clause}
            ORDER BY
                CASE priority
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'low' THEN 4
                END,
                created_at DESC
            LIMIT ? OFFSET ?
        """
        rows = conn.execute(query, values + [limit, offset]).fetchall()

    tasks = [task_to_dict(row) for row in rows]
    return json.dumps(
        {
            "tasks": tasks,
            "total": total,
            "limit": limit,
            "offset": offset,
        },
        indent=2,
    )


@mcp.tool()
async def get_task_history(task_id: int) -> str:
    """Get the state transition history for a task.

    Args:
        task_id: The unique task identifier

    Returns:
        JSON with chronological list of state transitions
    """
    with get_db() as conn:
        # Verify task exists
        task = conn.execute("SELECT id, title FROM tasks WHERE id = ?", (task_id,)).fetchone()
        if not task:
            return json.dumps({"error": f"Task {task_id} not found"})

        rows = conn.execute(
            """
            SELECT * FROM task_history
            WHERE task_id = ?
            ORDER BY changed_at ASC
            """,
            (task_id,),
        ).fetchall()

    history = [dict(row) for row in rows]
    return json.dumps(
        {
            "task_id": task_id,
            "task_title": task["title"],
            "history": history,
        },
        indent=2,
    )


@mcp.tool()
async def get_task_stats() -> str:
    """Get aggregate statistics about tasks.

    Returns:
        JSON with task counts by status and priority
    """
    with get_db() as conn:
        # Count by status
        status_counts = {}
        for row in conn.execute(
            "SELECT status, COUNT(*) as count FROM tasks GROUP BY status"
        ).fetchall():
            status_counts[row["status"]] = row["count"]

        # Count by priority
        priority_counts = {}
        for row in conn.execute(
            "SELECT priority, COUNT(*) as count FROM tasks GROUP BY priority"
        ).fetchall():
            priority_counts[row["priority"]] = row["count"]

        # Total and overdue
        total = conn.execute("SELECT COUNT(*) FROM tasks").fetchone()[0]
        now = now_iso()
        overdue = conn.execute(
            """
            SELECT COUNT(*) FROM tasks
            WHERE due_date < ? AND status NOT IN ('completed', 'cancelled')
            """,
            (now,),
        ).fetchone()[0]

    return json.dumps(
        {
            "total": total,
            "by_status": status_counts,
            "by_priority": priority_counts,
            "overdue": overdue,
        },
        indent=2,
    )


def main():
    """Run the MCP server."""
    logger.info("Starting Task Service MCP server")
    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
