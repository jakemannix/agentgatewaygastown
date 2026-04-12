---
type: reference
original_path: architecture/configuration.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Configuration Architecture (imported)

Source: `architecture/configuration.md`

Three configuration layers:

## Static Configuration
Set once at process startup via environment variables or YAML/JSON. Global settings: logging, ports. No routing/policies here.

## Local Configuration
File-based (YAML/JSON) with file-watch hot-reload. Defines full feature set: backends, routes, policies. Translates to shared Internal Representation (IR) used at runtime.

## XDS Configuration
Remote control plane using XDS Transport Protocol (Envoy's protocol, but with purpose-built types, NOT Envoy types). Maps to same shared IR as local config.

### Key Design Philosophy
- Nearly direct mapping: user-facing API → XDS → IR
- Resources point UP to parents (not parent containing child list) — avoids fanout
- One HTTPRoute rule = one agentgateway Route (not a list of all routes)
- One Pod = one Workload (not list of all endpoints)
- Policies applied to Gateway/Listener/HTTPRoute/HTTPRouteRule with merging semantics
- Precedence/merging of policies handled at runtime, not control plane

This avoids Envoy's problem where changing 1 field fans out to every Cluster/Route.
