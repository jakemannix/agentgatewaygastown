---
type: decision
status: accepted
date: 2024-01-01T00:00:00.000Z
related_features:
  - core
  - hbone
  - agentgateway-proxy
last_updated: 2026-04-12T00:00:00.000Z
tags:
  - doctrack/type/decision
  - doctrack/status/accepted
  - doctrack/audience/claude
---

# Decision: Built on Istio ztunnel Foundation

## Status
**Accepted**

## Context
Agentgateway needed a high-performance Rust proxy foundation with production-grade telemetry, connection management, and TLS support.

## Decision
Derived core infrastructure from Istio's ztunnel project (Apache 2.0 licensed). The `core`, `hbone`, and proxy modules carry this heritage. ztunnel provided: non-blocking logging with vectored I/O, adaptive buffer copy, graceful drain/shutdown, HBONE tunneling, and XDS integration.

## Consequences
- Production-tested foundation from Istio's ambient mesh
- HBONE tunneling support (may not be widely used in agentic AI context)
- XDS protocol for dynamic config (enables Kubernetes integration via kgateway)
- Strong Envoy compatibility in HTTP module (ext_authz, ext_proc patterns)

## Related
- [[features/core|Core crate]]
- [[features/hbone|HBONE]]
- [[features/xds|XDS]]
