# Modal.com Integration for Agentgateway Performance Testing

This example provides serverless performance testing for agentgateway using [Modal.com](https://modal.com/). It allows you to run load tests against agentgateway endpoints at scale, testing both MCP and A2A protocol patterns.

## Features

- **MCP Pattern Testing**: Test `tools/list` and `tools/call` endpoints
- **A2A Pattern Testing**: Test agent card fetching and message sending
- **Serverless Scaling**: Run tests on Modal's serverless infrastructure
- **Local Mode**: Run tests locally for development
- **Metrics Collection**: Latency percentiles, RPS, success rates

## Prerequisites

1. An agentgateway instance running (see main examples)
2. Modal account (for serverless execution)
3. Python 3.10+

## Quick Start

### Local Installation

```sh
# Install dependencies with uv
uv sync

# Or with pip
pip install -e .
```

### Local Testing

Run tests locally against a local agentgateway instance:

```sh
# Health check
uv run . --local --pattern health

# Test all patterns locally
uv run . --local --gateway-url http://localhost:3000 --num-requests 100

# Test only MCP patterns
uv run . --local --pattern mcp --num-requests 50 --concurrency 5

# Test only A2A patterns
uv run . --local --pattern a2a --num-requests 50

# With authentication
uv run . --local --auth-token "your-token" --pattern all
```

### Modal Deployment

1. Install Modal and authenticate:

```sh
pip install modal
modal setup
```

2. Create a Modal secret for authentication (optional):

```sh
modal secret create agentgateway-perf AGENTGATEWAY_AUTH_TOKEN=your-token
```

3. Run tests on Modal:

```sh
# Run all pattern tests on Modal
modal run src/modal_app.py --gateway-url https://your-gateway.example.com

# Run specific pattern
modal run src/modal_app.py --gateway-url https://your-gateway.example.com --pattern mcp

# With more requests
modal run src/modal_app.py --gateway-url https://your-gateway.example.com --num-requests 1000 --concurrency 50
```

## Test Patterns

### MCP Patterns

| Pattern | Description |
|---------|-------------|
| `mcp:tools/list` | List available MCP tools |
| `mcp:tools/call` | Call an MCP tool |
| `mcp:mixed` | Mixed list and call requests |

### A2A Patterns

| Pattern | Description |
|---------|-------------|
| `a2a:message` | Send a message to an agent |
| `a2a:agent_card` | Fetch the agent card |
| `a2a:conversation` | Multi-turn conversation |

## Output

Results include:

- **Total requests**: Number of requests sent
- **Successful/Failed**: Request counts by status
- **Latency metrics**: Average, P50, P95, P99
- **RPS**: Requests per second achieved
- **Errors**: First few error messages if any

Example output:

```
Gateway: http://localhost:3000

Pattern: mcp:tools/list
  Total requests: 100
  Successful: 100
  Failed: 0
  Avg latency: 12.34ms
  P50 latency: 11.20ms
  P95 latency: 18.50ms
  P99 latency: 25.00ms
  RPS: 450.23

Summary:
  Patterns tested: 3
  Total requests: 300
  Success rate: 100.0%
  Avg latency: 15.67ms
```

## Project Structure

```
examples/modal-perf/
├── pyproject.toml       # Project configuration
├── README.md            # This file
├── src/
│   ├── __init__.py
│   ├── __main__.py      # CLI entry point
│   ├── client.py        # Agentgateway client wrapper
│   ├── modal_app.py     # Modal function definitions
│   └── patterns/        # Pattern-specific test functions
│       ├── __init__.py
│       ├── mcp.py       # MCP patterns
│       └── a2a.py       # A2A patterns
└── tests/
    └── __init__.py
```

## API Usage

You can also use the patterns programmatically:

```python
import asyncio
from src.patterns.mcp import mcp_list_tools_pattern
from src.patterns.a2a import a2a_message_pattern

async def main():
    # Test MCP tools/list
    result = await mcp_list_tools_pattern(
        gateway_url="http://localhost:3000",
        num_requests=100,
        concurrency=10,
    )
    print(f"MCP tools/list: {result.avg_latency_ms:.2f}ms avg")

    # Test A2A messaging
    result = await a2a_message_pattern(
        gateway_url="http://localhost:3000",
        message="Hello, agent!",
        num_requests=50,
        concurrency=5,
    )
    print(f"A2A message: {result.avg_latency_ms:.2f}ms avg")

asyncio.run(main())
```

## Configuration

### Environment Variables

| Variable | Description |
|----------|-------------|
| `AGENTGATEWAY_AUTH_TOKEN` | Bearer token for authentication |

### Modal Secrets

Create a Modal secret named `agentgateway-perf` with your authentication credentials:

```sh
modal secret create agentgateway-perf \
  AGENTGATEWAY_AUTH_TOKEN=your-token
```
