---
type: reference
original_path: docs/design/fastmcp-transforms-comparison.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# FastMCP Transforms vs AgentGateway vMCP Comparison (imported)

Source: `docs/design/fastmcp-transforms-comparison.md`

**Status**: Analysis Document | **Date**: 2026-01-21

## Executive Summary
FastMCP 3.0 transforms and AgentGateway's vMCP serve **complementary rather than competing roles**.

| System | Where It Runs | Primary User | Scope |
|--------|---------------|--------------|-------|
| FastMCP Transforms | In MCP server process | Server authors | Single-server tool shaping |
| AgentGateway vMCP | Proxy/gateway layer | Agent authors, governance | Cross-server orchestration |

## Three Stakeholders / Three Deployment Patterns

1. **MCP Server Author** (FastMCP in-server): controls native tool implementation, server-side transforms. Changes require server redeployment.
2. **Agent Author** (FastMCP as sidecar proxy): can transform remote servers individually, uses Python `transform_fn` for output reshaping. No governance separation.
3. **Governance Body** (AgentGateway): controls tool visibility, compositions, output transforms, and policies WITHOUT modifying servers. Organizational policy enforcement.

## AgentGateway's Core Differentiators (no FastMCP equivalent)
- **Declarative compositions** (pipeline, scatter-gather) without imperative code
- **JSONPath-based output transforms** as configuration, not code
- **Cross-server data binding** (`$.steps.search.items[0]`)
- **Aggregation primitives** (flatten, dedupe, sort, limit)
- **Governance separation** from agent authors
- **Centralized policy enforcement** across all agents
- **SBOM/dependency tracking** and versioned tool registry

## The Governance Gap
The critical difference is organizational control:
- Sidecar: agent authors transform for their own convenience (same person controls agent + sidecar)
- Gateway: governance transforms for organizational control (different roles)
- Enterprise needs: "All agents MUST use `approved_search`, not raw `web_search`"

## Hybrid Architecture
Both can coexist: Gateway enforces what's allowed (governance), sidecar customizes within allowed bounds (agent convenience).

## Related Notes
- [[references/imported/virtual-tools|Virtual Tools]] — AgentGateway's virtual tool system
- [[references/imported/registry-v2|Registry v2]] — the registry that enables governance
- [[references/imported/mcp-algebra|MCP Algebra]] — the composition algebra
