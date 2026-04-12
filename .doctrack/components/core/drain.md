---
feature: core
type: component
files:
  - crates/core/src/drain.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - '3961'
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Drain (Graceful Shutdown Orchestration)

## Responsibility

Coordinate graceful shutdown across multiple concurrent connections and services, with configurable deadlines and force-shutdown fallback.

## Internal Logic

### Channel Primitives

`drain::new()` creates a `(DrainTrigger, DrainWatcher)` pair:

- **`DrainTrigger` (Signal)**: Owns the drain lifecycle. `start_drain_and_wait(mode)` broadcasts the drain signal and blocks until all watchers have dropped their handles.
- **`DrainWatcher` (Watch)**: Cloneable. Each clone represents an active connection/task. `wait_for_drain()` returns a `ReleaseShutdown` guard -- the drain completes only when all guards are dropped.

### DrainMode

- `DrainMode::Graceful` - Allow in-flight work to complete within deadline
- `DrainMode::Immediate` - Terminate without waiting

### Weak/Strong Pattern

`Watch::into_weak()` splits into `(Upgrader, Weak)`:
- `Weak` does NOT block drain completion (useful for long-lived background watchers)
- `Upgrader::upgrade(weak)` converts a Weak back into a full Watch that blocks drain
- `Upgrader::disable()` permanently detaches, creating dummy channels

### `run_with_drain`

High-level wrapper for running a component with drain support:

```
1. Create sub-drain for the component
2. Run the component future with (sub_drain, force_shutdown_rx)
3. When parent drain signals:
   a. Sleep for min_delay (workaround for hyper issue #3961)
   b. Start sub-drain, wait up to deadline
   c. If timeout: log warning, trigger force_shutdown
   d. Log "shutdown complete"
```

### Hyper Integration (`hyperfork` module)

`GracefulConnection` trait wraps hyper-util's same trait. `GracefulConnectionFuture` is a pin-projected future that:
1. Polls the cancel future alongside the connection
2. When cancel resolves, calls `graceful_shutdown()` on the connection
3. Continues polling the connection until it completes

`Watch::wrap_connection(conn)` composes a connection with drain support in a single call.

### Testing

Three tests cover: graceful shutdown with all connections completing, timeout with a stuck connection, and the weak/strong upgrade pattern.

## Relationships

- **Used by**: `crates/agentgateway` proxy for connection lifecycle management
- **Uses**: [[components/core/signal|signal]] (typically Signal triggers drain)
- **Parent**: [[features/core|Core]]
