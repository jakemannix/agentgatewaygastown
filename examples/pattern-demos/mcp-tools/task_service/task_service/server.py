"""MCP Task Service - SQLite-backed task/todo management.

This server demonstrates:
- CRUD operations for tasks
- State transitions with history tracking
- Filtering and listing capabilities
- Patterns useful for saga workflows
"""

import os
from typing import Optional

from mcp.server.fastmcp import FastMCP

from .db import TaskDatabase
from .models import (
    StateTransition,
    Task,
    TaskCreate,
    TaskListFilter,
    TaskPriority,
    TaskStatus,
    TaskUpdate,
)

# Initialize FastMCP server
mcp = FastMCP(
    "Task Service",
    json_response=True,
)

# Database path from environment or default to in-memory
DB_PATH = os.environ.get("TASK_SERVICE_DB", ":memory:")
db = TaskDatabase(DB_PATH)


@mcp.tool()
def create_task(
    title: str,
    description: Optional[str] = None,
    priority: str = "medium",
    assignee: Optional[str] = None,
    metadata: Optional[dict] = None,
) -> dict:
    """Create a new task.

    Args:
        title: The task title (required).
        description: Optional task description.
        priority: Task priority - one of: low, medium, high, critical.
        assignee: Optional user to assign the task to.
        metadata: Optional additional metadata as key-value pairs.

    Returns:
        The created task with all fields populated.
    """
    task_data = TaskCreate(
        title=title,
        description=description,
        priority=TaskPriority(priority),
        assignee=assignee,
        metadata=metadata,
    )
    task = db.create_task(task_data)
    return task.model_dump(mode="json")


@mcp.tool()
def get_task(task_id: str) -> dict:
    """Get a task by its ID.

    Args:
        task_id: The unique task identifier.

    Returns:
        The task details if found, or an error message.
    """
    task = db.get_task(task_id)
    if not task:
        return {"error": f"Task {task_id} not found"}
    return task.model_dump(mode="json")


@mcp.tool()
def update_task(
    task_id: str,
    title: Optional[str] = None,
    description: Optional[str] = None,
    priority: Optional[str] = None,
    assignee: Optional[str] = None,
    metadata: Optional[dict] = None,
) -> dict:
    """Update an existing task's fields.

    Only provided fields will be updated; others remain unchanged.

    Args:
        task_id: The task ID to update.
        title: New title (optional).
        description: New description (optional).
        priority: New priority - one of: low, medium, high, critical (optional).
        assignee: New assignee (optional).
        metadata: New metadata (optional).

    Returns:
        The updated task, or an error message if not found.
    """
    update_data = TaskUpdate(
        title=title,
        description=description,
        priority=TaskPriority(priority) if priority else None,
        assignee=assignee,
        metadata=metadata,
    )
    task = db.update_task(task_id, update_data)
    if not task:
        return {"error": f"Task {task_id} not found"}
    return task.model_dump(mode="json")


@mcp.tool()
def complete_task(
    task_id: str,
    reason: Optional[str] = None,
) -> dict:
    """Mark a task as completed.

    This records the completion in the task's state transition history.

    Args:
        task_id: The task ID to complete.
        reason: Optional reason for completion.

    Returns:
        The completed task, or an error message if not found.
    """
    task = db.complete_task(task_id, reason=reason)
    if not task:
        return {"error": f"Task {task_id} not found"}
    return task.model_dump(mode="json")


@mcp.tool()
def delete_task(task_id: str) -> dict:
    """Delete a task permanently.

    This also removes the task's state transition history.

    Args:
        task_id: The task ID to delete.

    Returns:
        Success status and message.
    """
    deleted = db.delete_task(task_id)
    if not deleted:
        return {"error": f"Task {task_id} not found", "deleted": False}
    return {"deleted": True, "task_id": task_id}


@mcp.tool()
def list_tasks(
    status: Optional[str] = None,
    priority: Optional[str] = None,
    assignee: Optional[str] = None,
    limit: int = 100,
    offset: int = 0,
) -> dict:
    """List tasks with optional filters.

    Args:
        status: Filter by status - one of: pending, in_progress, completed, cancelled.
        priority: Filter by priority - one of: low, medium, high, critical.
        assignee: Filter by assigned user.
        limit: Maximum number of results (default: 100).
        offset: Pagination offset (default: 0).

    Returns:
        List of matching tasks and count.
    """
    filters = TaskListFilter(
        status=TaskStatus(status) if status else None,
        priority=TaskPriority(priority) if priority else None,
        assignee=assignee,
        limit=limit,
        offset=offset,
    )
    tasks = db.list_tasks(filters)
    return {
        "tasks": [t.model_dump(mode="json") for t in tasks],
        "count": len(tasks),
        "filters": {
            "status": status,
            "priority": priority,
            "assignee": assignee,
            "limit": limit,
            "offset": offset,
        },
    }


@mcp.tool()
def transition_task_status(
    task_id: str,
    new_status: str,
    reason: Optional[str] = None,
) -> dict:
    """Transition a task to a new status.

    Valid status values: pending, in_progress, completed, cancelled.

    This is useful for saga patterns where you need explicit state transitions.
    Each transition is recorded in the task's history.

    Args:
        task_id: The task ID.
        new_status: The new status value.
        reason: Optional reason for the transition.

    Returns:
        The updated task, or an error message if not found.
    """
    try:
        status = TaskStatus(new_status)
    except ValueError:
        return {
            "error": f"Invalid status: {new_status}. "
            f"Valid values: {[s.value for s in TaskStatus]}"
        }

    task = db.transition_status(task_id, status, reason=reason)
    if not task:
        return {"error": f"Task {task_id} not found"}
    return task.model_dump(mode="json")


@mcp.tool()
def get_task_history(task_id: str) -> dict:
    """Get the complete state transition history for a task.

    This shows all status changes with timestamps and reasons,
    useful for auditing and saga pattern compensation.

    Args:
        task_id: The task ID.

    Returns:
        List of state transitions in chronological order.
    """
    # First verify task exists
    task = db.get_task(task_id)
    if not task:
        return {"error": f"Task {task_id} not found"}

    history = db.get_task_history(task_id)
    return {
        "task_id": task_id,
        "current_status": task.status.value,
        "transitions": [t.model_dump(mode="json") for t in history],
        "transition_count": len(history),
    }


@mcp.resource("schema://tasks")
def get_schema() -> str:
    """Get the task service database schema.

    Returns the SQLite schema for the tasks and state_transitions tables,
    useful for understanding the data model.
    """
    return """
-- Tasks table: stores all task/todo items
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,           -- Unique task identifier
    title TEXT NOT NULL,           -- Task title
    description TEXT,              -- Optional description
    status TEXT NOT NULL,          -- pending, in_progress, completed, cancelled
    priority TEXT NOT NULL,        -- low, medium, high, critical
    assignee TEXT,                 -- Assigned user (optional)
    created_at TEXT NOT NULL,      -- ISO timestamp
    updated_at TEXT NOT NULL,      -- ISO timestamp
    completed_at TEXT,             -- ISO timestamp (when completed)
    metadata TEXT                  -- JSON metadata
);

-- State transitions: audit log for status changes
CREATE TABLE state_transitions (
    id TEXT PRIMARY KEY,           -- Transition record ID
    task_id TEXT NOT NULL,         -- Associated task
    from_status TEXT,              -- Previous status (null for creation)
    to_status TEXT NOT NULL,       -- New status
    changed_by TEXT,               -- User who made the change
    changed_at TEXT NOT NULL,      -- ISO timestamp
    reason TEXT,                   -- Optional reason for change
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Indexes for efficient filtering
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_priority ON tasks(priority);
CREATE INDEX idx_tasks_assignee ON tasks(assignee);
CREATE INDEX idx_transitions_task_id ON state_transitions(task_id);
"""


@mcp.prompt()
def task_workflow_prompt(task_id: str) -> str:
    """Generate a prompt for analyzing a task's workflow.

    This prompt template helps LLMs analyze task history and suggest next steps.

    Args:
        task_id: The task ID to analyze.
    """
    return f"""Analyze the task with ID '{task_id}' and its workflow history.

Use the following tools to gather information:
1. Call get_task(task_id="{task_id}") to get the current task details
2. Call get_task_history(task_id="{task_id}") to see all state transitions

Based on this information, provide:
- A summary of the task's current state
- The workflow path it has taken (all status changes)
- Recommendations for next steps or actions
- Any potential issues with the task's workflow

If this is part of a multi-step saga, consider:
- Whether compensation actions might be needed
- Dependencies on other tasks
- State consistency requirements
"""


def run_server(transport: str = "stdio"):
    """Run the MCP server.

    Args:
        transport: Transport type - 'stdio' or 'streamable-http'.
    """
    mcp.run(transport=transport)
