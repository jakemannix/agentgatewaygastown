---
feature: agentgateway-client
type: feature
doctrack_version: "3.0.0"
files:
  - crates/agentgateway/src/client/mod.rs
  - crates/agentgateway/src/client/dns.rs
  - crates/agentgateway/src/client/tls.rs
  - crates/agentgateway/src/client/connect_tunnel.rs
  - crates/agentgateway/src/client/hbone_tunnel.rs
  - crates/agentgateway/src/client/azure.rs
last_updated: 2026-04-12
status: active
---

# Client (Upstream Connections)

## Purpose
Manages outbound connections to upstream backends — DNS resolution, TLS client config, HTTP CONNECT tunneling, HBONE tunneling, and Azure-specific auth.

## Key Files
- `client/mod.rs` — Client pool and connection management
- `client/dns.rs` — DNS resolution with hickory-resolver
- `client/tls.rs` — TLS client configuration, certificate management
- `client/connect_tunnel.rs` — HTTP CONNECT tunnel support
- `client/hbone_tunnel.rs` — HBONE tunnel client (uses [[features/hbone|hbone crate]])
- `client/azure.rs` — Azure-specific authentication

## Dependencies
- **Internal**: [[features/hbone|HBONE]], [[features/agentgateway-transport|Transport]], [[features/core|Core]]
- **Used by**: [[features/agentgateway-proxy|Proxy]], [[features/agentgateway-mcp|MCP]]
