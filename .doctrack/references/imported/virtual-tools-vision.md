---
type: reference
original_path: docs/design/virtual-tools-vision.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Virtual Tools & Compositions: Vision and Status (imported)

Source: `docs/design/virtual-tools-vision.md`

## Architecture
```
TypeScript DSL (packages/vmcp-dsl) -> compiles to -> Proto3 JSON Registry (registry.proto) -> loaded by -> Rust Runtime (crates/agentgateway/src/mcp/registry/)
```

## What's Working

### Virtual Tools (1:1 Mapping)
- Renaming, default injection, field hiding, output transformation, schema references — all working

### Compositions (N:1 Mapping)
- Pipeline, Scatter-Gather, Filter, SchemaMap, MapEach — all working

### Data Binding
- input (from composition input), step (from previous step), constant (literal), construct (build object) — all working

### Registry Features
- Schema definitions ($ref), server registration, agent registration, caller identity (X-Agent-Name / clientInfo.name), dependency-scoped discovery — all working

### Tooling
- TypeScript DSL, proto codegen, visual builder, vmcp-compile CLI — all working

## What's NOT Implemented

### Schema Type Safety in DSL
The TypeScript DSL has no type-level awareness of output schemas. Tool references are just strings — the compiler cannot detect schema mismatches at build time. Workaround: `scripts/validate-registry-schemas.py` as post-build lint.

### Stateful Patterns
Saga, Retry, Timeout, Cache, Circuit Breaker — proto definitions exist, TypeScript DSL supports them, but Rust executor returns `NotImplemented`.

### Parallel DAG Execution
Pipeline steps execute sequentially even when independent. Design exists (`dag-executor.md`) but not implemented.

### Agent-as-Tool (Phase 2)
Invoking A2A agents as steps in compositions. Requires A2A multiplexing and skill-based routing.

## Registry Format Notes
Canonical format is proto3 JSON. v1 `source.target` maps to v2 `source.server`. Rust parser supports both via fallback.

## Related Notes
- [[references/imported/code-walkthrough|Code Walkthrough]] — where the code lives
- [[references/imported/quickstart|Quickstart]] — how to build and run
- [[references/imported/DESIGN_STATEFUL_PATTERNS|Stateful Patterns]] — the unimplemented patterns
- [[references/imported/registry-v2|Registry v2]] — full registry specification
