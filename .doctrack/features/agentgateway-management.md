---
feature: agentgateway-management
type: feature
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/management/mod.rs
  - crates/agentgateway/src/management/admin.rs
  - crates/agentgateway/src/management/metrics_server.rs
  - crates/agentgateway/src/management/readiness_server.rs
  - crates/agentgateway/src/management/hyper_helpers.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# Management API

## Purpose
Admin/management HTTP server running on a separate port from the proxy. Provides health checks, readiness probes, prometheus metrics endpoint, and admin API for inspecting gateway state.

## Key Files
- `management/mod.rs` — Management server setup
- `management/admin.rs` — Admin API endpoints (config dump, backend state)
- `management/metrics_server.rs` — Prometheus metrics exposition
- `management/readiness_server.rs` — Health/readiness probe endpoints
- `management/hyper_helpers.rs` — Hyper HTTP helpers

## Dependencies
- **Internal**: [[features/agentgateway-telemetry|Telemetry]] (metrics), [[features/core|Core]]
- **External**: `hyper` (HTTP server)
