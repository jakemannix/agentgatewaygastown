# MCP Task Service

A SQLite-backed MCP server for task/todo management with state transition history tracking.

## Features

- **CRUD Operations**: Create, read, update, and delete tasks
- **Status Management**: Track task status through `pending`, `in_progress`, `completed`, `cancelled`
- **Priority Levels**: `low`, `medium`, `high`, `critical`
- **Assignee Tracking**: Assign tasks to users
- **State History**: Full audit trail of all status transitions
- **Filtering**: List tasks by status, priority, or assignee

## Use Cases

This server is designed to demonstrate:

1. **Saga Patterns**: Multi-step workflows where you need to track state transitions and potentially compensate failed steps
2. **Task Orchestration**: Managing work items across distributed systems
3. **Audit Requirements**: Full history tracking for compliance
4. **MCP Tool Patterns**: Best practices for building CRUD-based MCP tools

## Installation

```bash
cd examples/pattern-demos/mcp-tools/task_service
uv pip install -e .
```

## Running the Server

### STDIO Transport (for Claude Desktop, etc.)

```bash
# In-memory database
uv run python -m task_service

# With persistent database
uv run python -m task_service --db ./tasks.db
```

### HTTP Transport (for web clients)

```bash
uv run python -m task_service --transport streamable-http --port 8000
```

## MCP Tools

### `create_task`

Create a new task.

```json
{
  "title": "Implement feature X",
  "description": "Add support for...",
  "priority": "high",
  "assignee": "alice",
  "metadata": {"sprint": 42}
}
```

### `get_task`

Get a task by ID.

```json
{
  "task_id": "abc12345"
}
```

### `update_task`

Update task fields (only provided fields are changed).

```json
{
  "task_id": "abc12345",
  "title": "Updated title",
  "priority": "critical"
}
```

### `complete_task`

Mark a task as completed with optional reason.

```json
{
  "task_id": "abc12345",
  "reason": "Feature shipped in v2.0"
}
```

### `delete_task`

Permanently delete a task.

```json
{
  "task_id": "abc12345"
}
```

### `list_tasks`

List tasks with optional filters.

```json
{
  "status": "pending",
  "priority": "high",
  "assignee": "alice",
  "limit": 50,
  "offset": 0
}
```

### `transition_task_status`

Explicitly transition a task's status (useful for saga patterns).

```json
{
  "task_id": "abc12345",
  "new_status": "in_progress",
  "reason": "Starting work"
}
```

### `get_task_history`

Get the complete state transition history for a task.

```json
{
  "task_id": "abc12345"
}
```

## MCP Resources

### `schema://tasks`

Returns the database schema, useful for understanding the data model.

## MCP Prompts

### `task_workflow_prompt`

Generates a prompt for analyzing a task's workflow history, useful for LLMs to understand task state and suggest next actions.

## Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `TASK_SERVICE_DB` | SQLite database path | `:memory:` |

## Saga Pattern Example

The state transition tracking makes this service suitable for saga-style workflows:

```python
# Step 1: Create task for order processing
task = create_task(title="Process Order #123", metadata={"order_id": 123})

# Step 2: Start processing
transition_task_status(task["id"], "in_progress", reason="Payment received")

# Step 3: If downstream service fails, you can query history
history = get_task_history(task["id"])
# Use history to determine compensation actions

# Step 4: Complete or cancel based on outcome
complete_task(task["id"], reason="Order shipped")
# OR
transition_task_status(task["id"], "cancelled", reason="Inventory unavailable")
```

## Integration with Agent Gateway

Add to your Agent Gateway config:

```yaml
servers:
  - name: task-service
    transport:
      type: stdio
      command: uv
      args: ["run", "python", "-m", "task_service", "--db", "/data/tasks.db"]
```
