---
type: interface
implementors:
  - agentgateway-mcp
consumers:
  - agentgateway-proxy
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/interface
  - doctrack/status/active
  - doctrack/audience/claude
---

# Interface: MCP Transport Contract

## Contract
The gateway supports two MCP transport modes for downstream clients and upstream backends:

| Transport | Downstream (client→gateway) | Upstream (gateway→backend) |
|---|---|---|
| SSE (legacy) | `sse.rs` / `LegacySSEService` | `upstream/sse.rs` |
| StreamableHTTP | `streamablehttp.rs` / `StreamableHttpService` | `upstream/streamablehttp.rs` |
| Stdio | N/A | `upstream/stdio.rs` (spawn child process) |

## Session Lifecycle
1. Client sends `InitializeRequest` (no session header)
2. Gateway creates session, extracts `CallerIdentity` from `clientInfo`
3. Gateway returns `mcp-session-id` header
4. Client includes session header in subsequent requests
5. Stateless mode: gateway wraps each request in `InitializeRequest` for backends that require it

## Implementors
- [[features/agentgateway-mcp|MCP handling]] — session management, transport handlers

## Consumers
- [[features/agentgateway-proxy|Proxy]] — routes MCP requests to the MCP handler
