---
feature: agentgateway-orchestration
type: feature
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/saga/mod.rs
  - crates/agentgateway/src/saga/types.rs
  - crates/agentgateway/src/saga/executor.rs
  - crates/agentgateway/src/workflow/mod.rs
  - crates/agentgateway/src/workflow/types.rs
  - crates/agentgateway/src/workflow/router.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# Orchestration (Saga + Workflow)

## Purpose
Multi-step orchestration patterns — saga pattern for compensating transactions and workflow routing for complex multi-tool workflows.

## Saga (`saga/`)
- `saga/mod.rs` — Module root
- `saga/types.rs` — Saga step definitions, compensation actions
- `saga/executor.rs` — Saga executor with forward execution and compensating rollback

## Workflow (`workflow/`)
- `workflow/mod.rs` — Module root
- `workflow/types.rs` — Workflow step types
- `workflow/router.rs` — Workflow routing logic

## Dependencies
- **Internal**: [[features/agentgateway-mcp|MCP]] (tool invocation)
- **Related**: Registry composition patterns (pipeline, scatter-gather) in MCP module

## Notes
- The saga and workflow modules complement the MCP registry's composition patterns
- Saga provides transaction semantics with compensating actions (rollback on failure)
- Some overlap with the registry executor's pipeline pattern — saga adds compensation
