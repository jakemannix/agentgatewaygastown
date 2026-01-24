# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Agentgateway is an open source data plane for agentic AI connectivity, written in Rust. It provides security, observability, and governance for agent-to-agent and agent-to-tool communication, supporting Agent2Agent (A2A) and Model Context Protocol (MCP).

## Build Commands

```bash
# Build (requires Rust 1.86+, npm 10+)
cd ui && npm install && npm run build && cd ..
export CARGO_NET_GIT_FETCH_WITH_CLI=true
cargo build -p agentgateway-app # Debug build (fast, use for local testing)
make build                      # Release build with UI (slow, use for final testing only)

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

## Current Work: eCommerce Demo ADK Integration

### Changes Made
1. **Customer agent refactored to use native MCP** (`examples/ecommerce-demo/agents/customer_agent/agent.py`)
   - Replaced manual `FunctionTool` creation with ADK's `McpToolset`
   - Uses `StreamableHTTPConnectionParams` to connect to gateway at `/mcp`
   - Tool schemas now properly passed to LLM via MCP protocol

2. **Registry v2 updated with proper structure** (`examples/ecommerce-demo/gateway-configs/ecommerce_registry_v2.json`)
   - Added `schemas` section with 11 reusable JSON Schema definitions
   - Added `servers` section with backend service metadata
   - Tools now use `$ref` references instead of inline schemas
   - Added `outputTransform` with `mappings` for `personalized_search` composition

### Testing Steps
1. Start the ecommerce demo services:
   ```bash
   cd examples/ecommerce-demo && ./start_services.sh
   ```

2. In a separate terminal, restart the gateway to pick up registry changes:
   ```bash
   RUST_LOG=debug ./target/release/agentgateway -f examples/ecommerce-demo/gateway-configs/config.yaml
   ```

3. Test the customer agent via the chat endpoint:
   ```bash
   curl -X POST http://localhost:9001/chat \
     -H "Content-Type: application/json" \
     -d '{"message":"search for coffee makers","session_id":"test"}'
   ```

4. **What to look for:**
   - Agent should successfully call `personalized_search` with proper `query` argument
   - Gateway logs should show composition execution completing (not 500 error)
   - Response should include product results

### Remaining TODOs
1. **Fix gateway structuredContent response** - The gateway needs to populate `structuredContent` in the MCP response when a tool has an `outputSchema`. Currently returns text content only, which causes ADK to error with "Tool has an output schema but did not return structured content". Fix needed in `crates/agentgateway/src/mcp/session.rs` around line 468.

2. **Update merchandiser_agent to use McpToolset** - Same refactor as customer_agent: replace manual FunctionTool/LangChain tool creation with native `McpToolset`.

3. **Clean up gateway_client.py** - The manual `create_adk_tools()` and `create_langchain_tools()` functions are no longer needed if agents use `McpToolset` directly. Consider deprecating or removing.

4. **Test session-stored caller identity** - The fix for storing caller identity from MCP `clientInfo` during initialize (instead of only from headers) has been committed but needs integration testing. Run the test: `cargo test -p agentgateway session_identity_from_client_info_filters_tools`

5. **Test and validate test_integration.py** - New integration test script at `examples/ecommerce-demo/test_integration.py` has been added but is untested. Run with `python test_integration.py` (requires gateway + services running). Verify it works and update as needed.
