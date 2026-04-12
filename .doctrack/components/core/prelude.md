---
feature: core
type: component
files:
  - crates/core/src/prelude.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Prelude (Common Re-exports)

## Responsibility

Single import for the most commonly used types and traits across the workspace, reducing boilerplate `use` statements.

## Contents

### Standard Library
- `std::fmt::{Debug, Display}`
- `std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr}`
- `std::pin::Pin`
- `std::sync::{Arc, Mutex}`
- `std::task::{Context, Poll, ready}`
- `std::time::{Duration, Instant}`

### Third-Party
- `anyhow::Context as _` (unnamed import for `.context()` method on Results)
- `bytes::Bytes`
- `tokio::sync::Mutex as AsyncMutex`
- `tracing::{Instrument, debug, error, info, trace, warn}`

### Crate-Internal
- `crate::arc::{Atomic, AtomicOption}` -- `Arc<ArcSwap<T>>` / `Arc<ArcSwapOption<T>>`
- `crate::strng` (module) and `crate::strng::Strng` (type)

## Relationships

- **Used by**: Every module in `crates/agentgateway` via `use agent_core::prelude::*`
- **Re-exports**: [[components/core/strng|strng]], [[components/core/arc-bow|arc]]
- **Parent**: [[features/core|Core]]
