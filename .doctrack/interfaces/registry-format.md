---
type: interface
implementors:
  - agentgateway-mcp
consumers:
  - agentgateway-mcp
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/interface
  - doctrack/status/active
  - doctrack/audience/claude
---

# Interface: Registry JSON Format

## Contract
The registry is a JSON file defining virtual tools, compositions, schemas, servers, and agent definitions. It is the primary configuration surface for the virtual tools system.

## Top-Level Structure
```json
{
  "schemas": { "SchemaName": { "type": "object", ... } },
  "servers": { "server-name": { "url": "...", "description": "..." } },
  "tools": {
    "tool_name": {
      "description": "...",
      "source": { "server": "...", "tool": "..." },
      "inputSchema": { ... },
      "inputTransform": { "defaults": {}, "hide": [], "rename": {} },
      "outputTransform": { "fields": [...] },
      "outputSchema": { "$ref": "#/schemas/..." },
      "composition": { "pattern": "pipeline|scatter_gather|filter|..." }
    }
  },
  "agents": {
    "agent-name": { "tools": ["tool1", "tool2"], "unknownCallerPolicy": "allow|deny" }
  }
}
```

## Key Sections
- **schemas** — Reusable JSON Schema definitions, referenced via `$ref`
- **servers** — Backend MCP server metadata
- **tools** — Virtual tool definitions with source, transforms, compositions
- **agents** — Agent-to-tool mappings with caller identity policies

## Implementors
- [[features/agentgateway-mcp|MCP handling]] — `registry/types.rs` defines the Rust types, `registry/client.rs` loads from file/HTTP

## Consumers
- Gateway configs in `examples/*/gateway-configs/*.json`
- [[references/imported/virtual-tools-requirements|Requirements spec]]
- `jakemannix/virtual-tools-spec` repo (external)

## Related
- [[concepts/virtual-tool-composition|Virtual Tool Composition]]
- [[decisions/chose-declarative-composition|Why declarative composition]]
