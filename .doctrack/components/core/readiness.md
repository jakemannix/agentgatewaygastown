---
feature: core
type: component
files:
  - crates/core/src/readiness.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Readiness (Startup Tracking)

## Responsibility

Track startup readiness of the agentgateway process by requiring all registered tasks to complete before the server is marked ready.

## Internal Logic

### RAII Pattern

`Ready` holds a `HashSet<String>` of pending task names behind `Arc<Mutex<>>`.

```rust
let ready = Ready::new();
let block = ready.register_task("config-load");  // Adds "config-load" to pending set
// ... do initialization work ...
drop(block);  // Removes "config-load", logs completion with elapsed time
```

When the last `BlockReady` is dropped, it logs "marking server ready" with the total elapsed time since `APPLICATION_START_TIME`.

### Subtasks

`BlockReady::subtask(name)` registers a child task on the same `Ready` instance. Parent and child are independent -- dropping one doesn't affect the other.

### Observability

- `ready.pending()` returns the current set of incomplete task names (useful for health check endpoints)
- Each task completion logs elapsed time since process start
- Final task logs the "server ready" message

### Design Notes

- Derived from Istio ztunnel
- Uses `APPLICATION_START_TIME` from [[components/core/telemetry|telemetry]] for elapsed time calculation
- Thread-safe via `Arc<Mutex<>>` -- tasks can complete from any thread/task

## Relationships

- **Used by**: `crates/agentgateway-app` for startup sequencing
- **Uses**: [[components/core/telemetry|telemetry]] for `APPLICATION_START_TIME`
- **Parent**: [[features/core|Core]]
