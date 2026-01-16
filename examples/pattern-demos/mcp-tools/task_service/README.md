# MCP Task Service

A SQLite-backed task management MCP server demonstrating CRUD operations, state transitions, and history tracking. Useful for demonstrating saga patterns (multi-step task workflows).

## Features

- **CRUD Operations**: Create, read, update, delete tasks
- **State Machine**: Validated status transitions with history
- **Filters**: List tasks by status, priority, assignee
- **Audit Trail**: Full history of state changes with timestamps
- **Statistics**: Aggregate task metrics

## Tools

| Tool | Description |
|------|-------------|
| `create_task` | Create a new task with title, description, priority, assignee, due date |
| `get_task` | Retrieve a task by ID |
| `update_task` | Update task fields (not status) |
| `delete_task` | Remove a task and its history |
| `transition_task` | Change task status with validation |
| `complete_task` | Convenience method to mark task completed |
| `list_tasks` | List tasks with filters and pagination |
| `get_task_history` | Get state transition history for a task |
| `get_task_stats` | Get aggregate statistics |

## Task Status State Machine

```
                 +-> cancelled
                 |
pending -----> in_progress -----> completed
    |              |
    |              v
    +--------> blocked
                   |
                   +-> cancelled
```

Valid transitions:
- `pending` → `in_progress`, `cancelled`
- `in_progress` → `pending`, `blocked`, `completed`, `cancelled`
- `blocked` → `in_progress`, `cancelled`
- `completed` → (terminal state)
- `cancelled` → (terminal state)

## Running the Example

### Prerequisites

- Python 3.10+
- [uv](https://docs.astral.sh/uv/) package manager
- agentgateway binary

### Install Dependencies

```bash
cd examples/pattern-demos/mcp-tools/task_service
uv sync
```

### Run with agentgateway

```bash
# From repository root
cargo run -- -f examples/pattern-demos/mcp-tools/task_service/config.yaml
```

### Test with MCP Inspector

```bash
npx @modelcontextprotocol/inspector
```

Connect to `http://localhost:3000/mcp` and explore the available tools.

## Example Usage

### Create a Task

```json
{
  "name": "task-service:create_task",
  "arguments": {
    "title": "Implement user authentication",
    "description": "Add OAuth2 support for user login",
    "priority": "high",
    "assignee": "alice",
    "due_date": "2025-02-01"
  }
}
```

### Transition to In Progress

```json
{
  "name": "task-service:transition_task",
  "arguments": {
    "task_id": 1,
    "new_status": "in_progress",
    "changed_by": "alice",
    "reason": "Starting sprint work"
  }
}
```

### List High Priority Tasks

```json
{
  "name": "task-service:list_tasks",
  "arguments": {
    "priority": "high",
    "status": "in_progress"
  }
}
```

### View Task History

```json
{
  "name": "task-service:get_task_history",
  "arguments": {
    "task_id": 1
  }
}
```

## Saga Pattern Demo

This service is designed to demonstrate saga patterns where multi-step workflows require coordinated state changes:

1. **Task Creation Saga**: Create task → assign → start
2. **Review Saga**: Submit → review → approve/reject → complete
3. **Blocking Saga**: Identify blocker → block task → resolve → unblock → complete

Each transition is recorded with `changed_by` and `reason` for full audit capability.

## Database

Tasks are stored in SQLite (`tasks.db` in the service directory). The database is automatically created on first run.

Set `TASK_SERVICE_DB` environment variable to customize the database location:

```bash
TASK_SERVICE_DB=/path/to/custom.db uv run python server.py
```

## Schema

```sql
-- Tasks table
CREATE TABLE tasks (
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
CREATE TABLE task_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    old_status TEXT,
    new_status TEXT NOT NULL,
    changed_by TEXT,
    reason TEXT,
    changed_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
```
