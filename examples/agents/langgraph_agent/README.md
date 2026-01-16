# LangGraph ReAct Agent with agentgateway

This example demonstrates a LangGraph-based ReAct agent that connects to agentgateway's MCP endpoint for dynamic tool discovery and orchestration.

## Features

- **LangGraph Integration**: Built on LangGraph with langchain-anthropic for state management
- **MCP Tool Discovery**: Dynamically loads tools from agentgateway MCP endpoint
- **ReAct Pattern**: Implements reasoning-action-observation loop
- **State Management**: Tracks execution state, tool calls, and phases
- **Execution Visualization**: Rich terminal visualization of agent execution

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   LangGraph ReAct Agent                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      [START]                                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       agent                                 │
│  • Receives messages                                        │
│  • Reasons about next action                                │
│  • Decides: use tools or respond                            │
└─────────────────────────────────────────────────────────────┘
                              │
               ┌──────────────┴──────────────┐
               │ has_tool_calls?             │
               │                             │
               ▼                             ▼
┌─────────────────────┐    ┌─────────────────────────────────┐
│       tools         │    │           [END]                  │
│  • Execute MCP tool │    │  • Return final response        │
│  • Return result    │    └─────────────────────────────────┘
└─────────────────────┘
               │
               └───────────────────────────────┐
                                               ▼
                                      (back to agent)
```

## Prerequisites

1. **agentgateway running**: Start agentgateway with an MCP backend
   ```bash
   cargo run -- -f examples/basic/config.yaml
   ```

2. **Python dependencies**: Install with uv or pip
   ```bash
   cd examples/agents/langgraph_agent
   uv sync
   # or
   pip install -e .
   ```

3. **Anthropic API key**: Set your API key
   ```bash
   export ANTHROPIC_API_KEY="your-api-key"
   ```

## Usage

### Basic Usage

```bash
# Run the demo scenario
python -m langgraph_agent

# Or with uv
uv run python -m langgraph_agent
```

### Command Line Options

```bash
# Custom gateway URL
python -m langgraph_agent --gateway-url http://localhost:3000

# Custom model
python -m langgraph_agent --model claude-sonnet-4-20250514

# Custom prompt
python -m langgraph_agent --prompt "Search for information about MCP and summarize it"

# Show graph structure
python -m langgraph_agent --show-graph

# Save graph visualization
python -m langgraph_agent --save-graph graph.png

# Async execution
python -m langgraph_agent --async

# Verbose output
python -m langgraph_agent -v
```

### Demo Scenario

The default demo scenario demonstrates a multi-step workflow:

1. **Research**: Use available tools to research AI agent developments
2. **Summarize**: Synthesize findings into key insights
3. **Notify**: Format a team notification with highlights

```bash
python -m langgraph_agent
```

## Module Structure

```
langgraph_agent/
├── __init__.py        # Package exports
├── __main__.py        # CLI entry point and demo
├── agent.py           # LangGraph ReAct agent implementation
├── mcp_client.py      # MCP client for agentgateway
├── tools.py           # LangChain tool adapters for MCP
├── visualization.py   # Execution trace visualization
├── pyproject.toml     # Package configuration
└── README.md          # This file
```

## Key Components

### MCPClient

Connects to agentgateway's MCP endpoint for tool discovery and execution:

```python
from langgraph_agent import MCPClient

with MCPClient("http://localhost:3000") as client:
    tools = client.list_tools()
    result = client.call_tool("everything:echo", {"message": "hello"})
```

### MCPToolProvider

Converts MCP tools into LangChain-compatible tools:

```python
from langgraph_agent import MCPClient, MCPToolProvider

with MCPClient("http://localhost:3000") as client:
    provider = MCPToolProvider(client)
    langchain_tools = provider.load_tools()
```

### create_react_agent

Creates a LangGraph-based ReAct agent:

```python
from langgraph_agent import create_react_agent

graph, trace = create_react_agent(
    tools=langchain_tools,
    model="claude-sonnet-4-20250514",
    max_iterations=10,
)
```

### AgentState

The agent maintains rich state throughout execution:

```python
class AgentState(TypedDict):
    messages: Sequence[BaseMessage]  # Conversation history
    step_count: int                   # Iteration counter
    tool_calls_made: list[dict]       # Tool invocation log
    current_phase: str                # Workflow phase tracking
```

## Visualization

The agent provides rich terminal visualization:

- **Execution Trace**: Tree view of all reasoning and tool steps
- **State Summary**: Table showing current agent state
- **Graph Structure**: ASCII diagram of the LangGraph structure

```bash
# Enable visualization
python -m langgraph_agent --show-graph
```

## State Management

The agent tracks state across multiple dimensions:

1. **Message History**: Full conversation context
2. **Step Counting**: Prevents infinite loops
3. **Tool Call Log**: Complete record of tool invocations
4. **Phase Tracking**: Semantic workflow phases (research, summarizing, notifying)

## Extending

### Custom System Prompt

```python
graph, trace = create_react_agent(
    tools=tools,
    system_prompt="You are a research assistant specializing in AI topics.",
)
```

### Adding Local Tools

```python
from langchain_core.tools import tool

@tool
def my_local_tool(query: str) -> str:
    """A custom local tool."""
    return f"Result for: {query}"

# Combine MCP and local tools
all_tools = mcp_tools + [my_local_tool]
graph, trace = create_react_agent(tools=all_tools)
```

## Related Examples

- [basic](../../basic/) - Simple agentgateway configuration
- [a2a](../../a2a/) - Agent-to-Agent protocol
- [modal-perf](../../modal-perf/) - Performance testing patterns
