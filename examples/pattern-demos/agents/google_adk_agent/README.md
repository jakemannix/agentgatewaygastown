# Google ADK Agent: Multi-step Project Setup with Saga Pattern

This example demonstrates [Google ADK (Agent Development Kit)](https://google.github.io/adk-docs/) integration with agentgateway, showcasing:

1. **Agent Composition**: Hierarchical agent structure with a coordinator and specialized sub-agents
2. **Saga Pattern**: Distributed transaction pattern with compensating actions for rollback
3. **AgentGateway Integration**: Connecting ADK agents to MCP tools via agentgateway

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Coordinator Agent                             │
│  (Orchestrates saga workflow, delegates to sub-agents)          │
├─────────────┬─────────────┬─────────────┬─────────────────────────┤
│  Project    │    Git      │   Config    │   Dependencies          │
│  Init Agent │   Agent     │   Agent     │   Agent                 │
│             │             │             │                         │
│ • create    │ • init repo │ • pyproject │ • add deps              │
│ • rollback  │ • gitignore │ • README    │ • version file          │
└─────────────┴─────────────┴─────────────┴─────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │  AgentGateway   │
                    │   (MCP Tools)   │
                    └─────────────────┘
```

## Saga Pattern Implementation

The saga pattern ensures that multi-step operations can be safely rolled back if any step fails:

```
Step 1: Create Project ──► Step 2: Init Git ──► Step 3: Config ──► Step 4: Dependencies
    │                          │                    │                   │
    ▼                          ▼                    ▼                   ▼
Compensate:              Compensate:           Compensate:         Compensate:
Remove dirs              Remove .git           Remove files        Reset deps
```

If Step 3 fails, the saga automatically executes compensating actions for Steps 1 and 2 in reverse order.

## Prerequisites

- Python 3.10+
- [uv](https://github.com/astral-sh/uv) (recommended) or pip
- Google API key (for Gemini models)
- Node.js/npx (for MCP servers)

## Setup

### 1. Set up Google API credentials

```bash
export GOOGLE_API_KEY="your-api-key"
```

### 2. Install dependencies

```bash
cd examples/pattern-demos/agents/google_adk_agent
uv sync
```

### 3. Start agentgateway (for MCP tools)

For access to sophisticated virtual tools (pipelines, scatter-gather, sagas):

```bash
# From the pattern-demos directory
cd examples/pattern-demos

# Start the custom MCP services
make start-mcp-services

# Start the gateway with full v2 registry
make start-services
```

This gives you access to 27 virtual tools including:
- **Pipelines**: `fetch_and_store`, `search_and_summarize`
- **Scatter-Gather**: `multi_search` (parallel search)
- **Saga**: `order_saga` (distributed transactions)

For minimal setup with just basic tools:
```bash
cargo run -- -f examples/pattern-demos/agents/google_adk_agent/config.yaml
```

## Running the Agent

### Interactive Chat Mode (Recommended)

The best way to explore the agent is through interactive chat:

```bash
# CLI chat - launches 'adk run' with full conversation support
python -m google_adk_agent --chat

# Web interface - launches 'adk web' with visual chat UI
python -m google_adk_agent --web
```

Or use ADK commands directly:

```bash
# Interactive CLI (same as --chat)
adk run .

# Web interface with chat UI (same as --web)
adk web .
```

The web interface (`adk web`) provides:
- Visual chat UI in your browser
- Tool execution visualization
- Session management
- Response streaming

### One-Shot Demo Mode

For testing or scripting:

```bash
# Run a demo saga execution
python -m google_adk_agent --demo

# With custom gateway URL
python -m google_adk_agent --demo --gateway-url http://localhost:3000
```

## Example Interaction

```
User: Set up a new Python project called "my-api"

Agent: I'll coordinate the project setup using the saga pattern.

[Delegating to project_init_agent]
✓ Created project structure at /tmp/projects/my-api
  - src/
  - tests/
  - docs/
  - .github/workflows/

[Delegating to git_agent]
✓ Initialized git repository
  - Branch: main
  - Created .gitignore

[Delegating to config_agent]
✓ Created configuration files
  - pyproject.toml
  - README.md

[Delegating to dependencies_agent]
✓ Set up dependencies
  - pytest
  - ruff
  - Python version: 3.11

Saga completed successfully. Project ready at /tmp/projects/my-api
```

### Rollback Example

If git initialization fails:

```
[Delegating to project_init_agent]
✓ Created project structure

[Delegating to git_agent]
✗ Git initialization failed: git not found

[Executing compensations]
✓ Compensated: Removed project structure

Saga rolled back. No partial state remains.
```

## Code Structure

```
google_adk_agent/
├── __init__.py          # Package init
├── __main__.py          # Entry point and CLI
├── agent.py             # ADK agents and saga implementation
├── gateway_tools.py     # AgentGateway MCP integration
├── config.yaml          # AgentGateway configuration
├── pyproject.toml       # Python project config
└── README.md            # This file
```

## Key Concepts

### Agent Composition (ADK's sub_agents)

```python
coordinator = Agent(
    name="project_setup_coordinator",
    model="gemini-2.0-flash",
    sub_agents=[project_init, git_agent, config_agent, deps_agent],
    tools=[execute_saga, get_saga_status],
)
```

### Saga Steps with Compensations

```python
@dataclass
class SagaStep:
    name: str
    status: StepStatus
    result: dict[str, Any]
    error: str | None

# Each action has a compensating action
def create_project_structure(...) -> dict: ...
def compensate_project_structure(...) -> dict: ...
```

### Gateway Tools Discovery

```python
async def discover_gateway_tools(gateway_url: str) -> list[FunctionTool]:
    client = AgentGatewayMCPClient(gateway_url)
    mcp_tools = await client.list_tools()
    return [create_gateway_tool(client, tool) for tool in mcp_tools]
```

## Tool Scoping (Registry v2)

This agent identifies itself to the gateway as `adk-demo-agent` via HTTP headers. The gateway uses this identity to scope tool visibility based on the agent's declared dependencies in the registry.

### How It Works

1. **Agent identity headers** are sent with every request:
   ```python
   headers = {
       "X-Agent-Name": "adk-demo-agent",
       "X-Agent-Version": "1.0.0",
   }
   ```

2. **Registry declaration** in `configs/registry-v2-example.json`:
   ```json
   {
     "name": "adk-demo-agent",
     "capabilities": {
       "extensions": [{
         "uri": "urn:agentgateway:sbom",
         "params": {
           "depends": [
             { "type": "tool", "name": "create_task", "version": "1.0.0" },
             { "type": "tool", "name": "process_order", "version": "1.0.0" }
           ]
         }
       }]
     }
   }
   ```

3. **Gateway runtime hooks** match the agent identity to its declared dependencies and filter the tool list accordingly.

### Tools Available to This Agent

| Tool | Type | Description |
|------|------|-------------|
| `create_task` | Source | Create tasks |
| `list_tasks` | Source | List/filter tasks |
| `complete_task` | Source | Mark tasks done |
| `list_users` | Source | List users |
| `search_users_by_bio` | Source | Semantic user search |
| `send_notification` | Source | Send notifications |
| `multi_search` | Scatter-Gather | Parallel search across services |
| `process_order` | Saga | Order with inventory/payment/shipping |
| `create_task_and_notify` | Pipeline | Create task → notify assignee |

This is a focused subset optimized for **task orchestration and distributed transactions**. The Claude demo agent has a different subset focused on research and knowledge management.

## Integration with AgentGateway

The agent can discover and use MCP tools from agentgateway:

```python
from gateway_tools import create_gateway_enhanced_agent

# Create agent with gateway tools
agent = create_gateway_enhanced_agent("http://localhost:3000")

# Tools from gateway are automatically available
# e.g., everything:echo, filesystem:read, git:status
```

## Extending the Example

### Add Custom Sub-Agent

```python
def create_testing_agent() -> Agent:
    return Agent(
        name="testing_agent",
        model="gemini-2.0-flash",
        description="Sets up testing infrastructure",
        tools=[
            FunctionTool(setup_pytest),
            FunctionTool(compensate_testing),
        ],
    )
```

### Add Gateway Tools

Update `config.yaml` to include additional MCP servers:

```yaml
backends:
- mcp:
    targets:
    - name: custom-tool
      stdio:
        cmd: python
        args: ["-m", "my_mcp_server"]
```

## Related Documentation

- [Google ADK Documentation](https://google.github.io/adk-docs/)
- [ADK Multi-Agent Systems](https://google.github.io/adk-docs/agents/multi-agents/)
- [AgentGateway MCP Documentation](../../README.md)
- [Saga Pattern](https://microservices.io/patterns/data/saga.html)
