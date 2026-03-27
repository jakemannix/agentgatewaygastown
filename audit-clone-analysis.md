# Value::clone() Audit: Executor Hot Path

## Overview

The DAG orchestrator's executor path (`crates/agentgateway/src/mcp/registry/executor/`)
performs extensive `serde_json::Value::clone()` operations on request payloads as they flow
through pipeline steps, scatter-gather fan-out, and schema-map transformations.

`serde_json::Value` is a recursive enum — cloning it deep-copies the entire JSON tree
(all nested objects, arrays, strings). For a middleware gateway handling structured payloads
of 10-100KB, this is the dominant allocation cost on the hot path.

**27 production clone sites** across 8 files. Most are unnecessary — the data is immutable
after creation and only needs to be *shared*, not *copied*.

## Fix Strategies

| Strategy | Description | Cost |
|----------|-------------|------|
| **MOVE** | Transfer ownership instead of copying | Zero-cost |
| **ARC** | `Arc<Value>` for cheap multi-reader sharing | ~1ns atomic increment |
| **REF** | Pass `&Value` where callee only reads | Zero-cost |
| **STRUCTURAL** | Unavoidable — data must exist in two places | Keep clone |

## Clone-by-Clone Analysis

### mod.rs — CompositionExecutor

| Line | Code | Strategy | Rationale |
|------|------|----------|-----------|
| 179 | `input.clone()` into `ExecutionContext::new_with_tracing` | **ARC** | `ctx.input` is read-only; `Arc<Value>` eliminates the copy |
| 184 | `input.clone()` into `ExecutionContext::new` | **ARC** | Same — both branches create ctx then pass `input` to `execute_pattern` |

**Fix**: Change `ExecutionContext.input` from `Value` to `Arc<Value>`. Wrap `input` in
`Arc::new()` once at entry, then `Arc::clone()` for ctx and pass `&input` to `execute_pattern`.

### pipeline.rs — PipelineExecutor

| Line | Code | Strategy | Rationale |
|------|------|----------|-----------|
| 24 | `let mut current_result = input.clone()` | **ARC** | Input is also used by-ref in `resolve_binding`; share via Arc |
| 32 | `current_result.clone()` (default step input) | **ARC** | Pipeline pass-through: value consumed by step but retained for next |
| 59 | `step_input.clone()` (child context for Pattern) | **ARC** | Dual-use: seed child ctx + pass to `execute_pattern` |
| 99 | `result.clone()` (store step result) | **ARC** | Most impactful clone — every step result deep-copied for storage |
| 121 | `value.clone()` in `DataBinding::Constant` | **ARC** | Constants are spec-defined, reusable; store as `Arc<Value>` in spec |
| 138 | `value.clone()` in `apply_jsonpath("$", ...)` | **REF/Cow** | Root-path fast-path; return `Cow::Borrowed(value)` |
| 145 | `(*v).clone()` in JSONPath query results | **STRUCTURAL** | `serde_json_path` returns refs; must clone to take ownership |

**Fix**: Change `step_results` to `HashMap<String, Arc<Value>>`. Accept/return `Arc<Value>`
throughout. Pipeline hot loop becomes zero-copy except at JSONPath extraction boundaries.

### scatter_gather.rs — ScatterGatherExecutor

| Line | Code | Strategy | Rationale |
|------|------|----------|-----------|
| 32 | `input.clone()` per scatter target (N times) | **ARC** | Classic broadcast pattern — textbook `Arc<Value>` use case |
| 130 | `input.clone()` child context for Pattern target | **ARC** | Same dual-use as pipeline |
| 138 | `values.clone()` in `aggregate()` init | **MOVE** | Bug fix: move `values` into `Value::Array()`, don't clone |
| 179 | `(*v).clone()` in `extract()` | **STRUCTURAL** | JSONPath extraction, unavoidable |
| 201-206 | `item.clone()` / `.cloned()` in `flatten`, `dedupe` | **MOVE** | Consume input by ownership, iterate with `into_iter()` |
| 253-257 | `item.clone()` in `dedupe` keeping items | **MOVE** | Same — consume the source array |
| 270 | `.cloned()` in `limit` | **MOVE** | Same |

**Fix**: Change aggregate ops to consume `Value` by ownership instead of borrowing `&Value`.
This eliminates all per-item clones in flatten/dedupe/sort/limit. The `values.clone()` on
line 138 is also a **bug** (P0-5): the `Merge` op reads the original `values` vec instead
of the current `result`, producing stale output when Merge isn't the first op.

### schema_map.rs — SchemaMapExecutor

| Line | Code | Strategy | Rationale |
|------|------|----------|-----------|
| 39 | `input.clone()` in nested SchemaMap | **REF** | Make `execute()` take `&Value` (it never mutates) |
| 71 | `input.clone()` in ArrayMap | **REF** | Same |
| 84 | `value.clone()` root path | **REF/Cow** | Return borrowed value for root path |
| 91 | `(*v).clone()` JSONPath results | **STRUCTURAL** | Unavoidable with serde_json_path |

**Fix**: Make `SchemaMapExecutor::execute()` synchronous and take `&Value`. This:
1. Eliminates the `now_or_never()` hack (correctness landmine — panics if any `.await` added)
2. Removes 2 clones by passing references
3. Simplifies the code significantly

### map_each.rs — MapEachExecutor

| Line | Code | Strategy | Rationale |
|------|------|----------|-----------|
| 28 | `item.clone()` per array element | **MOVE** | Consume input array via `into_iter()` |
| 45 | `item.clone()` child context for Pattern | **ARC** | Dual-use: child ctx + pattern arg |

**Fix**: Destructure `input` into owned `Vec<Value>`, iterate with `into_iter()`.
For the Pattern case, wrap `item` in `Arc` since it needs to exist in two places.

### timeout.rs — TimeoutExecutor

| Line | Code | Strategy | Rationale |
|------|------|----------|-----------|
| 35 | `input.clone()` primary operation | **ARC** | Input may be used twice (primary + fallback) |
| 63 | `input.clone()` child context for Pattern | **ARC** | Same dual-use pattern |

**Fix**: Wrap in `Arc<Value>` at entry. Primary op gets `Arc::clone`, fallback gets the
last reference via move.

### filter.rs — FilterExecutor

| Line | Code | Strategy | Rationale |
|------|------|----------|-----------|
| 30 | `item.clone()` for items passing predicate | **MOVE** | Consume input array, move matching items |

**Fix**: Change `execute()` to consume `input: Value`, destructure into `Vec<Value>`,
iterate with `into_iter()`, evaluate predicate on `&item`, then `push(item)` (move).

### handler.rs — RelayToolInvoker

| Line | Code | Strategy | Rationale |
|------|------|----------|-----------|
| 461 | `args.clone()` before `resolve_tool_call` | **Investigate** | Likely MOVE if `resolve_tool_call` consumes args |

### context.rs — ExecutionContext (implicit clones)

| Line | Code | Strategy | Rationale |
|------|------|----------|-----------|
| 107 | `get_step_result` returns `.cloned()` | **ARC** | Store as `Arc<Value>`, return `Arc::clone` |

## Refactoring Roadmap

### Phase 1: `Arc<Value>` for shared state (~12 clones eliminated)

The foundational change. Touch 3 files, unlock all downstream improvements.

1. `ExecutionContext.input: Arc<Value>` (context.rs)
2. `step_results: HashMap<String, Arc<Value>>` (context.rs)
3. Scatter-gather input broadcast via `Arc` (scatter_gather.rs)
4. Timeout input via `Arc` (timeout.rs)

### Phase 2: Ownership transfer / consume-input (~8 clones eliminated)

Change aggregation and filter/map ops to consume their input instead of borrowing.

1. `FilterExecutor::execute(input: Value)` — destructure, move items (filter.rs)
2. `MapEachExecutor::execute(input: Value)` — destructure, move items (map_each.rs)
3. `aggregate(values: Vec<Value>)` — move into `Value::Array`, fix Merge bug (scatter_gather.rs)
4. Aggregation ops consume `Value` instead of `&Value` (scatter_gather.rs)

### Phase 3: SchemaMap sync + `&Value` (~4 clones eliminated + correctness fix)

1. Make `SchemaMapExecutor::execute` synchronous, take `&Value` (schema_map.rs)
2. Remove `NowOrNever` trait and `now_or_never()` hack
3. Remove `futures::task::noop_waker()` usage

### Phase 4: `Cow<'_, Value>` for JSONPath returns (~2 clones eliminated)

1. `apply_jsonpath` returns `Cow<'a, Value>` — root path returns borrowed
2. Remaining JSONPath extraction clones are structural/unavoidable

## Estimated Impact

For a typical gateway request (4-target scatter-gather, 50KB payloads, 3-step pipeline):

| Metric | Before | After Phase 1+2 | Reduction |
|--------|--------|------------------|-----------|
| Deep copies per request | ~15 | ~3 | 80% |
| Bytes allocated | ~750KB | ~150KB | 80% |
| CPU time (clone overhead) | ~50μs | ~10μs | 80% |

At 1000 req/s, Phase 1 alone saves ~40ms/s of CPU time and ~600MB/s of allocation churn.

## Key Insight

Almost every `Value` in this system is **write-once, read-many**:
- Composition input: written once, read by every step
- Step results: written once, read by dependent steps
- Scatter input: written once, broadcast to N parallel tasks
- Constants: defined in spec, never change

This is the exact usage pattern `Arc<T>` was designed for. The current code treats every
value as if it needs independent mutation, but none of them are ever mutated after creation.

The `serde_json::Value` type is not `Copy` (it heap-allocates strings and collections),
so Rust correctly requires explicit `.clone()`. But the right response is `Arc`, not `clone` —
sharing the same allocation instead of duplicating it.
