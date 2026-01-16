# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Agentgateway is an open source data plane for agentic AI connectivity, written in Rust. It provides security, observability, and governance for agent-to-agent and agent-to-tool communication, supporting Agent2Agent (A2A) and Model Context Protocol (MCP).

## Build Commands

```bash
# Build (requires Rust 1.86+, npm 10+)
cd ui && npm install && npm run build && cd ..
export CARGO_NET_GIT_FETCH_WITH_CLI=true
make build                      # Release build with UI

# Lint
make lint                       # Check formatting and clippy
make fix-lint                   # Auto-fix lint issues

# Test
make test                       # Run all tests
cargo test -p agentgateway --test <test_name>  # Single test file
cargo test <test_fn_name>       # Single test function

# Code generation
make gen                        # Regenerate APIs and schema

# Validate configs
make validate                   # Validate all example configs

# Run
./target/release/agentgateway -f examples/basic/config.yaml
# UI at http://localhost:15000/ui
```

## Crate Architecture

```
crates/
├── agentgateway/       # Main library: proxy, MCP/A2A handling, LLM support, config
├── agentgateway-app/   # Binary entry point
├── a2a-sdk/            # Agent2Agent protocol types (published to crates.io)
├── core/               # Shared primitives: telemetry, metrics, tracing
├── celx/               # CEL expression evaluation wrapper
├── xds/                # XDS protocol for dynamic configuration
├── hbone/              # HTTP/2 CONNECT tunneling (HBONE)
└── xtask/              # Development tasks (schema generation)
```

## Key Modules in `crates/agentgateway/src/`

- `mcp/` - Model Context Protocol handling (tools, resources, prompts)
- `a2a/` - Agent2Agent protocol handling
- `llm/` - LLM request handling (OpenAI, Anthropic, etc.) with passthrough parsing
- `proxy/` - Core proxy logic and request routing
- `config.rs` - Configuration parsing and validation
- `cel/` - CEL expression context building and evaluation
- `transport/` - HTTP/SSE/WebSocket transport handling
- `control/` - Control plane for dynamic updates

## Configuration Model

Three configuration layers:
1. **Static** - Environment variables/YAML for global settings (ports, logging)
2. **Local** - File-based with hot-reload for routing, policies, backends
3. **XDS** - Remote control plane for dynamic configuration

Configuration hierarchy: `binds[] → listeners[] → routes[] → backends[]`

Key config file locations:
- `examples/*/config.yaml` - Example configurations
- `schema/config.json` - JSON Schema for config validation
- `crates/agentgateway/proto/resource.proto` - XDS resource definitions

## CEL (Common Expression Language)

Used extensively for runtime policies:
- Authorization: `jwt.sub == "user" && mcp.tool.name == "add"`
- Header modification
- Rate limiting key extraction
- Log/trace field enrichment

Variables available depend on request phase. Schema at `schema/cel.json`.

## LLM Passthrough Pattern

LLM types use passthrough parsing for compatibility:
```rust
#[serde(flatten, default)]
pub rest: serde_json::Value
```
Only fields explicitly operated on are defined; unknown fields pass through.

## Conventions

- Follows Conventional Commits: `feat:`, `fix:`, `docs:`, etc.
- UI changes require: `cd ui && npm run lint && npm test`
- Proto changes require: `make generate-apis`
- Config/schema changes require: `make generate-schema`
