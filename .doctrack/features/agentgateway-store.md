---
feature: agentgateway-store
type: feature
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/store/mod.rs
  - crates/agentgateway/src/store/binds.rs
  - crates/agentgateway/src/store/discovery.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# Store (Runtime State)

## Purpose
Runtime data store holding the current gateway configuration state — binds, listeners, routes, backends, policies. The `Stores` type is the central state object shared across the gateway. Updated by local config file watch or XDS.

## Key Files
- `store/mod.rs` — `Stores` type — the main state container, shared via `Arc`
- `store/binds.rs` — Bind configuration state management
- `store/discovery.rs` — Service discovery state

## Key Types
- `Stores` — Central state container holding all runtime configuration
- `BackendPolicies` — Per-backend policy configuration (auth, rate limiting, etc.)

## Dependencies
- **Internal**: [[features/agentgateway-types|Types]] (IR types), [[features/agentgateway-config|Config]] (parsing)
- **Used by**: [[features/agentgateway-proxy|Proxy]], [[features/agentgateway-mcp|MCP]]
