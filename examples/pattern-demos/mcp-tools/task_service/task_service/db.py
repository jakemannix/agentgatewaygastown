"""SQLite database operations for the task service."""

import json
import sqlite3
import uuid
from contextlib import contextmanager
from datetime import datetime
from pathlib import Path
from typing import Optional

from .models import (
    StateTransition,
    Task,
    TaskCreate,
    TaskListFilter,
    TaskPriority,
    TaskStatus,
    TaskUpdate,
)


class TaskDatabase:
    """SQLite-backed task database with history tracking."""

    def __init__(self, db_path: str = ":memory:"):
        """Initialize the database.

        Args:
            db_path: Path to SQLite database file, or ':memory:' for in-memory DB.
        """
        self.db_path = db_path
        self._is_memory = db_path == ":memory:"
        self._conn: Optional[sqlite3.Connection] = None
        self._init_db()

    def _create_connection(self) -> sqlite3.Connection:
        """Create a new database connection."""
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        # Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON")
        return conn

    @contextmanager
    def _get_conn(self):
        """Get a database connection with row factory.

        For in-memory databases, reuses a single connection.
        For file-based databases, creates a new connection each time.
        """
        if self._is_memory:
            # For in-memory DB, reuse the same connection
            if self._conn is None:
                self._conn = self._create_connection()
            try:
                yield self._conn
                self._conn.commit()
            except Exception:
                self._conn.rollback()
                raise
        else:
            # For file-based DB, create new connection each time
            conn = self._create_connection()
            try:
                yield conn
                conn.commit()
            except Exception:
                conn.rollback()
                raise
            finally:
                conn.close()

    def _init_db(self):
        """Initialize database schema."""
        with self._get_conn() as conn:
            cursor = conn.cursor()

            # Tasks table
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS tasks (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    description TEXT,
                    status TEXT NOT NULL DEFAULT 'pending',
                    priority TEXT NOT NULL DEFAULT 'medium',
                    assignee TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    completed_at TEXT,
                    metadata TEXT
                )
            """)

            # State transitions history table
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS state_transitions (
                    id TEXT PRIMARY KEY,
                    task_id TEXT NOT NULL,
                    from_status TEXT,
                    to_status TEXT NOT NULL,
                    changed_by TEXT,
                    changed_at TEXT NOT NULL,
                    reason TEXT,
                    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
                )
            """)

            # Indexes for efficient querying
            cursor.execute("""
                CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)
            """)
            cursor.execute("""
                CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority)
            """)
            cursor.execute("""
                CREATE INDEX IF NOT EXISTS idx_tasks_assignee ON tasks(assignee)
            """)
            cursor.execute("""
                CREATE INDEX IF NOT EXISTS idx_transitions_task_id
                ON state_transitions(task_id)
            """)

    def _generate_id(self) -> str:
        """Generate a unique ID."""
        return str(uuid.uuid4())[:8]

    def _now(self) -> str:
        """Get current timestamp as ISO string."""
        return datetime.utcnow().isoformat()

    def _row_to_task(self, row: sqlite3.Row) -> Task:
        """Convert a database row to a Task model."""
        return Task(
            id=row["id"],
            title=row["title"],
            description=row["description"],
            status=TaskStatus(row["status"]),
            priority=TaskPriority(row["priority"]),
            assignee=row["assignee"],
            created_at=datetime.fromisoformat(row["created_at"]),
            updated_at=datetime.fromisoformat(row["updated_at"]),
            completed_at=(
                datetime.fromisoformat(row["completed_at"])
                if row["completed_at"]
                else None
            ),
            metadata=json.loads(row["metadata"]) if row["metadata"] else None,
        )

    def _row_to_transition(self, row: sqlite3.Row) -> StateTransition:
        """Convert a database row to a StateTransition model."""
        return StateTransition(
            id=row["id"],
            task_id=row["task_id"],
            from_status=(
                TaskStatus(row["from_status"]) if row["from_status"] else None
            ),
            to_status=TaskStatus(row["to_status"]),
            changed_by=row["changed_by"],
            changed_at=datetime.fromisoformat(row["changed_at"]),
            reason=row["reason"],
        )

    def _record_transition(
        self,
        cursor: sqlite3.Cursor,
        task_id: str,
        from_status: Optional[TaskStatus],
        to_status: TaskStatus,
        changed_by: Optional[str] = None,
        reason: Optional[str] = None,
    ) -> StateTransition:
        """Record a state transition in history."""
        transition_id = self._generate_id()
        now = self._now()

        cursor.execute(
            """
            INSERT INTO state_transitions
            (id, task_id, from_status, to_status, changed_by, changed_at, reason)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            """,
            (
                transition_id,
                task_id,
                from_status.value if from_status else None,
                to_status.value,
                changed_by,
                now,
                reason,
            ),
        )

        return StateTransition(
            id=transition_id,
            task_id=task_id,
            from_status=from_status,
            to_status=to_status,
            changed_by=changed_by,
            changed_at=datetime.fromisoformat(now),
            reason=reason,
        )

    def create_task(
        self, data: TaskCreate, created_by: Optional[str] = None
    ) -> Task:
        """Create a new task.

        Args:
            data: Task creation data.
            created_by: User creating the task.

        Returns:
            The created task.
        """
        task_id = self._generate_id()
        now = self._now()

        with self._get_conn() as conn:
            cursor = conn.cursor()

            cursor.execute(
                """
                INSERT INTO tasks
                (id, title, description, status, priority, assignee,
                 created_at, updated_at, metadata)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    task_id,
                    data.title,
                    data.description,
                    TaskStatus.PENDING.value,
                    data.priority.value,
                    data.assignee,
                    now,
                    now,
                    json.dumps(data.metadata) if data.metadata else None,
                ),
            )

            # Record initial state transition
            self._record_transition(
                cursor,
                task_id,
                from_status=None,
                to_status=TaskStatus.PENDING,
                changed_by=created_by,
                reason="Task created",
            )

            cursor.execute("SELECT * FROM tasks WHERE id = ?", (task_id,))
            return self._row_to_task(cursor.fetchone())

    def get_task(self, task_id: str) -> Optional[Task]:
        """Get a task by ID.

        Args:
            task_id: The task ID.

        Returns:
            The task if found, None otherwise.
        """
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT * FROM tasks WHERE id = ?", (task_id,))
            row = cursor.fetchone()
            return self._row_to_task(row) if row else None

    def update_task(
        self,
        task_id: str,
        data: TaskUpdate,
        updated_by: Optional[str] = None,
    ) -> Optional[Task]:
        """Update a task.

        Args:
            task_id: The task ID.
            data: Update data.
            updated_by: User making the update.

        Returns:
            The updated task if found, None otherwise.
        """
        with self._get_conn() as conn:
            cursor = conn.cursor()

            # Check task exists
            cursor.execute("SELECT * FROM tasks WHERE id = ?", (task_id,))
            row = cursor.fetchone()
            if not row:
                return None

            # Build update query dynamically
            updates = []
            params = []

            if data.title is not None:
                updates.append("title = ?")
                params.append(data.title)
            if data.description is not None:
                updates.append("description = ?")
                params.append(data.description)
            if data.priority is not None:
                updates.append("priority = ?")
                params.append(data.priority.value)
            if data.assignee is not None:
                updates.append("assignee = ?")
                params.append(data.assignee)
            if data.metadata is not None:
                updates.append("metadata = ?")
                params.append(json.dumps(data.metadata))

            if not updates:
                return self._row_to_task(row)

            updates.append("updated_at = ?")
            params.append(self._now())
            params.append(task_id)

            cursor.execute(
                f"UPDATE tasks SET {', '.join(updates)} WHERE id = ?",
                params,
            )

            cursor.execute("SELECT * FROM tasks WHERE id = ?", (task_id,))
            return self._row_to_task(cursor.fetchone())

    def complete_task(
        self,
        task_id: str,
        completed_by: Optional[str] = None,
        reason: Optional[str] = None,
    ) -> Optional[Task]:
        """Mark a task as completed.

        Args:
            task_id: The task ID.
            completed_by: User completing the task.
            reason: Reason for completion.

        Returns:
            The completed task if found, None otherwise.
        """
        with self._get_conn() as conn:
            cursor = conn.cursor()

            cursor.execute("SELECT * FROM tasks WHERE id = ?", (task_id,))
            row = cursor.fetchone()
            if not row:
                return None

            old_status = TaskStatus(row["status"])
            now = self._now()

            cursor.execute(
                """
                UPDATE tasks
                SET status = ?, updated_at = ?, completed_at = ?
                WHERE id = ?
                """,
                (TaskStatus.COMPLETED.value, now, now, task_id),
            )

            self._record_transition(
                cursor,
                task_id,
                from_status=old_status,
                to_status=TaskStatus.COMPLETED,
                changed_by=completed_by,
                reason=reason or "Task completed",
            )

            cursor.execute("SELECT * FROM tasks WHERE id = ?", (task_id,))
            return self._row_to_task(cursor.fetchone())

    def delete_task(self, task_id: str) -> bool:
        """Delete a task.

        Args:
            task_id: The task ID.

        Returns:
            True if deleted, False if not found.
        """
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM tasks WHERE id = ?", (task_id,))
            return cursor.rowcount > 0

    def list_tasks(self, filters: Optional[TaskListFilter] = None) -> list[Task]:
        """List tasks with optional filters.

        Args:
            filters: Optional filters to apply.

        Returns:
            List of matching tasks.
        """
        filters = filters or TaskListFilter()

        with self._get_conn() as conn:
            cursor = conn.cursor()

            query = "SELECT * FROM tasks WHERE 1=1"
            params = []

            if filters.status:
                query += " AND status = ?"
                params.append(filters.status.value)
            if filters.priority:
                query += " AND priority = ?"
                params.append(filters.priority.value)
            if filters.assignee:
                query += " AND assignee = ?"
                params.append(filters.assignee)

            query += " ORDER BY created_at DESC LIMIT ? OFFSET ?"
            params.extend([filters.limit, filters.offset])

            cursor.execute(query, params)
            return [self._row_to_task(row) for row in cursor.fetchall()]

    def transition_status(
        self,
        task_id: str,
        new_status: TaskStatus,
        changed_by: Optional[str] = None,
        reason: Optional[str] = None,
    ) -> Optional[Task]:
        """Transition a task to a new status.

        Args:
            task_id: The task ID.
            new_status: The new status.
            changed_by: User making the change.
            reason: Reason for the transition.

        Returns:
            The updated task if found, None otherwise.
        """
        with self._get_conn() as conn:
            cursor = conn.cursor()

            cursor.execute("SELECT * FROM tasks WHERE id = ?", (task_id,))
            row = cursor.fetchone()
            if not row:
                return None

            old_status = TaskStatus(row["status"])
            now = self._now()

            updates = {"status": new_status.value, "updated_at": now}
            if new_status == TaskStatus.COMPLETED:
                updates["completed_at"] = now

            set_clause = ", ".join(f"{k} = ?" for k in updates.keys())
            cursor.execute(
                f"UPDATE tasks SET {set_clause} WHERE id = ?",
                [*updates.values(), task_id],
            )

            self._record_transition(
                cursor,
                task_id,
                from_status=old_status,
                to_status=new_status,
                changed_by=changed_by,
                reason=reason,
            )

            cursor.execute("SELECT * FROM tasks WHERE id = ?", (task_id,))
            return self._row_to_task(cursor.fetchone())

    def get_task_history(self, task_id: str) -> list[StateTransition]:
        """Get the state transition history for a task.

        Args:
            task_id: The task ID.

        Returns:
            List of state transitions in chronological order.
        """
        with self._get_conn() as conn:
            cursor = conn.cursor()
            cursor.execute(
                """
                SELECT * FROM state_transitions
                WHERE task_id = ?
                ORDER BY changed_at ASC
                """,
                (task_id,),
            )
            return [self._row_to_transition(row) for row in cursor.fetchall()]
