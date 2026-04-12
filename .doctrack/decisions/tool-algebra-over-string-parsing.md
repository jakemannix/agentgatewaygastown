---
type: decision
status: draft
date: 2026-04-12T00:00:00.000Z
related_features:
  - agentgateway-mcp
  - agentgateway-types
last_updated: 2026-04-12T00:00:00.000Z
tags:
  - doctrack/type/decision
  - doctrack/status/draft
  - doctrack/audience/claude
---

# Decision: Tool Algebra Over String Parsing

## Status
**Draft** — recognized as needed, not yet implemented

## Context
The current tool routing uses string manipulation to encode routing info in tool names (`{server}_{tool}`) via `resource_name()` and `parse_resource_name()`. This splits on `_`, breaks if tool names contain underscores, and uses a `virtual_` prefix as a special case.

## Decision
Replace string-based routing with a proper `(server, name)` tuple model where:
- Backend tools: server = the MCP backend name
- Virtual/composed tools: server = `gateway` or `local`
- Registry lookup is the primary resolution mechanism, not string parsing
- `tools/list` returns just the `name` — routing is internal

## Alternatives Considered

| Alternative | Pros | Cons | Why Rejected |
|---|---|---|---|
| Keep `_` delimiter | No code changes | Breaks on underscored tool names, fragile | Fundamental design flaw |
| Use `/` delimiter | Less ambiguous | Still string parsing, leaks into tool names | Same category of hack |
| ToolId struct | Clean, type-safe | Requires refactor | **Chosen** |

## Consequences
Eliminates: `virtual_` prefix convention, `DELIMITER` constant, `default_target_name` special case, string manipulation in `resource_name()` / `parse_resource_name()`.

## Related
- [[concepts/virtual-tool-composition|Virtual Tool Composition]]
- [[features/agentgateway-mcp|MCP handling]]
- See CLAUDE.md "Design TODO: Proper Tool Algebra"
