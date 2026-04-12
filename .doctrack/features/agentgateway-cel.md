---
feature: agentgateway-cel
type: feature
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/cel/mod.rs
  - crates/agentgateway/src/cel/tests.rs
  - crates/agentgateway/src/cel/benches.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# CEL Context (agentgateway-internal)

## Purpose
Builds CEL evaluation contexts with request/response data for runtime policy evaluation. This is the glue between the [[features/celx|celx crate]] (CEL function library) and the gateway's runtime — it populates the evaluation context with variables like `jwt.sub`, `mcp.tool.name`, request headers, etc.

## Key Files
- `cel/mod.rs` — `ContextBuilder` — builds CEL evaluation context from request parts, initializes the global CEL function registry via `celx::insert_all`
- `cel/tests.rs` — Context building tests
- `cel/benches.rs` — Benchmark harness (compile, build, execute phases)

## Key Type
- `ContextBuilder` — Constructs a CEL context populated with: JWT claims, MCP info, HTTP headers, connection metadata. The context is then evaluated against CEL expressions in authorization policies, header modification, rate limiting keys, and log enrichment.

## Dependencies
- **Internal**: [[features/celx|celx]] (function library), [[features/agentgateway-http|HTTP]] (request/JWT types)
- **Used by**: [[features/agentgateway-proxy|Proxy]] (authorization), [[features/agentgateway-http|HTTP]] (header modification, rate limiting)
