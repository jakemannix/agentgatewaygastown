---
type: reference
original_path: docs/design/virtual-tools-requirements.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Virtual Tools Requirements Specification (imported)

Source: `docs/design/virtual-tools-requirements.md`

**Status**: Draft v0.1 | **Author**: Jake Mannix | **Date**: 2026-03-21

## Purpose
Software is increasingly programmed in natural language. MCP introduced N+M factorization (tool authors + agent authors). Virtual tools preserve this while letting agent authors customize tool appearance via configuration — "natural language code."

## Three Customization Axes
1. **Naming** — tool name, description, field names use agent's domain language
2. **Simplification** — input schemas projected to needed fields, defaults injected for deterministic fields
3. **Composition** — multi-tool workflows precompiled into single tool call, executed by data plane with zero LLM involvement

## Personas
- **Agent Author** — builds AI agents, may not be programmer, wants domain-matched tools
- **Tool Author** — builds MCP servers, publishes native schemas
- **Platform Operator** — manages registry, enforces governance
- **Governance Team** — audits tool usage, manages lifecycle

## Key Insight
LLMs can also generate virtual tools (offline with agent author, or online via CodeAct paradigm) — validation + constrained DSL enables this.

## Related Notes
- [[references/imported/virtual-tools|Virtual Tools]] — implementation
- [[references/imported/mcp-algebra|MCP Algebra]] — compositional vision
