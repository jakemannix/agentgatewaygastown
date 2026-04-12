---
type: reference
original_path: docs/ALIGNMENT_WITH_AGENTGATEWAY.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# vMCP ↔ Agentgateway Alignment Analysis (imported)

Source: `docs/ALIGNMENT_WITH_AGENTGATEWAY.md`

**Alignment Score: 8.5/10**

## Core Finding
Agentgateway provides a production-grade implementation of the **Tool Adapter** pattern from the vMCP algebra. It operates at the data plane level (runtime mechanics of tool virtualization) while vMCP provides the declarative patterns for composing those tools.

## Layered Architecture

```
vMCP Composition Algebra (conceptual framework)
    ↓
Agentgateway Virtual Tools & Registry (production implementation)
    ↓
Primitive MCP Servers & Backends
```

## What's Fully Aligned: Tool Adapter Pattern
- Rename tool, override description, transform input schema
- Inject defaults, hide fields, transform output via JSONPath
- Hot-reloadable registry (file + HTTP), auth support, metadata

## Partially Aligned: Higher-Order Patterns
Agentgateway was a data plane proxy handling one request/response at a time. Composition patterns (pipeline, scatter-gather, enricher, etc.) require control plane orchestration logic. These have since been implemented.

## Complementary Strengths
- **vMCP**: algebra of composition — what patterns exist, how they compose, type signatures
- **Agentgateway**: foundation layer — virtual tool adaptation, hot-reload config, governance, Rust performance

## Recommended Path (4 phases)
1. Registry Enhancement (complete)
2. Composition Definitions in registry (complete)
3. Composition Executor (complete)
4. Control Plane Integration via XDS (future)

## Related Notes
- [[references/imported/mcp-algebra|MCP Algebra]] — the compositional algebra
- [[references/imported/virtual-tools|Virtual Tools]] — implementation
- [[references/imported/ARCHITECTURE_DISCONNECTS|Architecture Disconnects]] — gaps analysis
