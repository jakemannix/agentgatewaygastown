---
feature: core
type: component
files:
  - crates/core/src/signal.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Signal (OS Signal Handling)

## Responsibility

Cross-platform OS signal handling for graceful and forced shutdown, with double-SIGINT immediate exit support.

## Internal Logic

### Shutdown Struct

`Shutdown` wraps an mpsc channel pair for programmatic shutdown:

```rust
let shutdown = Shutdown::new();
let trigger = shutdown.trigger();  // Clone and pass to other tasks

// In main:
shutdown.wait().await;  // Blocks until signal or trigger

// From anywhere:
trigger.shutdown_now().await;  // Programmatic shutdown
```

### Unix Signal Handling

Listens for both SIGINT and SIGTERM via `tokio::signal::unix`:
- First SIGINT: starts graceful shutdown, then spawns a background task watching for a second SIGINT
- Second SIGINT: calls `process::exit(0)` immediately (force exit)
- SIGTERM: starts graceful shutdown (no double-signal handling)
- Programmatic `shutdown_now()`: accepted via the mpsc channel

### Windows Signal Handling

Simplified: listens for Ctrl+C via `tokio::signal::windows::ctrl_c()`, plus the mpsc channel for programmatic shutdown. No double-signal handling.

### Design Notes

- `Shutdown` is NOT Clone; `ShutdownTrigger` IS Clone -- this enforces single-waiter, multiple-trigger semantics
- The mpsc channel has capacity 1, so `shutdown_now()` is effectively fire-and-forget

## Relationships

- **Used by**: `crates/agentgateway-app` binary entry point
- **Triggers**: [[components/core/drain|drain]] (signal typically initiates drain sequence)
- **Parent**: [[features/core|Core]]
