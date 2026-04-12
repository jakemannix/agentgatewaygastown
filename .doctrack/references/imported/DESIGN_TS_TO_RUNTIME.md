---
type: reference
original_path: docs/DESIGN_TS_TO_RUNTIME.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Design: TypeScript DSL to JSON IR to Runtime (imported)

Source: `docs/DESIGN_TS_TO_RUNTIME.md`

Concrete design for the three-layer tool algebra composition system.

## The Three Layers

### Layer 1: TypeScript DSL (`@vmcp/dsl`)
What developers write. Package includes:
- Core type system: `ToolRef<I,O>`, `Composition`, `Pattern`, `Step`
- Pattern implementations: Pipeline, Router, ScatterGather, Filter, SchemaMap, MapEach
- Type-safe path builders for JSONPath field references
- `toIR()` method on every pattern for serialization

### Layer 2: JSON IR
Portable, versionable, schematized intermediate representation. Defined by `registry.proto` (proto3 JSON mapping). Each pattern has a spec type (PipelineSpec, ScatterGatherSpec, etc.) with data binding for input/step/constant/construct sources.

### Layer 3: Rust Runtime
Fast, safe execution in the gateway. `CompositionExecutor` dispatches to pattern-specific executors. Data binding resolution via JSONPath.

## Key Patterns Designed

**Pipeline**: Sequential steps with data binding between them. Each step references a tool or nested composition. Input binding types: `input` (from composition input), `step` (from previous step output), `constant` (literal value), `construct` (build object from multiple bindings).

**Router**: Conditional dispatch based on CEL predicates. Routes evaluated in order, optional default.

**ScatterGather**: Parallel execution across multiple targets with aggregation. Aggregation ops: flatten, sort, dedupe, limit, concat, merge. Configurable timeout.

**Filter**: Array filtering with field-based predicates (gt, lt, eq, contains, etc.).

**SchemaMap**: Declarative field mapping. Sources: path (JSONPath), coalesce (first non-null from multiple paths), template (string interpolation), literal (constant value), arrayMap (per-element array transformation).

**MapEach**: Apply an inner pattern to each element of an array.

## Related Notes
- [[references/imported/DESIGN_STATEFUL_PATTERNS|Stateful Patterns]] — extends this with cache, retry, circuit breaker
- [[references/imported/ARCHITECTURE_DISCONNECTS|Architecture Disconnects]] — the gaps this design addresses
- [[references/imported/proto-codegen-migration|Proto Codegen Migration]] — how the IR was formalized into proto
