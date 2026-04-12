---
feature: agentgateway-stateful
type: feature
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/stateful/mod.rs
  - crates/agentgateway/src/stateful/store.rs
  - crates/agentgateway/src/stateful/memory.rs
  - crates/agentgateway/src/stateful/cache.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# Stateful Sessions

## Purpose
Manages stateful session data for MCP connections — session persistence, in-memory state stores, and caching. Enables the gateway to maintain state across multiple requests within an MCP session.

## Key Files
- `stateful/mod.rs` — Module root
- `stateful/store.rs` — Session state storage abstraction
- `stateful/memory.rs` — In-memory session storage implementation
- `stateful/cache.rs` — Cache subsystem for session data

## Dependencies
- **Used by**: [[features/agentgateway-mcp|MCP]] (session management), [[features/agentgateway-http|HTTP]] (session persistence)
