---
feature: xtask
type: feature
doctrack_version: 3.0.0
files:
  - crates/xtask/src/main.rs
  - crates/xtask/src/schema.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# Xtask (Development Tasks)

## Purpose
Cargo xtask pattern for development automation. Currently only generates JSON schemas from Rust types.

## Commands
- `cargo xtask schema` — Generate JSON Schema for config validation (writes to `schema/config.json`, `schema/cel.json`)

## Key Files
- `crates/xtask/src/main.rs` — Task dispatcher
- `crates/xtask/src/schema.rs` — Schema generation logic

## Dependencies
- **Internal**: Uses agentgateway types to derive schemas via `schemars`
