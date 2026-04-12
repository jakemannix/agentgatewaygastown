---
component: celx-integration
parent_feature: celx
type: component
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/cel/mod.rs
  - crates/agentgateway/src/telemetry/log.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# CEL Integration — How agentgateway Consumes celx

## Purpose

Documents the integration points between the `celx` crate and the main `agentgateway` crate. There are exactly two consumption points.

## Integration Points

### 1. Root Context Initialization (`cel/mod.rs`)

```rust
fn root_context() -> Arc<Context<'static>> {
    let mut ctx = Context::default();
    agent_celx::insert_all(&mut ctx);
    Arc::new(ctx)
}

static ROOT_CONTEXT: Lazy<Arc<Context<'static>>> = Lazy::new(root_context);
```

A single `Lazy<Arc<Context>>` is created at startup with all celx functions registered. This context is cloned (via `Arc`) for every CEL expression evaluation — authorization checks, header transforms, rate limiting key extraction, log field enrichment. The functions are registered once and shared across all threads.

### 2. FlattenSignal Handling (`telemetry/log.rs`)

```rust
match agent_celx::FlattenSignal::from_value(celv) {
    Some(FlattenSignal::List(li)) => { /* expand list items as log fields */ }
    Some(FlattenSignal::ListRecursive(li)) => { /* recursive expand */ }
    Some(FlattenSignal::Map(m)) => { /* expand map entries as log fields */ }
    Some(FlattenSignal::MapRecursive(m)) => { /* recursive expand */ }
    None => { /* normal value, serialize directly */ }
}
```

When a CEL expression in a logging config calls `flatten()` or `flattenRecursive()`, it returns a `FlattenSignal` opaque value instead of a normal value. The telemetry layer pattern-matches on this signal to expand maps/lists into individual structured log fields rather than serializing them as a single nested JSON value.

## Data Flow

```mermaid
graph LR
    STARTUP["Gateway startup"] -->|once| ROOT["root_context()"]
    ROOT -->|"insert_all()"| CELX["celx functions"]
    ROOT --> LAZY["Lazy&lt;Arc&lt;Context&gt;&gt;"]

    REQ["Incoming request"] --> EVAL["CEL evaluation"]
    EVAL -->|clone Arc| LAZY
    EVAL -->|"auth, headers, rate-limit"| RESULT["Value"]
    EVAL -->|"flatten() in log config"| SIGNAL["FlattenSignal"]
    SIGNAL --> LOG["telemetry/log.rs"]
    LOG --> FIELDS["Structured log fields"]

    class CELX,LAZY,SIGNAL internal-link;
```

## Related

- [[features/celx|celx Feature]] — the crate being consumed
- [[components/celx/helpers|Helpers Module]] — wrapper system that makes this integration clean
