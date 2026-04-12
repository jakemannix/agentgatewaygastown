---
feature: core
type: component
files:
  - crates/core/src/arc.rs
  - crates/core/src/bow.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Arc + Bow (Smart Pointer Helpers)

## Responsibility

Convenience type aliases and abstractions for atomic shared ownership and borrowed-or-owned values.

## Internal Logic

### arc.rs -- Atomic Swap Types

Two type aliases wrapping `arc-swap`:

```rust
pub type Atomic<T> = Arc<ArcSwap<T>>;        // Always has a value
pub type AtomicOption<T> = Arc<ArcSwapOption<T>>; // May be empty
```

These provide lock-free read access with atomic swap for updates. Used throughout for configuration that can be hot-reloaded (e.g., routing tables, policy rules). The outer `Arc` makes the atomic swappable value itself cheaply shareable across tasks.

### bow.rs -- OwnedOrBorrowed

```rust
pub enum OwnedOrBorrowed<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}
```

Like `std::borrow::Cow` but does NOT require `T: Clone`. Implements `Deref<Target = T>` and `AsRef<T>`. Useful when a function may either borrow an existing value or construct a new one, and the caller only needs read access.

## Relationships

- **Re-exported via**: [[components/core/prelude|prelude]] (`Atomic`, `AtomicOption`)
- **Used by**: `crates/agentgateway` config module for hot-reloadable state
- **Parent**: [[features/core|Core]]
