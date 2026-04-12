---
type: decision
status: accepted
date: 2025-06-01T00:00:00.000Z
related_features:
  - agentgateway-mcp
last_updated: 2026-04-12T00:00:00.000Z
tags:
  - doctrack/type/decision
  - doctrack/status/accepted
  - doctrack/audience/claude
---

# Decision: OpenAPI-to-MCP Adapter for Legacy APIs

## Status
**Accepted**

## Context
Many existing services expose REST APIs via OpenAPI specs but don't have MCP servers. Agents need to call these as tools.

## Decision
The gateway includes an OpenAPI-to-MCP adapter (`upstream/openapi/`) that transforms OpenAPI endpoint definitions into MCP tool definitions. Agents see them as regular MCP tools; the gateway translates tool calls to HTTP requests.

## Consequences
- Legacy APIs usable without writing MCP servers
- OpenAPI specs used as the source of truth for tool schemas
- gRPC support planned but not yet implemented

## Related
- [[features/agentgateway-mcp|MCP handling]] — `upstream/openapi/` module
