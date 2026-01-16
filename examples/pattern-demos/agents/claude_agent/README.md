# Claude Agent SDK ReAct Demo

This example demonstrates how to use the [Claude Agent SDK](https://platform.claude.com/docs/en/agent-sdk/overview) to build a ReAct-style agent that connects to agentgateway for virtual composed tools.

## Overview

The agent implements a ReAct (Reasoning + Acting) pattern:
1. **THINK**: Analyzes what needs to be done
2. **ACT**: Selects and executes tools from agentgateway
3. **OBSERVE**: Reviews tool results
4. **REPEAT**: Continues until the task is complete

This pattern enables autonomous task completion using tools exposed through agentgateway's MCP interface.

## Prerequisites

1. **Python 3.10+**
2. **Claude Code CLI** - The Agent SDK uses Claude Code as its runtime:
   ```bash
   # macOS/Linux
   curl -fsSL https://claude.ai/install.sh | bash

   # Or via Homebrew
   brew install --cask claude-code
   ```
3. **Anthropic API key** or **Claude Code authentication** (run `claude` once to authenticate)
4. **agentgateway running** with MCP tools configured

## Installation

```bash
# Navigate to this example
cd examples/pattern-demos/agents/claude_agent

# Install with uv (recommended)
uv sync

# Or with pip
pip install -e .
```

## Running agentgateway

Before running the agent, start agentgateway with MCP tools. You can use the provided config or your own:

```bash
# From the repository root, using the sample config
cargo run -- -f examples/pattern-demos/agents/claude_agent/config.yaml

# Or use the basic example
cargo run -- -f examples/basic/config.yaml
```

## Usage

### Basic Usage

```bash
# Run with default demo prompt
uv run python agent.py

# Or with pip-installed package
python agent.py
```

### Custom Prompts

```bash
# Use a custom task prompt
uv run python agent.py --prompt "List all available tools and explain what each does"

# Document analysis task
uv run python agent.py --prompt "Find documents about authentication and summarize key points"
```

### Configuration Options

```bash
# Connect to a different gateway URL
uv run python agent.py --gateway-url http://gateway.example.com:3000

# With authentication token
uv run python agent.py --auth-token "your-bearer-token"

# Quiet mode (only show final result)
uv run python agent.py --quiet
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AGENTGATEWAY_URL` | URL of the agentgateway instance | `http://localhost:3000` |
| `AGENTGATEWAY_AUTH_TOKEN` | Bearer token for authentication | (none) |
| `ANTHROPIC_API_KEY` | Anthropic API key (if not using Claude Code auth) | (none) |

## Demo Scenario

The default demo scenario is "document tasks" - finding relevant documents and creating actionable tasks from them:

```bash
uv run python agent.py --prompt "Help me find relevant documents about API design and create tasks for implementing the suggestions."
```

This demonstrates:
- Connecting to agentgateway via MCP SSE transport
- Discovering available tools dynamically
- Multi-step reasoning and tool use
- Task creation based on document analysis

## Example Output

```
============================================================
Claude Agent SDK ReAct Demo
============================================================

Prompt: Help me find relevant documents about API design...
Gateway: http://localhost:3000
------------------------------------------------------------
[Connected] MCP server: agentgateway
  Available tools: search_documents, create_task, list_tasks, get_document, update_task

[Thinking] I need to search for documents related to API design first...

[Action] Calling tool: mcp__agentgateway__search_documents
  Input: {"query": "API design best practices"}

[Thinking] Found several relevant documents. Let me extract key action items...

[Action] Calling tool: mcp__agentgateway__create_task
  Input: {"title": "Implement REST versioning", "description": "..."}

[Complete] Task finished successfully

============================================================
Final Result:
============================================================
Created 3 tasks based on API design documents:
1. Implement REST versioning strategy
2. Add OpenAPI specification
3. Set up API rate limiting
```

## Architecture

```
                    ┌─────────────────────┐
                    │   Claude Agent SDK  │
                    │    (ReAct Agent)    │
                    └──────────┬──────────┘
                               │
                               │ MCP over SSE
                               │
                    ┌──────────▼──────────┐
                    │    agentgateway     │
                    │  (MCP Server Proxy) │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
     ┌────────▼────────┐ ┌─────▼─────┐ ┌────────▼────────┐
     │  Tool Server 1  │ │ Tool 2... │ │  Tool Server N  │
     │  (Documents)    │ │           │ │    (Tasks)      │
     └─────────────────┘ └───────────┘ └─────────────────┘
```

## Programmatic Usage

You can also use the agent as a library:

```python
import asyncio
from agent import run_agent

async def main():
    result = await run_agent(
        prompt="What tools are available?",
        gateway_url="http://localhost:3000",
        verbose=True,
    )
    print(result)

asyncio.run(main())
```

## Customization

### Custom System Prompts

Modify the `build_system_prompt()` function in `agent.py` to customize agent behavior:

```python
def build_system_prompt(scenario: str = "custom") -> str:
    return """You are a specialized agent for [your use case].

    Always follow these guidelines:
    1. [Your guidelines here]
    2. ...
    """
```

### Tool Filtering

Restrict which tools the agent can use:

```python
options = ClaudeAgentOptions(
    mcp_servers=mcp_servers,
    allowed_tools=[
        "mcp__agentgateway__search_documents",
        "mcp__agentgateway__list_tasks",
    ],  # Only allow specific tools
)
```

## Troubleshooting

### "Claude Code not found"

Install Claude Code:
```bash
curl -fsSL https://claude.ai/install.sh | bash
```

Then restart your terminal and run `claude` once to authenticate.

### "MCP server failed to connect"

1. Ensure agentgateway is running:
   ```bash
   curl http://localhost:3000/health
   ```

2. Check the gateway URL matches your configuration

3. Verify authentication token if required

### "No tools available"

Ensure agentgateway is configured with MCP tool backends. Check the gateway config file has `mcp` backends defined.

## Related Examples

- [Basic](../../../basic/README.md) - Simple agentgateway setup
- [Multiplex](../../../multiplex/README.md) - Multiple tool servers
- [Modal Performance](../../../modal-perf/README.md) - Performance testing patterns
