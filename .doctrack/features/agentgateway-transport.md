---
feature: agentgateway-transport
type: feature
doctrack_version: "3.0.0"
files:
  - crates/agentgateway/src/transport/mod.rs
  - crates/agentgateway/src/transport/stream.rs
  - crates/agentgateway/src/transport/tls.rs
  - crates/agentgateway/src/transport/hbone.rs
  - crates/agentgateway/src/transport/rewind.rs
last_updated: 2026-04-12
status: active
---

# Transport

## Purpose
Low-level transport abstractions for incoming connections — TCP streams, TLS termination, HBONE tunneling, and stream rewinding. Provides the `BufferLimit` and connection info types used by the proxy.

## Key Files
- `transport/mod.rs` — `BufferLimit` newtype
- `transport/stream.rs` — `TCPConnectionInfo`, `TLSConnectionInfo` — connection metadata
- `transport/tls.rs` — TLS termination and certificate handling
- `transport/hbone.rs` — HBONE tunnel integration (uses [[features/hbone|hbone crate]])
- `transport/rewind.rs` — Stream rewinding for protocol detection (peek at bytes without consuming)

## Dependencies
- **Internal**: [[features/hbone|HBONE]], [[features/core|Core]]
- **Used by**: [[features/agentgateway-proxy|Proxy]]
