---
component: celx-helpers
parent_feature: celx
type: component
doctrack_version: 3.0.0
files:
  - crates/celx/src/lib.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Helpers Module — Type-Safe CEL Function Wrappers

## Purpose

The `helpers` module (defined inline in `lib.rs`) provides a type-safe bridge between Rust functions and CEL's dynamic dispatch system. It eliminates boilerplate for registering custom opaque types and wrapping Rust functions as CEL-callable functions.

## Key Types

```rust
pub type FResult<T> = Result<T, ExecutionError>;
pub type FVResult = Result<Value, ExecutionError>;
pub type Function = Box<dyn Fn(&mut FunctionContext) -> ResolveResult + Send + Sync>;
```

## Wrapper Functions

Each wrapper handles extracting `this` (for method-style calls) and arguments from CEL's dynamic `Value` type, downcasting to concrete Rust types, and converting results back:

| Wrapper | Signature Pattern | Use Case |
|---------|-------------------|----------|
| `wrap1(f)` | `T -> V` | Method with no args (e.g., `cidr.ip()`, `ip.family()`) |
| `wrap2(f, a1)` | `(T, A1) -> FResult<V>` | Method with one typed arg (e.g., `cidr.containsCIDR(other)`) |
| `wrap2_val(f, ctx, val)` | `(T, &FunctionContext, Value) -> FResult<V>` | Method with one dynamic arg needing context (e.g., `cidr.containsIP(ip_or_string)`) |
| `wrapnew(f)` | `(&FunctionContext, &str) -> FResult<V>` | Constructor from string (e.g., `cidr("10.0.0.0/8")`, `ip("1.2.3.4")`) |
| `split_this(method, free)` | dispatch on `this` | Overloaded: different behavior as method vs free function (e.g., `ip()`) |

## impl_opaque! Macro

Registers a Rust type as a CEL opaque value:

```rust
impl_opaque!(Cidr, "cidr");
impl_opaque!(IP, "ip");
impl_opaque!(FlattenSignal, "flatten_signal");
```

This implements both `TypeName` (for error messages) and `cel::objects::Opaque` (for runtime type info + JSON serialization).

## cast() Helper

Safe downcast from `Arc<dyn Opaque>` to a concrete type with descriptive error on mismatch:

```rust
pub fn cast<T: Opaque + TypeName>(val: &Arc<dyn Opaque>) -> FResult<&T>
```

## Related

- [[features/celx|celx Feature]] — parent feature
- [[components/celx/cidr|CIDR/IP Module]] — uses `wrap1`, `wrap2`, `wrapnew`, `split_this`, `impl_opaque!`
