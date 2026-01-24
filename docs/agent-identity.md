# Agent Identity and Dependency-Scoped Tool Discovery

This document describes how agents identify themselves to AgentGateway and how the gateway uses that identity to filter tools based on declared dependencies.

## Overview

AgentGateway supports **dependency-scoped tool discovery**: agents only see the tools they've declared as dependencies in their SBOM (Software Bill of Materials). This enables:

- **Least-privilege access**: Agents can't discover or call tools they haven't declared
- **Clear dependency graphs**: The registry documents which agents use which tools
- **Governance**: Operators can audit and control agent capabilities

## How Agents Identify Themselves

Agents identify themselves through one of two mechanisms:

### 1. MCP `clientInfo` (Recommended for Python/SDK clients)

When an MCP client connects, it sends an `initialize` request containing `clientInfo` with `name` and `version` fields. AgentGateway extracts the identity from this standard MCP field and associates it with the session.

**Python example using the `mcp` SDK:**

```python
from mcp import ClientSession
from mcp.client.sse import sse_client

async def connect_as_agent():
    async with sse_client("http://gateway:15000/mcp/sse") as (read, write):
        async with ClientSession(read, write) as session:
            # The client_info is sent during initialize
            await session.initialize(
                client_info={
                    "name": "customer-agent",      # This is the agent identity!
                    "version": "1.0.0"
                }
            )

            # Now tools/list will only return tools that customer-agent depends on
            tools = await session.list_tools()
```

**How it works:**
1. Client sends `initialize` with `clientInfo.name = "customer-agent"`
2. Gateway extracts identity and stores it in session state
3. All subsequent requests (like `tools/list`) use this session identity
4. Gateway looks up `customer-agent` in the registry
5. If the agent has SBOM dependencies declared, only those tools are returned

### 2. HTTP Headers (For clients with transport control)

If you have control over the HTTP transport layer, you can set headers directly:

```
X-Agent-Name: customer-agent
X-Agent-Version: 1.0.0
```

This is useful for:
- Custom HTTP clients
- Testing/debugging with curl
- Proxies that inject identity

**Example with curl:**

```bash
curl -H "X-Agent-Name: customer-agent" \
     -H "X-Agent-Version: 1.0.0" \
     http://gateway:15000/mcp/sse
```

### Priority Order

When multiple identity sources are present, they are used in this order:
1. HTTP headers (highest priority - explicit override)
2. MCP `clientInfo` (session-based identity)
3. JWT claims (`agent_name`, `agent_version`) if JWT auth is configured

## Registry Configuration

### Declaring Agent Dependencies

Agents are declared in the registry with their tool dependencies via the SBOM extension:

```json
{
  "schemaVersion": "2.0",
  "agents": [
    {
      "name": "customer-agent",
      "version": "1.0.0",
      "description": "Handles customer shopping interactions",
      "capabilities": {
        "extensions": [
          {
            "uri": "urn:agentgateway:sbom",
            "params": {
              "depends": [
                { "type": "tool", "name": "find_products" },
                { "type": "tool", "name": "add_to_cart" },
                { "type": "tool", "name": "checkout" }
              ]
            }
          }
        ]
      }
    }
  ],
  "tools": [
    // ... tool definitions
  ]
}
```

### Unknown Caller Policy

The registry can configure how to handle requests without agent identity:

```json
{
  "schemaVersion": "2.0",
  "unknownCallerPolicy": "allowAll",
  "agents": [...],
  "tools": [...]
}
```

**Policy options:**

| Policy | Behavior |
|--------|----------|
| `allowAll` (default) | Return all tools - backwards compatible |
| `denyAll` | Return empty tool list for unknown callers |
| `allowUnregistered` | Registered agents get their SBOM tools; unregistered agents get all tools |

## Best Practices

### Agent Naming Conventions

Use consistent, descriptive names that match your registry:

```python
# Good - matches registry entry
client_info={"name": "customer-agent", "version": "1.0.0"}

# Bad - generic name won't match registry
client_info={"name": "mcp-client", "version": "0.1.0"}
```

### Version Management

Include version in agent identity for:
- Debugging and tracing
- Future version-specific policies
- Audit logging

### Development vs Production

During development, use `unknownCallerPolicy: allowAll` to see all tools.

For production, consider:
- `denyAll` for strict enforcement
- `allowUnregistered` if you want registered agents scoped but allow debugging

## Troubleshooting

### Agent sees all tools instead of filtered list

1. Check that `clientInfo.name` matches the agent name in the registry exactly
2. Verify the agent has SBOM dependencies declared
3. Check gateway logs for identity extraction messages

### Agent sees no tools

1. Verify `unknownCallerPolicy` is not `denyAll`
2. Check that the agent is registered in the registry
3. Ensure SBOM dependencies list the correct tool names

### Testing identity with curl

```bash
# List tools as a specific agent
curl -X POST http://gateway:15000/mcp \
  -H "Content-Type: application/json" \
  -H "X-Agent-Name: customer-agent" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

## Architecture Notes

### Session-Based Identity

For SSE/WebSocket connections, identity is established once during `initialize` and persists for the session lifetime. This means:

- Identity cannot change mid-session
- All requests in a session share the same tool visibility
- Session termination clears identity state

### Stateless Requests

For one-shot HTTP requests (non-streaming), use HTTP headers since there's no session to store identity.
