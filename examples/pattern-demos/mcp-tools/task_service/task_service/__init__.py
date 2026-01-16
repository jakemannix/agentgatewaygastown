"""MCP Task Service - SQLite-backed task/todo management."""

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
from .server import mcp, run_server

__all__ = [
    "mcp",
    "run_server",
    "TaskDatabase",
    "Task",
    "TaskCreate",
    "TaskUpdate",
    "TaskListFilter",
    "TaskStatus",
    "TaskPriority",
    "StateTransition",
]


def main():
    """Entry point for the task service."""
    run_server()
