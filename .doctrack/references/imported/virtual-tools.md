---
type: reference
original_path: docs/virtual-tools.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Virtual Tools and Registry (imported)

Source: `docs/virtual-tools.md`

The virtual tools system provides tool aliasing, input/output transformation, and composition patterns.

## Capabilities
1. **Alias tools** — expose backend tools under different names
2. **Transform inputs** — inject defaults, hide fields, use templates
3. **Transform outputs** — restructure responses using JSONPath mappings
4. **Compose tools** — combine multiple tools into pipelines or parallel operations

## Architecture
- MCP Client (Agent) → AgentGateway (Registry with virtual tool definitions, transforms, compositions) → Backend MCP Servers (stdio/HTTP)

## Related Notes
- [[references/imported/mcp-algebra|MCP Algebra]] — the compositional algebra vision
- [[references/imported/virtual-tools-requirements|Virtual Tools Requirements]] — formal spec
- [[features/agentgateway-mcp|MCP handling]] — implementation
