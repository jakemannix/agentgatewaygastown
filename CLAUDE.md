# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Agentgateway is an open source data plane for agentic AI connectivity, written in Rust. It provides security, observability, and governance for agent-to-agent and agent-to-tool communication, supporting Agent2Agent (A2A) and Model Context Protocol (MCP).

## Build Commands

**IMPORTANT: NEVER use `--release` or `make build` during iterative development/testing. Debug builds are much faster.**

```bash
# Build (requires Rust 1.86+, npm 10+)
cd ui && npm install && npm run build && cd ..
export CARGO_NET_GIT_FETCH_WITH_CLI=true
cargo build -p agentgateway-app # Debug build (fast, ALWAYS use this for testing)
make build                      # Release build with UI (SLOW - only for final builds, not testing)

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

## Current Work: Research Assistant Demo + ArrayMap

### Status Summary (2026-02-02)

**FULLY WORKING** - ArrayMap transforms and scatter-gather compositions are operational:

**Rust implementation:**
- `FieldSource::ArrayMap(ArrayMapSource)` variant exists in `patterns/schema_map.rs:68`
- Executor logic in `executor/schema_map.rs:48-78`
- Compiled variant in `compiled.rs:120`
- All unit tests pass: `cargo test -p agentgateway array_map` (4 tests)

**Registry pattern for unified search results:**
```json
{
  "title": "Result title",
  "url": "https://...",
  "snippet": "Description/excerpt",
  "source": "github|huggingface|exa|arxiv",
  "source_type": "repo|model|web|paper"
}
```

**Normalized virtual tools (with arrayMap transforms):**
- `virtual_normalized_exa` - maps `$.results` items from Exa web search
- `virtual_normalized_arxiv` - maps `$.papers` items from arXiv
- `virtual_normalized_github` - maps `$.repos` items from GitHub
- `virtual_normalized_huggingface` - maps `$.models` items from HuggingFace

**Scatter-gather compositions (target normalized tools):**
- `virtual_multi_source_search` - all 4 sources in parallel
- `virtual_academic_search` - arxiv + huggingface
- `virtual_code_search` - github + huggingface

**Aggregation pattern:**
```json
"aggregation": {
  "ops": [
    {"extract": {"path": "$.results"}},
    {"flatten": true},
    {"dedupe": {"field": "$.url"}}
  ]
}
```

**Verified working:**
```bash
# Test scatter-gather with normalized outputs
curl -s -N http://localhost:3000/mcp -X POST \
  -H "mcp-session-id: $SESSION" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"virtual_code_search","arguments":{"query":"llama","num_results":3}}}'

# Returns unified array: 6 items (3 github + 3 huggingface), all with same schema
```

### Quick Start

```bash
# Start backends (from examples/research-assistant-demo)
cd examples/research-assistant-demo
uv run python -m mcp_tools.search_service --port 8001 &
uv run python -m mcp_tools.fetch_service --port 8002 &
uv run python -m mcp_tools.entity_service --port 8003 &
uv run python -m mcp_tools.category_service --port 8004 &
uv run python -m mcp_tools.tag_service --port 8005 &

# Start gateway (from repo root)
./target/debug/agentgateway -f examples/research-assistant-demo/gateway-configs/config.yaml
```

### Python Agent Status

Agent refactored to use Google ADK standard pattern:
- Uses `get_fast_api_app()` from ADK CLI
- `root_agent` defined at module level with `McpToolset`
- Auto-detects LLM provider (Anthropic > OpenAI > Google)

### Next Steps

1. **Test Python agent end-to-end** - Verify ADK agent can call the 40+ gateway tools
2. **Fix arXiv backend** - Getting HTTP 301 redirect, need to follow redirects or update URL
3. **Test Exa** - Requires API key, verify arrayMap works with real data
4. **Design proper tool algebra** (see below) - Clean up virtual_ prefix handling
5. **Review REGISTRY_FORMAT.md** - Document in `examples/research-assistant-demo/gateway-configs/REGISTRY_FORMAT.md` needs review and possible expansion to cover all patterns

### Design TODO: Proper Tool Algebra

The current `virtual_` prefix and underscore-based parsing is a hack that needs to be replaced with a proper model.

**Current problems:**
- `resource_name()` encodes routing info in tool name strings (`{server}_{tool}`)
- `parse_resource_name()` splits on `_`, breaks if tool names contain underscores
- Single vs multiple backend configs behave differently
- `virtual_` prefix is a special case that leaks into tool names
- Fragile string manipulation throughout

**Desired model:**
- All tools have a **server** (source of truth for routing)
- Backend tools: server = the MCP backend name (e.g., `search-service`)
- Virtual/composed tools: server = `gateway` or `local` (the gateway itself)
- Tools are identified by `(server, name)` tuple internally, not string manipulation
- tools/list returns just the `name` - routing is internal
- Registry lookup is the primary resolution mechanism, not string parsing

**Key principle:** A virtual tool can reference another virtual tool which references another virtual tool which references a backend tool. The registry structure and parsing logic should look the same at each level - no special cases for "virtual" vs "real".

**Implementation sketch:**
```rust
// Internal tool identifier - no string parsing needed
struct ToolId {
    server: String,  // "search-service", "gateway", etc.
    name: String,    // "exa_search", "normalized_exa", etc.
}

// Resolution: check registry first, then fall back to backend routing
fn resolve_tool(name: &str) -> ToolId {
    if let Some(tool) = registry.get(name) {
        // Virtual tool - server is implicitly "gateway"
        ToolId { server: "gateway", name }
    } else if let Some((server, tool_name)) = backends.find_tool(name) {
        // Backend tool
        ToolId { server, name: tool_name }
    } else {
        // Error: unknown tool
    }
}
```

This eliminates:
- The `virtual_` prefix convention
- The `DELIMITER` constant and underscore parsing
- The `default_target_name` special case for single-backend configs
- String manipulation in `resource_name()` and `parse_resource_name()`

## Integration Tests for Virtual Tools

**Goal:** Test virtual tool compositions without requiring an LLM, enabling CI/CD integration and regression testing.

**Status:** 43 tests implemented in `examples/research-assistant-demo/tests/`

### Running Tests

```bash
cd examples/research-assistant-demo
uv run pytest tests/ -v
```

Tests automatically start/stop all backend services and the gateway. Total runtime ~25 seconds.

### Current Results (2026-02-02)

**24 PASSED | 2 SKIPPED | 17 FAILED**

The scatter-gather response format issue was fixed by:
1. Adding `outputSchema` referencing `NormalizedSearchResponse` to scatter-gather tools
2. Adding `{"wrap": {"field": "results"}}` to aggregation ops
3. Adding validation in `scripts/validate-registry-schemas.py` to check consistency

See `examples/research-assistant-demo/tests/README.md` for detailed breakdown.

**Remaining Issues:**
1. **Entity service format (4 tests):** Returns `{"success": true, "entity": {...}}`, tests expect `id` at top level
2. **Error handling (6 tests):** Gateway returns error responses instead of raising exceptions
3. **Backend errors (3 tests):** 500 errors from category service and other tools
4. **Data-dependent (2 tests):** HN URL extraction, arrayMap coalesce
5. **Backend validation (1 test):** Backend enforces max=30, returns error instead of capping

### Test Infrastructure

Location: `examples/research-assistant-demo/tests/`

```
tests/
├── conftest.py                    # Pytest fixtures: start services, gateway, MCP client
├── test_arraymap_transforms.py    # ArrayMap field source tests (5 pass, 1 fail)
├── test_scatter_gather.py         # Parallel execution + aggregation (0 pass, 7 fail, 1 skip)
├── test_pipelines.py              # Sequential step execution (2 pass, 5 fail)
├── test_aggregation_ops.py        # Extract, flatten, dedupe, merge (0 pass, 7 fail)
└── test_error_handling.py         # Error cases (4 pass, 9 fail)
```

### Test Categories

1. **ArrayMap Transforms** - Verify `arrayMap` correctly transforms array elements to normalized schema
2. **Scatter-Gather** - Test parallel execution and result aggregation
3. **Pipelines** - Test sequential step execution with data flow
4. **Aggregation Ops** - Test extract, flatten, dedupe, merge operations
5. **Error Handling** - Test graceful degradation and error messages

### Test Fixtures (conftest.py)

- `ServiceManager` - Starts 5 backend MCP services (ports 8001-8005)
- `GatewayManager` - Starts agentgateway binary (port 3000)
- `McpHttpClient` - Synchronous HTTP client for MCP over SSE
- `call_tool` fixture - Simple `call_tool(name, args)` helper for tests

Key implementation detail: The MCP session handshake requires:
1. Initialize request with NO session header
2. Extract `mcp-session-id` from response header
3. Include session header in subsequent tool calls

### Running Tests

```bash
# From repo root
cd examples/research-assistant-demo
pytest tests/ -v

# Run specific test
pytest tests/test_scatter_gather.py::test_scatter_gather_code_search -v

# With coverage
pytest tests/ --cov=. --cov-report=html
```

### Mock Backends for CI

For CI without external API keys, create mock backends that return canned responses:

```python
# mcp_tools/mock_search_service/server.py
@mcp.tool()
async def github_search(query: str, num_results: int = 10) -> dict:
    """Mock GitHub search for testing."""
    return {
        "query": query,
        "repos": [
            {"full_name": f"test/repo-{i}", "description": f"Test repo {i}",
             "html_url": f"https://github.com/test/repo-{i}"}
            for i in range(min(num_results, 5))
        ]
    }
```

This allows testing the gateway's composition logic without hitting real APIs.
