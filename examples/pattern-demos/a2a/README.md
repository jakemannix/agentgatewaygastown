# A2A: Agent-to-Agent Communication Demo

This example demonstrates A2A (Agent-to-Agent) communication using agentgateway,
showing how agents can discover each other and delegate tasks in a chain.

## Architecture

```
                    ┌─────────────────┐
                    │     Client      │
                    └────────┬────────┘
                             │ A2A message
                             ▼
                    ┌─────────────────┐
                    │  AgentGateway   │
                    │    :3000        │
                    └────────┬────────┘
                             │
                             ▼
              ┌──────────────────────────┐
              │    Claude Delegator      │
              │    (Orchestrator)        │
              │         :9001            │
              └──────────┬───────────────┘
                         │ Delegates workflow tasks
                         ▼
              ┌──────────────────────────┐
              │   LangGraph Processor    │
              │   (Workflow Engine)      │
              │         :9002            │
              └──────────┬───────────────┘
                         │ Delegates specialist tasks
                         ▼
              ┌──────────────────────────┐
              │   Google ADK Specialist  │
              │   (Domain Expert)        │
              │         :9003            │
              └──────────────────────────┘
```

## Components

### 1. Claude Delegator (Port 9001)
The entry point orchestrator that:
- Analyzes incoming tasks
- Determines optimal delegation strategy
- Routes to specialized agents
- Synthesizes final results

### 2. LangGraph Processor (Port 9002)
A workflow engine that:
- Executes multi-step workflows
- Maintains state across steps
- Delegates specialist tasks to ADK agent
- Handles data transformation pipelines

### 3. Google ADK Specialist (Port 9003)
A domain expert agent that:
- Performs specialized operations
- Provides domain-specific capabilities
- Returns structured results

## Running the Demo

### Prerequisites

- Python 3.11+
- uv (Python package manager)
- Rust toolchain (for agentgateway)

### Step 1: Start the Agents

Start each agent in a separate terminal:

```bash
# Terminal 1: Claude Delegator
cd examples/pattern-demos/a2a/agents/claude-delegator
uv run python -m claude_delegator

# Terminal 2: LangGraph Processor
cd examples/pattern-demos/a2a/agents/langgraph-processor
uv run python -m langgraph_processor

# Terminal 3: ADK Specialist
cd examples/pattern-demos/a2a/agents/adk-specialist
uv run python -m adk_specialist
```

### Step 2: Start AgentGateway

```bash
cargo run -- -f examples/pattern-demos/a2a/config.yaml
```

### Step 3: Send a Test Request

```bash
# Get the agent card
curl http://localhost:3000/.well-known/agent.json | jq

# Send a message
curl -X POST http://localhost:3000 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "message/send",
    "params": {
      "message": {
        "role": "user",
        "content": [{"kind": "Text", "text": "Process this workflow with GCP data"}]
      }
    }
  }'
```

Or use the test client:

```bash
cd examples/pattern-demos/a2a/client
uv run python -m a2a_client
```

## Agent Cards

Each agent exposes its capabilities via the A2A agent card at `/.well-known/agent.json`:

```json
{
  "name": "Claude Delegator",
  "description": "Orchestrating agent for task delegation",
  "url": "http://localhost:3000",
  "protocolVersion": "0.3.0",
  "capabilities": {
    "streaming": true
  },
  "skills": [
    {
      "id": "task-analysis",
      "name": "Task Analysis",
      "description": "Analyzes tasks and determines delegation strategy"
    }
  ]
}
```

## Discovery Configuration

The `discovery.json` file defines the agent registry for discovery:

- Lists all available agents with their URLs and capabilities
- Defines routing rules for task delegation
- Supports pattern-based routing (e.g., `*workflow*` -> LangGraph)

## Key A2A Concepts Demonstrated

1. **Agent Discovery**: Agents discover each other via well-known endpoints
2. **Task Delegation**: Orchestrator routes tasks to specialized agents
3. **Capability Matching**: Tasks are matched to agents based on skills
4. **URL Rewriting**: Gateway rewrites agent URLs to ensure traffic flows through it
5. **Streaming Support**: All agents support streaming responses

## Gateway Features

AgentGateway provides:

- **URL Rewriting**: Agent card URLs are rewritten to route through gateway
- **A2A Telemetry**: Logs include `a2a.method` for debugging
- **CORS Support**: Cross-origin requests enabled for browser clients
- **Protocol Support**: Handles both legacy and current A2A protocol versions
