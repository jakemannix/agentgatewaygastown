---
feature: agentgateway-control
type: feature
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/control/mod.rs
  - crates/agentgateway/src/control/caclient.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# Control Plane Integration

## Purpose
Connects to a remote control plane via [[features/xds|XDS]] for dynamic configuration. Also manages CA (Certificate Authority) client for mTLS certificate provisioning.

## Key Files
- `control/mod.rs` — Control plane connection management, XDS subscription setup
- `control/caclient.rs` — CA client for certificate signing requests

## Dependencies
- **Internal**: [[features/xds|XDS]], [[features/agentgateway-transport|Transport]] (TLS)
- **Used by**: [[features/agentgateway-proxy|Proxy]] (dynamic config delivery)
