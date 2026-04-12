---
type: concept
related_features:
  - agentgateway-types
  - agentgateway-config
  - agentgateway-control
  - xds
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/concept
  - doctrack/status/active
  - doctrack/audience/claude
---

# Three-Layer Configuration Model

## What It Is
Agentgateway supports three configuration sources that all converge into a shared Internal Representation (IR):

1. **Static** — environment variables / CLI YAML, set once at startup (ports, logging)
2. **Local** — file-based YAML/JSON with hot-reload via file watch (full feature set)
3. **XDS** — remote control plane via gRPC Delta Discovery (dynamic, purpose-built types)

## Where It Appears
- [[features/agentgateway-types|Types]] — defines the shared IR
- [[features/agentgateway-config|Config parsing]] — local config → IR
- [[features/xds|XDS]] — XDS → IR
- [[features/agentgateway-control|Control plane]] — XDS client connection
- [[features/agentgateway-store|Store]] — holds the runtime IR state
- [[references/imported/configuration|Configuration Architecture]] — original design doc

## Key Design Principle
Nearly direct mapping: user-facing API → XDS → IR. Resources point UP to parents (not parent containing child list). This avoids Envoy's fanout problem where changing 1 field fans out to every Cluster/Route.
