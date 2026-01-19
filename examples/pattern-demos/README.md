# AgentGateway Pattern Demos

This directory contains comprehensive demos showcasing all major AgentGateway patterns. Use these to understand how AgentGateway can enhance your agentic AI infrastructure.

## Quick Start

```bash
# 1. Setup dependencies
make setup

# 2. Start the gateway (in one terminal)
make start-services

# 3. Run the interactive demo (in another terminal)
make demo
```

## What's Included

| File | Description |
|------|-------------|
| `docker-compose.yml` | Docker orchestration for all services |
| `start_gateway.sh` | Shell script to launch gateway with configs |
| `run_demo.py` | Interactive Python demo showing all patterns |
| `Makefile` | Convenient targets for setup and demos |
| `configs/demo-config.yaml` | Gateway configuration with all patterns |
| `configs/registry.json` | Virtual tool definitions |

## Patterns Demonstrated

### 1. MCP Multiplexing

AgentGateway aggregates multiple MCP servers into a single endpoint. Clients see a unified tool catalog from different backend servers.

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Client    │────▶│  AgentGateway    │────▶│  fetch-server   │
│  (Claude,   │     │  localhost:3000  │     ├─────────────────┤
│  LangGraph) │     │                  │────▶│  memory-server  │
└─────────────┘     │                  │     ├─────────────────┤
                    │                  │────▶│  time-server    │
                    └──────────────────┘     └─────────────────┘
```

**Try it:**
```bash
# List all tools from all servers
make list-tools

# Or use MCP Inspector
make inspector
```

### 2. Tool Aliasing

Virtual tools map semantic names to underlying tools, making tool discovery more natural for AI agents.

```json
{
  "name": "get_webpage",
  "description": "Semantic alias for fetch",
  "source": {"target": "fetch-server", "tool": "fetch"}
}
```

Multiple aliases can point to the same underlying tool:
- `fetch` → `fetch-server:fetch`
- `get_webpage` → `fetch-server:fetch`
- `browse` → `fetch-server:fetch`

### 3. Output Projection

Extract specific fields from complex responses to reduce token usage and simplify agent processing.

```json
{
  "name": "list_entity_names",
  "source": {"target": "memory-server", "tool": "read_graph"},
  "outputTransform": {
    "mappings": {
      "names": {"path": "$.entities[*].name"}
    }
  }
}
```

**Before (read_graph):**
```json
{
  "entities": [
    {"name": "Alice", "entityType": "person", "observations": ["..."]},
    {"name": "Bob", "entityType": "person", "observations": ["..."]}
  ],
  "relations": [...]
}
```

**After (list_entity_names):**
```json
{
  "names": ["Alice", "Bob"]
}
```

### 4. Output Transformation

Restructure and flatten nested JSON responses for consistent output formats.

```json
{
  "name": "get_connections",
  "outputTransform": {
    "mappings": {
      "connections[*].subject": {"path": "$.from"},
      "connections[*].predicate": {"path": "$.relationType"},
      "connections[*].object": {"path": "$.to"}
    }
  }
}
```

### 5. Tool Composition (Pipelines)

Chain multiple tools into a single operation for complex workflows.

```json
{
  "name": "fetch_and_remember",
  "spec": {
    "pipeline": {
      "steps": [
        {"id": "fetch_content", "operation": {"tool": {"name": "fetch"}}},
        {"id": "store_note", "operation": {"tool": {"name": "create_entities"}}}
      ]
    }
  }
}
```

### 6. A2A Protocol Proxy

Proxy Agent-to-Agent (A2A) protocol traffic with URL rewriting and observability.

```yaml
# Gateway config for A2A
- policies:
    a2a: {}
  backends:
  - host: localhost:9999
```

## Interactive Agent Demos

The best way to explore AgentGateway's compositional tools is through interactive chat with an AI agent. We provide two agent implementations:

### Claude Agent (Claude Agent SDK)

```bash
cd agents/claude_agent

# Start interactive chat (recommended)
python agent.py --chat

# One-shot with custom prompt
python agent.py --prompt "Search for documents about APIs and create tasks"

# Run a predefined scenario
python agent.py --scenario document_search
```

The Claude agent uses the Claude Agent SDK with ClaudeSDKClient for multi-turn conversations. In chat mode, you can:
- Ask questions and give commands naturally
- Watch the agent reason through tool selection
- See tool calls and results in real-time
- Type `quit` to exit, `tools` to list available tools

### Google ADK Agent

```bash
cd agents/google_adk_agent

# Interactive CLI chat (launches 'adk run')
python -m google_adk_agent --chat

# Web interface with chat UI (launches 'adk web')
python -m google_adk_agent --web

# One-shot demo
python -m google_adk_agent --demo
```

The ADK agent uses Google's built-in `adk run` (terminal) and `adk web` (browser) for the best interactive experience. The web interface provides a visual chat UI with tool execution visualization.

### Prerequisites

Both agents require:
1. AgentGateway running with MCP services
2. API credentials (Anthropic API key for Claude, Google API key for ADK)

```bash
# Terminal 1: Start the custom MCP services (document, task, user, notification)
make start-mcp-services

# Terminal 2: Start the gateway with full v2 registry
make start-services

# Terminal 3: Start an agent
cd agents/claude_agent && python agent.py --chat
```

### Agent Tool Scoping

Each agent declares its tool dependencies in the registry. When connecting, agents pass their identity via HTTP headers (`X-Agent-Name`, `X-Agent-Version`), and the gateway scopes their tool visibility accordingly.

**Claude Agent** (`claude-demo-agent`) - Research & Knowledge focus:
| Tool | Type | Description |
|------|------|-------------|
| `search_documents` | Source | Semantic document search |
| `create_document` | Source | Create searchable documents |
| `fetch` | Source | Fetch web content |
| `read_graph` | Source | Query knowledge graph |
| `create_entities` | Source | Add to knowledge graph |
| `fetch_and_store` | Pipeline | Fetch URL → store as document |

**ADK Agent** (`adk-demo-agent`) - Task Orchestration & Saga focus:
| Tool | Type | Description |
|------|------|-------------|
| `create_task` | Source | Create tasks |
| `list_tasks` | Source | List/filter tasks |
| `send_notification` | Source | Send notifications |
| `multi_search` | Scatter-Gather | Parallel search across services |
| `process_order` | Saga | Order with inventory/payment/shipping |
| `create_task_and_notify` | Pipeline | Create task → notify assignee |

This demonstrates how different agents can have different tool views from the same gateway, based on their declared dependencies in the registry.

## Framework Integration Examples

### Claude Code

Configure Claude Code to use AgentGateway as an MCP server:

```json
{
  "mcpServers": {
    "agentgateway": {
      "command": "npx",
      "args": ["@anthropic/claude-code-mcp-client", "http://localhost:3000/sse"]
    }
  }
}
```

Or use SSE transport directly: `http://localhost:3000/sse`

### LangGraph

```python
from langchain_mcp import MCPToolkit

toolkit = MCPToolkit(
    transport="sse",
    url="http://localhost:3000/sse"
)

tools = toolkit.get_tools()

# Use with LangGraph agent
from langgraph.prebuilt import create_react_agent
agent = create_react_agent(llm, tools)
```

### Google ADK

```python
from google.adk import AgentClient

client = AgentClient(
    agent_url="http://localhost:3000/a2a"
)

# Get agent capabilities
card = client.get_agent_card()

# Send message
response = client.send_message("Hello, agent!")
```

## Directory Structure

```
examples/pattern-demos/
├── README.md               # This file
├── Makefile                # Build/run targets
├── docker-compose.yml      # Docker orchestration
├── start_gateway.sh        # Gateway launcher
├── run_demo.py             # Interactive demo
├── configs/
│   ├── demo-config.yaml    # Gateway configuration
│   └── registry.json       # Virtual tool definitions
└── a2a-agents/             # Sample A2A agents (optional)
```

## Make Targets

```bash
# Setup
make setup              # Install dependencies
make build              # Build agentgateway

# Services
make start-services     # Start gateway (foreground)
make stop-services      # Stop background services

# Demos
make demo               # Interactive demo (all patterns)
make demo-multiplexing  # MCP multiplexing
make demo-aliasing      # Tool aliasing
make demo-projection    # Output projection
make demo-transformation# Output transformation
make demo-composition   # Tool composition
make demo-a2a           # A2A protocol

# Framework demos
make run-claude         # Claude Code setup
make run-langgraph      # LangGraph integration
make run-adk            # Google ADK integration

# Docker
make docker-up          # Start via Docker Compose
make docker-down        # Stop Docker services

# Utilities
make check              # Check if gateway is running
make list-tools         # List available tools
make inspector          # Open MCP Inspector
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `GATEWAY_PORT` | 3000 | Main MCP/HTTP port |
| `ADMIN_PORT` | 15000 | Admin UI port |
| `LOG_LEVEL` | info | Log level (debug, info, warn, error) |

### Endpoints

| Endpoint | Protocol | Description |
|----------|----------|-------------|
| `/mcp` | MCP Streamable HTTP | Main MCP endpoint |
| `/sse` | MCP SSE | Server-Sent Events transport |
| `/a2a` | A2A | Agent-to-Agent protocol |
| `/ui` (port 15000) | HTTP | Admin dashboard |

## Troubleshooting

### Gateway won't start

1. Check if port is in use: `lsof -i :3000`
2. Build the binary: `make build`
3. Check logs: `./start_gateway.sh 2>&1 | head -50`

### MCP servers fail to spawn

Ensure required tools are installed:
```bash
# Node.js MCP servers
npm install -g @modelcontextprotocol/server-everything
npm install -g @modelcontextprotocol/server-memory

# Python MCP servers
pip install mcp-server-fetch mcp-server-time
# Or use uvx (recommended)
```

### Demo script errors

Install Python dependencies:
```bash
pip install httpx rich
```

### A2A agent not reachable

Start the sample A2A agent:
```bash
cd ../a2a/strands-agents
uv run .
```

## Learn More

- [AgentGateway Documentation](https://agentgateway.dev/docs)
- [MCP Protocol Specification](https://modelcontextprotocol.io)
- [A2A Protocol](https://a2aproject.github.io/A2A/)
- [Virtual Tools Design Doc](../../docs/virtual-tools.md)
