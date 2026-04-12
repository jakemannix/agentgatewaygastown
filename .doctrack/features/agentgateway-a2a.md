---
feature: agentgateway-a2a
type: feature
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/a2a/mod.rs
  - crates/agentgateway/src/a2a/tests.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# A2A Handling

## Purpose
Handles Agent2Agent (A2A) protocol requests within the gateway — routing agent discovery and task requests to upstream A2A backends. Uses types from [[features/a2a-sdk|a2a-sdk crate]].

## Key Files
- `a2a/mod.rs` — A2A request handling, agent card resolution, task routing
- `a2a/tests.rs` — Unit tests

## Dependencies
- **Internal**: [[features/a2a-sdk|a2a-sdk]], [[features/agentgateway-proxy|Proxy]], [[features/agentgateway-http|HTTP]]
- **Used by**: [[features/agentgateway-proxy|Proxy]] (A2A route dispatch)
