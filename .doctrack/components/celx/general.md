---
component: celx-general
parent_feature: celx
type: component
doctrack_version: 3.0.0
files:
  - crates/celx/src/general.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# General Functions Module

## Purpose

Gateway-specific CEL functions that extend the base cel-rust engine. These are the functions unique to agentgateway (not from any external spec).

## Notable Functions

### FlattenSignal (`flatten` / `flattenRecursive`)

The most architecturally significant functions. Instead of returning a normal value, they return a `FlattenSignal` opaque type that acts as a **signal** to downstream consumers (specifically [[components/celx/cel-integration|telemetry/log.rs]]) to expand map/list contents into the parent structure.

```rust
pub enum FlattenSignal {
    Map(Map),           // Expand map entries as sibling fields
    MapRecursive(Map),  // Recursive expansion
    List(Arc<Vec<Value>>),
    ListRecursive(Arc<Vec<Value>>),
}
```

`FlattenSignal` is deliberately non-serializable (`Serialize` returns an error) and non-comparable (`PartialEq` always returns false) because it is transient — it exists only in the evaluation pipeline between expression evaluation and log field emission.

### `default(expr, fallback)`

Provides safe navigation for potentially missing fields. Catches `NoSuchKey` and `UndeclaredReference` errors from the inner expression, returning the fallback value instead:

```cel
default(request.headers["x-custom"], "none")
default(jwt.claims.role, "anonymous")
```

### `with(value, ident, expr)`

Creates a scoped binding — evaluates an expression with a value bound to an identifier:

```cel
[1,2].with(a, a + a)  // -> [1, 2, 1, 2]
```

### `variables()`

Introspects all variables in the current CEL context by walking the `Context` tree (root + child scopes). Returns a map of variable name to value. Useful for debugging CEL expressions.

### `mapValues(map, ident, expr)`

Transforms all values in a map while preserving keys:

```cel
{"a": 1, "b": 2}.mapValues(v, v * 2)  // -> {"a": 2, "b": 4}
```

### JSON / Base64 / Regex / UUID

Utility functions for common data transformations in policy expressions:

- `json(str)` / `toJson(value)` — parse and serialize JSON
- `base64Encode()` / `base64Decode()` — base64 codec
- `regexReplace(pattern, replacement)` — regex-based substitution
- `uuid()` — generate UUID v4
- `random()` — uniform random float in [0, 1]
- `fail(msg)` — abort with error message

## Related

- [[features/celx|celx Feature]] — parent feature
- [[components/celx/cel-integration|CEL Integration]] — `FlattenSignal` consumption
