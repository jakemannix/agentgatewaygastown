---
feature: core
type: component
files:
  - crates/core/src/responsechannel.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# ResponseChannel (Request/Response Channels)

## Responsibility

Typed async request/response channel combining mpsc (for requests) with oneshot (for per-request responses), enabling send-and-wait patterns.

## Internal Logic

### Channel Creation

```rust
let (sender, receiver) = responsechannel::new::<Request, Response>(buffer_size);
```

### Sender Side

- `send_and_wait(request)` - Sends request and awaits the response via a oneshot channel. Returns `anyhow::Result<R>`.
- `send_ignore(request)` - Sends request but discards the response (creates a oneshot but drops the receiver). Returns `SendError` if the channel is closed.
- `Sender` is `Clone` -- multiple producers can share the same channel.

### Receiver Side

- `recv()` - Returns `Option<(T, oneshot::Sender<R>)>`. The receiver processes the request and sends the response back via the oneshot sender.
- Returns `None` when all senders are dropped (channel closed).

### Type Aliases

- `AckSender<T>` = `Sender<T, ()>` -- for fire-and-acknowledge patterns (response is just `()`)
- `AckReceiver<T>` = `Receiver<T, ()>` -- matching receiver

### Design Notes

- This is essentially a typed RPC channel over tokio primitives
- The oneshot response channel is created per-request, so responses are guaranteed to reach the correct sender
- No timeout built in -- callers should wrap `send_and_wait` with `tokio::time::timeout` if needed

## Relationships

- **Used by**: `crates/agentgateway` for internal control plane communication
- **Parent**: [[features/core|Core]]
