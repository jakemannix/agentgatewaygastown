---
type: reference
original_path: docs/agent-identity.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Agent Identity and Dependency-Scoped Tool Discovery (imported)

Source: `docs/agent-identity.md`

## Overview
AgentGateway supports **dependency-scoped tool discovery**: agents only see tools they've declared as dependencies in their SBOM.

## Identity Mechanisms (priority order)
1. **HTTP headers** (highest): `X-Agent-Name` / `X-Agent-Version`
2. **MCP `clientInfo`** (session-based): extracted during `initialize` request
3. **JWT claims**: `agent_name`, `agent_version` if JWT auth configured

## SBOM Dependencies
Agents declare tool dependencies via the `urn:agentgateway:sbom` extension in the registry:
```json
{
  "agents": [{
    "name": "customer-agent",
    "capabilities": {
      "extensions": [{
        "uri": "urn:agentgateway:sbom",
        "params": {
          "depends": [
            { "type": "tool", "name": "find_products" },
            { "type": "tool", "name": "add_to_cart" }
          ]
        }
      }]
    }
  }]
}
```

## Unknown Caller Policies
- `allowAll` (default) — return all tools (backwards compatible)
- `denyAll` — return empty tool list for unknown callers
- `allowUnregistered` — registered agents get SBOM tools; unregistered get all

## Session-Based Identity
For SSE/WebSocket: identity established once during `initialize`, persists for session lifetime, cannot change mid-session.

## Related Notes
- [[references/imported/registry-v2|Registry v2]] — agent registration schema
- [[references/imported/virtual-tools|Virtual Tools]] — the tools being filtered
