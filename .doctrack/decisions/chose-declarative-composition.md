---
type: decision
status: accepted
date: 2026-01-01T00:00:00.000Z
related_features:
  - agentgateway-mcp
last_updated: 2026-04-12T00:00:00.000Z
tags:
  - doctrack/type/decision
  - doctrack/status/accepted
  - doctrack/audience/claude
---

# Decision: Declarative Composition over Imperative Glue

## Status
**Accepted**

## Context
Agent developers need to combine multiple MCP tools into workflows. Two approaches: (1) let agents orchestrate at runtime (burns LLM tokens, fragile), or (2) precompile compositions into the data plane (zero LLM involvement, deterministic).

## Decision
Adopt a declarative composition model inspired by Apache Camel's Enterprise Integration Patterns. Tool compositions are defined in JSON registry files and compiled to an IR at load time. The gateway executes them at runtime without any LLM involvement.

## Alternatives Considered

| Alternative | Pros | Cons | Why Rejected |
|---|---|---|---|
| Agent-side orchestration | Flexible | Burns tokens, non-deterministic, slow | Too expensive for known patterns |
| Code-based plugins | Full power | Requires Rust knowledge, recompilation | Not accessible to agent authors |
| Declarative JSON | Accessible, validated, hot-reloadable | Limited expressiveness | **Chosen** — expressiveness added via patterns |

## Consequences
- Agent authors define compositions as JSON (not code)
- LLMs can generate/edit virtual tool definitions (CodeAct paradigm)
- Hot-reloadable without gateway restart
- Pattern library grows over time (pipeline, scatter-gather, filter, etc.)

## Related
- [[concepts/virtual-tool-composition|Virtual Tool Composition]]
- [[references/imported/mcp-algebra|MCP Algebra]]
- [[references/imported/virtual-tools-requirements|Requirements Spec]]
