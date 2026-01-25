# Virtual Tools & Compositions: Vision and Status

## Overview

AgentGateway's virtual tools system enables tool abstraction, composition, and governance at the gateway layer. This document describes the current state and future direction.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     TypeScript DSL                               │
│   (packages/vmcp-dsl)                                           │
│                                                                  │
│   tool("personalized_search")                                   │
│     .description("...")                                          │
│     .pipeline(p => p.step("search", ...).step("hydrate", ...))  │
│     .build()                                                     │
└────────────────────────────┬────────────────────────────────────┘
                             │ compiles to
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Proto3 JSON Registry                         │
│   (registry.proto → pbjson → serde)                             │
│                                                                  │
│   {                                                              │
│     "schemaVersion": "2.0",                                     │
│     "tools": [{ "name": "personalized_search", "spec": {...} }] │
│   }                                                              │
└────────────────────────────┬────────────────────────────────────┘
                             │ loaded by
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Rust Runtime                                 │
│   (crates/agentgateway/src/mcp/registry/)                       │
│                                                                  │
│   CompiledRegistry → CompositionExecutor → Tool Invocation      │
└─────────────────────────────────────────────────────────────────┘
```

## What's Working

### Virtual Tools (1:1 Mapping)

Transform a backend tool into a virtual tool exposed to agents:

| Feature | Status | Description |
|---------|--------|-------------|
| **Renaming** | ✅ Working | `source.tool` maps backend tool to new name |
| **Default injection** | ✅ Working | `source.defaults` injects values into calls |
| **Field hiding** | ✅ Working | `source.hideFields` removes fields from schema |
| **Output transformation** | ✅ Working | `outputTransform.mappings` reshapes responses |
| **Schema references** | ✅ Working | `inputSchema.$ref` / `outputSchema.$ref` |

### Compositions (N:1 Mapping)

Orchestrate multiple tools into a single virtual tool:

| Pattern | Status | Description |
|---------|--------|-------------|
| **Pipeline** | ✅ Working | Sequential steps with data binding between them |
| **Scatter-Gather** | ✅ Working | Parallel execution with result aggregation |
| **Filter** | ✅ Working | Filter array results with CEL predicates |
| **SchemaMap** | ✅ Working | Map fields from input to tool arguments |
| **MapEach** | ✅ Working | Apply operation to each array element |

### Data Binding

Compositions support flexible data binding:

| Binding Type | Status | Description |
|--------------|--------|-------------|
| **input** | ✅ Working | `{ "input": { "path": "$.query" } }` - from composition input |
| **step** | ✅ Working | `{ "step": { "stepId": "search", "path": "$.matches" } }` - from previous step |
| **constant** | ✅ Working | `{ "constant": {"limit": 10} }` - static value |
| **construct** | ✅ Working | `{ "construct": { "fields": {...} } }` - build object from multiple sources |

### Registry Features

| Feature | Status | Description |
|---------|--------|-------------|
| **Schema definitions** | ✅ Working | Reusable `$ref` schemas in `schemas[]` |
| **Server registration** | ✅ Working | Declare tool providers in `servers[]` |
| **Agent registration** | ✅ Working | Agent metadata with `depends` for tool filtering |
| **Caller identity** | ✅ Working | `X-Agent-Name` header or MCP `clientInfo.name` |
| **Dependency-scoped discovery** | ✅ Working | `tools/list` filtered by agent's dependencies |

### Tooling

| Component | Status | Description |
|-----------|--------|-------------|
| **TypeScript DSL** | ✅ Working | Fluent builder API for compositions |
| **Proto codegen** | ✅ Working | `registry.proto` → Rust + TypeScript types |
| **Visual builder** | ✅ Working | `tool-builder/index.html` for UI composition |
| **vmcp-compile CLI** | ✅ Working | `npx vmcp-compile tools.ts -o registry.json` |

## What's Not Implemented

### Stateful Patterns

These patterns are designed but not yet implemented in the executor:

| Pattern | Status | Description |
|---------|--------|-------------|
| **Saga** | ❌ Not implemented | Distributed transaction with compensation |
| **Retry** | ❌ Not implemented | Automatic retry with backoff |
| **Timeout** | ❌ Not implemented | Time-bounded execution |
| **Cache** | ❌ Not implemented | Result caching |
| **Circuit Breaker** | ❌ Not implemented | Fail-fast on repeated failures |

The proto definitions exist, TypeScript DSL supports them, but the Rust executor returns `NotImplemented` errors.

### Parallel DAG Execution

Currently, pipeline steps execute sequentially even when independent. A design exists (`dag-executor.md`) for wave-based parallel execution but is not implemented.

### Agent-as-Tool (Phase 2)

Invoking A2A agents as steps in compositions:

```json
{
  "operation": {
    "agent": { "name": "summarizer", "skill": "summarize" }
  }
}
```

This requires A2A multiplexing and skill-based routing, which are Phase 2 features.

## Registry Format

The canonical format is proto3 JSON. Key differences from legacy v1:

| Field | v1 (Legacy) | v2 (Proto) |
|-------|-------------|------------|
| Server reference | `source.target` | `source.server` |
| Step ID | `step_id` | `stepId` |
| Schema version | `schemaVersion: "1.0"` | `schemaVersion: "2.0"` |

The Rust parser supports both formats via fallback parsing for backward compatibility.

## Demo: eCommerce

The `examples/ecommerce-demo/` demonstrates the full system:

- **5 MCP backend services**: catalog, cart, order, inventory, supplier
- **Registry with compositions**: `personalized_search` (3-step pipeline), `product_with_availability`, etc.
- **2 ADK agents**: customer-agent, merchandiser-agent
- **Tool filtering**: Each agent sees only its declared dependencies

## Related Documents

| Document | Description |
|----------|-------------|
| [code-walkthrough.md](./code-walkthrough.md) | Where the code lives |
| [quickstart.md](./quickstart.md) | How to build and run |
| [proto-codegen-migration.md](./proto-codegen-migration.md) | Proto codegen implementation details |
| [registry-v2.md](./registry-v2.md) | Full registry v2 specification |
