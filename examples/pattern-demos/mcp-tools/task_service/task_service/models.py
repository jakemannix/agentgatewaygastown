"""Pydantic models for the task service."""

from datetime import datetime
from enum import Enum
from typing import Optional

from pydantic import BaseModel, Field


class TaskStatus(str, Enum):
    """Task status values."""

    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"
    CANCELLED = "cancelled"


class TaskPriority(str, Enum):
    """Task priority levels."""

    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"


class Task(BaseModel):
    """A task/todo item."""

    id: str = Field(description="Unique task identifier")
    title: str = Field(description="Task title")
    description: Optional[str] = Field(default=None, description="Task description")
    status: TaskStatus = Field(default=TaskStatus.PENDING, description="Current status")
    priority: TaskPriority = Field(
        default=TaskPriority.MEDIUM, description="Task priority"
    )
    assignee: Optional[str] = Field(default=None, description="Assigned user")
    created_at: datetime = Field(description="Creation timestamp")
    updated_at: datetime = Field(description="Last update timestamp")
    completed_at: Optional[datetime] = Field(
        default=None, description="Completion timestamp"
    )
    metadata: Optional[dict] = Field(default=None, description="Additional metadata")


class TaskCreate(BaseModel):
    """Input for creating a task."""

    title: str = Field(description="Task title")
    description: Optional[str] = Field(default=None, description="Task description")
    priority: TaskPriority = Field(
        default=TaskPriority.MEDIUM, description="Task priority"
    )
    assignee: Optional[str] = Field(default=None, description="Assigned user")
    metadata: Optional[dict] = Field(default=None, description="Additional metadata")


class TaskUpdate(BaseModel):
    """Input for updating a task."""

    title: Optional[str] = Field(default=None, description="New title")
    description: Optional[str] = Field(default=None, description="New description")
    priority: Optional[TaskPriority] = Field(default=None, description="New priority")
    assignee: Optional[str] = Field(default=None, description="New assignee")
    metadata: Optional[dict] = Field(default=None, description="New metadata")


class StateTransition(BaseModel):
    """Record of a state transition."""

    id: str = Field(description="Transition record ID")
    task_id: str = Field(description="Associated task ID")
    from_status: Optional[TaskStatus] = Field(description="Previous status")
    to_status: TaskStatus = Field(description="New status")
    changed_by: Optional[str] = Field(default=None, description="User who made change")
    changed_at: datetime = Field(description="Timestamp of change")
    reason: Optional[str] = Field(default=None, description="Reason for transition")


class TaskListFilter(BaseModel):
    """Filters for listing tasks."""

    status: Optional[TaskStatus] = Field(default=None, description="Filter by status")
    priority: Optional[TaskPriority] = Field(
        default=None, description="Filter by priority"
    )
    assignee: Optional[str] = Field(default=None, description="Filter by assignee")
    limit: int = Field(default=100, description="Maximum results to return")
    offset: int = Field(default=0, description="Offset for pagination")
