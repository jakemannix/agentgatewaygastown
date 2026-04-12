---
feature: agentgateway-app
type: feature
doctrack_version: 3.0.0
files:
  - crates/agentgateway-app/src/main.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# Agentgateway App (Binary Entry Point)

## Purpose
Binary entry point for the agentgateway proxy. Handles CLI argument parsing, config loading, telemetry initialization, and starts the gateway. Originally derived from Istio's ztunnel.

## Key File
- `crates/agentgateway-app/src/main.rs` — Single file binary

## CLI Args
- `-c / --config` — Config from inline bytes
- `-f / --file` — Config from YAML/JSON file
- `--validate-only` — Validate config without starting
- `-V` — Short version string
- `--version` — Version as JSON
- `--copy-self` — Copy binary to destination (hidden, for deployment)

## Runtime Features
- jemalloc allocator (with profiling support, except on musl)
- Version info via `agent_core::version::BuildInfo`
- Logging format selection via `LoggingFormat`

## Dependencies
- **Internal**: [[features/core|Core]] (telemetry, version), `agentgateway` (all gateway logic)
- **External**: `clap` (CLI), `tikv-jemallocator` (allocator)
