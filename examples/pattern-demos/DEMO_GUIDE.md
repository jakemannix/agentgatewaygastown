# AgentGateway Pattern Demos - Full Stack Guide

This guide walks through running the complete demo stack: custom MCP services,
the AgentGateway, and connecting agents to use the aggregated tools.

## Quick Start

```bash
cd examples/pattern-demos

# 1. Install dependencies
make setup

# 2. Start custom MCP services (SQLite-backed)
make start-mcp-services

# 3. Start the gateway (in another terminal)
make start-services

# 4. Run the interactive demo
make demo
```

## Architecture

```
Agents (Claude/ADK/LangGraph)
           │
           ▼
    AgentGateway:3000
           │
    ┌──────┴──────┐
    ▼             ▼
Pre-built      Custom Services
MCP servers    (HTTP on 8001-8004)
(stdio)
```

## Step-by-Step Setup

### Step 1: Start Custom MCP Services

These are SQLite-backed services with persistent data:

```bash
# Start all custom services
make start-mcp-services

# Check they're running
make check-mcp-services
```

Services started:
| Service | Port | Database | Description |
|---------|------|----------|-------------|
| document-service | 8001 | demo_data/document-service/documents.db | Semantic doc search |
| task-service | 8002 | demo_data/task-service/tasks.db | Task management |
| user-service | 8003 | demo_data/user-service/users.db | User profiles |
| notification-service | 8004 | demo_data/notification-service/notifications.db | Notifications |

### Step 2: Start AgentGateway

The gateway aggregates all MCP servers (pre-built + custom) into a single endpoint:

```bash
# Foreground (see logs)
make start-services

# Or background
make start-services-bg
```

Gateway endpoints:
- **MCP**: http://localhost:3000/mcp (streamable-http)
- **SSE**: http://localhost:3000/sse (for Claude Desktop, etc.)
- **A2A**: http://localhost:3000/a2a (agent-to-agent)
- **Admin UI**: http://localhost:15000/ui

### Step 3: Verify Tools are Available

```bash
# List all available tools through the gateway
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}'
```

You should see tools from all services:
- **fetch-server**: `fetch`
- **memory-server**: `read_graph`, `create_entities`, etc.
- **time-server**: `get_current_time`
- **document-service**: `create_document`, `search_documents`, etc.
- **task-service**: `create_task`, `list_tasks`, etc.
- **user-service**: `create_user`, `search_users_by_bio`, etc.
- **notification-service**: `send_notification`, `get_notifications`, etc.
- **Virtual tools**: `get_webpage`, `find_documents`, `add_todo`, `notify`, etc.

## Connecting Agents

### Option A: Interactive Demo Script

The built-in demo script connects to the gateway and exercises all patterns:

```bash
make demo
```

### Option B: Claude Desktop

Add to Claude Desktop's MCP settings (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "agentgateway": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "http://localhost:3000/sse"]
    }
  }
}
```

Then restart Claude Desktop. All tools from the gateway will be available.

### Option C: Claude Agent SDK

```bash
cd agents/claude_agent
uv run python -m claude_agent --prompt "Create a document about Python best practices, then create a task to review it"
```

The agent connects to `http://localhost:3000/sse` and can use all aggregated tools.

### Option D: Google ADK Agent

```bash
cd agents/google_adk_agent
uv run python -m google_adk_agent
```

The ADK agent discovers tools from the gateway and wraps them as ADK FunctionTools.

### Option E: Direct MCP Client (Python)

```python
import httpx
import asyncio

async def demo():
    async with httpx.AsyncClient() as client:
        # Initialize session
        init_resp = await client.post(
            "http://localhost:3000/mcp",
            json={
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "my-agent", "version": "1.0"}
                },
                "id": 1
            },
            headers={
                "Content-Type": "application/json",
                "Accept": "application/json, text/event-stream"
            }
        )
        session_id = init_resp.headers.get("mcp-session-id")
        
        # Create a document
        await client.post(
            "http://localhost:3000/mcp",
            json={
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "create_document",
                    "arguments": {
                        "title": "My Notes",
                        "content": "Important information about the project..."
                    }
                },
                "id": 2
            },
            headers={
                "Content-Type": "application/json",
                "Accept": "application/json, text/event-stream",
                "mcp-session-id": session_id
            }
        )
        
        # Search documents semantically
        search_resp = await client.post(
            "http://localhost:3000/mcp",
            json={
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "search_documents",
                    "arguments": {"query": "project information"}
                },
                "id": 3
            },
            headers={
                "Content-Type": "application/json",
                "Accept": "application/json, text/event-stream",
                "mcp-session-id": session_id
            }
        )
        print(search_resp.text)

asyncio.run(demo())
```

## Example Workflows

### Workflow 1: Document → Task → Notification

```
1. Create a document with findings
2. Create a task to act on the findings  
3. Send notification to the assignee
```

This can be done with the virtual tool `create_task_and_notify` or step by step:

```bash
# Using the demo script
uv run python run_demo.py --demo composition
```

### Workflow 2: Semantic Search Across Services

```
1. Search documents for "security"
2. Find users interested in "security" (bio search)
3. Create tasks for relevant users
```

### Workflow 3: Cross-Service Pipeline

The registry defines composed tools like `fetch_and_remember` that chain multiple services.

## Troubleshooting

### Services not starting?
```bash
# Check logs
tail -f /tmp/mcp-*.log

# Check ports
lsof -i :8001 -i :8002 -i :8003 -i :8004 -i :3000
```

### Gateway can't connect to services?
```bash
# Verify services are up
curl http://localhost:8001/mcp -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}'
```

### Tools not showing up?
Check the gateway logs at `/tmp/agentgateway-demo.log` for connection errors.

## Stopping Everything

```bash
make stop-all
```

Or individually:
```bash
make stop-mcp-services  # Stop custom services
make stop-services      # Stop gateway
```
