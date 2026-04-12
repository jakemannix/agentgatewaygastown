---
feature: agentgateway-config
type: feature
doctrack_version: "3.0.0"
files:
  - crates/agentgateway/src/parse/mod.rs
  - crates/agentgateway/src/parse/passthrough.rs
  - crates/agentgateway/src/parse/sse.rs
  - crates/agentgateway/src/parse/aws_sse.rs
  - crates/agentgateway/src/parse/transform.rs
  - crates/agentgateway/src/parse/websocket.rs
last_updated: 2026-04-12
status: active
---

# Config Parsing and Protocol Parsing

## Purpose
Handles configuration file parsing and protocol-level parsing for SSE, WebSocket, and AWS-specific SSE streams. Also contains the passthrough parsing pattern used for LLM request forwarding.

## Key Files
- `parse/mod.rs` — Module root, parsing utilities
- `parse/passthrough.rs` — Passthrough parsing pattern (parse only known fields, forward rest)
- `parse/sse.rs` — Server-Sent Events stream parsing
- `parse/aws_sse.rs` — AWS Bedrock SSE stream parsing
- `parse/transform.rs` — Request/response transformation parsing
- `parse/websocket.rs` — WebSocket frame parsing

## Dependencies
- **Used by**: [[features/agentgateway-llm|LLM]], [[features/agentgateway-mcp|MCP]], [[features/agentgateway-proxy|Proxy]]
