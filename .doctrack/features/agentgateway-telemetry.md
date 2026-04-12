---
feature: agentgateway-telemetry
type: feature
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/telemetry/mod.rs
  - crates/agentgateway/src/telemetry/log.rs
  - crates/agentgateway/src/telemetry/metrics.rs
  - crates/agentgateway/src/telemetry/trc.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# Telemetry

## Purpose
Gateway-specific telemetry: structured logging with CEL enrichment, prometheus metrics, and distributed tracing. Builds on [[features/core|core crate]]'s telemetry primitives.

## Key Files
- `telemetry/mod.rs` — Module root, telemetry initialization
- `telemetry/log.rs` — Structured logging with CEL-based field enrichment, `CompositionVerbosity` for controlling virtual tool debug output, `FlattenSignal` handling from [[features/celx|celx]]
- `telemetry/metrics.rs` — Prometheus metrics: request counts, latency histograms, error rates per operation type
- `telemetry/trc.rs` — Distributed tracing (OpenTelemetry spans)

## Key Types
- `CompositionVerbosity` / `CompositionVerbosityExpr` — Controls how much detail compositions log (off/summary/full)
- `AsyncLog` — Async structured logging trait
- `OrderedStringMap` — Ordered map for structured log fields

## Dependencies
- **Internal**: [[features/core|Core]] (telemetry primitives), [[features/celx|celx]] (`FlattenSignal`)
- **Used by**: All features — provides observability layer
