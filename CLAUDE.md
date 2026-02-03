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

## Integration Tests for Virtual Tools (IMPLEMENTED)

**Goal:** Test virtual tool compositions without requiring an LLM, enabling CI/CD integration and regression testing.

**Status:** 43 tests implemented in `examples/research-assistant-demo/tests/`

### Running Tests

```bash
cd examples/research-assistant-demo
uv run pytest tests/ -v
```

### Test Infrastructure

Location: `examples/research-assistant-demo/tests/`

```
tests/
├── conftest.py                    # Pytest fixtures: start services, create MCP client
├── test_arraymap_transforms.py    # ArrayMap field source tests
├── test_scatter_gather.py         # Parallel execution + aggregation
├── test_pipelines.py              # Sequential step execution
├── test_aggregation_ops.py        # Extract, flatten, dedupe, sort, limit
└── test_error_handling.py         # Partial failures, backend errors
```

### Test Categories

#### 1. ArrayMap Transforms
Test that `arrayMap` correctly transforms each element of an array.

```python
async def test_arraymap_github_normalization(gateway):
    """GitHub results are normalized to common schema."""
    result = await gateway.call_tool("virtual_normalized_github", {
        "query": "test", "num_results": 3
    })

    assert "results" in result
    for item in result["results"]:
        # Verify normalized schema
        assert item["source"] == "github"
        assert item["source_type"] == "repo"
        assert "title" in item
        assert "url" in item
        assert "snippet" in item
        # Verify URL is properly extracted (not nested)
        assert item["url"].startswith("https://github.com/")
```

Similar tests for: `virtual_normalized_exa`, `virtual_normalized_arxiv`, `virtual_normalized_huggingface`

#### 2. Scatter-Gather Compositions
Test parallel execution and result aggregation.

```python
async def test_scatter_gather_code_search(gateway):
    """virtual_code_search runs github + huggingface in parallel."""
    result = await gateway.call_tool("virtual_code_search", {
        "query": "transformer", "num_results": 3
    })

    # Should have results from both sources
    sources = {r["source"] for r in result["results"]}
    assert "github" in sources
    assert "huggingface" in sources

    # Results should be flattened (not nested arrays)
    assert isinstance(result["results"], list)
    assert all(isinstance(r, dict) for r in result["results"])

async def test_scatter_gather_partial_failure(gateway):
    """If one backend fails, others still return results."""
    # Stop one backend, verify scatter-gather still works
    # with results from remaining backends
```

#### 3. Aggregation Operations
Test extract, flatten, dedupe, sort, limit.

```python
async def test_aggregation_dedupe(gateway):
    """Dedupe removes duplicate URLs across sources."""
    result = await gateway.call_tool("virtual_multi_source_search", {
        "query": "langchain", "num_results": 20
    })

    urls = [r["url"] for r in result["results"]]
    assert len(urls) == len(set(urls)), "Duplicate URLs found"

async def test_aggregation_flatten(gateway):
    """Results from multiple sources are flattened into single array."""
    result = await gateway.call_tool("virtual_academic_search", {
        "query": "attention mechanism", "num_results": 5
    })

    # Should be flat list, not nested
    assert isinstance(result["results"], list)
    assert not any(isinstance(r, list) for r in result["results"])
```

#### 4. Pipeline Compositions
Test sequential step execution with data flow.

```python
async def test_pipeline_fetch_and_extract(gateway):
    """Pipeline: url_fetch -> extract_urls."""
    result = await gateway.call_tool("virtual_fetch_and_extract", {
        "url": "https://example.com"
    })

    assert "content" in result or "extracted_urls" in result
    # Verify step 2 received output from step 1

async def test_pipeline_store_research_finding(gateway):
    """Cross-service pipeline: entity-service -> tag-service."""
    result = await gateway.call_tool("virtual_store_research_finding", {
        "title": "Test Finding",
        "url": "https://test.com",
        "summary": "Test summary",
        "tags": ["test"]
    })

    # Verify entity was created
    assert "entity" in result
    assert "id" in result["entity"]

    # Verify content was registered with entity_id from step 1
    assert "content" in result
    assert result["content"]["metadata"]["entity_id"] == result["entity"]["id"]
```

#### 5. Error Handling
Test graceful degradation.

```python
async def test_backend_timeout_handling(gateway):
    """Slow backend doesn't block entire scatter-gather."""
    # Configure one backend to be slow
    # Verify other results still return

async def test_invalid_tool_name(gateway):
    """Unknown tool returns proper error."""
    with pytest.raises(McpError) as exc:
        await gateway.call_tool("nonexistent_tool", {})
    assert "unknown tool" in str(exc.value).lower()

async def test_missing_required_param(gateway):
    """Missing required parameter returns proper error."""
    with pytest.raises(McpError) as exc:
        await gateway.call_tool("virtual_normalized_github", {})  # missing 'query'
    assert "query" in str(exc.value).lower()
```

### Test Fixtures

```python
# conftest.py
import pytest
import asyncio
import subprocess
from mcp import ClientSession
from mcp.client.streamable_http import streamablehttp_client

@pytest.fixture(scope="session")
def event_loop():
    loop = asyncio.new_event_loop()
    yield loop
    loop.close()

@pytest.fixture(scope="session")
async def services():
    """Start backend MCP services."""
    procs = []
    for port, module in [
        (8001, "mcp_tools.search_service"),
        (8002, "mcp_tools.fetch_service"),
        (8003, "mcp_tools.entity_service"),
        (8004, "mcp_tools.category_service"),
        (8005, "mcp_tools.tag_service"),
    ]:
        proc = subprocess.Popen(
            ["uv", "run", "python", "-m", module, "--port", str(port)],
            cwd="examples/research-assistant-demo"
        )
        procs.append(proc)

    await asyncio.sleep(3)  # Wait for startup
    yield

    for proc in procs:
        proc.terminate()

@pytest.fixture(scope="session")
async def gateway(services):
    """Start gateway and return MCP client."""
    proc = subprocess.Popen(
        ["./target/debug/agentgateway", "-f",
         "examples/research-assistant-demo/gateway-configs/config.yaml"]
    )
    await asyncio.sleep(2)

    async with streamablehttp_client("http://localhost:3000/mcp") as (r, w, _):
        async with ClientSession(r, w) as session:
            await session.initialize()
            yield session

    proc.terminate()
```

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
