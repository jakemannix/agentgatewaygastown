# Integration Tests for Virtual Tool Compositions

These tests verify the gateway's virtual tool composition patterns (scatter-gather, pipelines, arrayMap transforms) without requiring an LLM.

## Prerequisites

1. Build the gateway (from repo root):
   ```bash
   cargo build -p agentgateway-app
   ```

2. Install Python test dependencies (from `examples/research-assistant-demo`):
   ```bash
   uv sync
   ```

## Running the Tests

From `examples/research-assistant-demo`:

```bash
# Run all tests (services and gateway start automatically)
uv run pytest tests/ -v

# Run a specific test file
uv run pytest tests/test_arraymap_transforms.py -v

# Run a single test
uv run pytest tests/test_arraymap_transforms.py::test_normalized_github_schema -v

# Run with verbose output
uv run pytest tests/ -v --tb=short
```

The test fixtures automatically:
1. Start 5 backend MCP services (ports 8001-8005)
2. Start the agentgateway (port 3000)
3. Initialize an MCP session
4. Run tests
5. Shut down all processes

Total runtime is ~25 seconds.

## Test Results Summary

**24 PASSED | 2 SKIPPED | 17 FAILED**

The scatter-gather response format issue was fixed by adding `outputSchema` and `wrap` aggregation op to the registry tools.

### Passing Tests (24)

| Test | File | Description |
|------|------|-------------|
| `test_normalized_github_schema` | arraymap_transforms | GitHub results normalized to common schema |
| `test_normalized_huggingface_schema` | arraymap_transforms | HuggingFace results normalized to common schema |
| `test_normalized_arxiv_schema` | arraymap_transforms | arXiv results normalized to common schema |
| `test_arraymap_preserves_array_order` | arraymap_transforms | ArrayMap maintains element order |
| `test_arraymap_handles_empty_source_array` | arraymap_transforms | Empty arrays handled gracefully |
| `test_code_search_combines_sources` | scatter_gather | GitHub + HuggingFace parallel search |
| `test_academic_search_combines_sources` | scatter_gather | arXiv + HuggingFace parallel search |
| `test_scatter_gather_results_flattened` | scatter_gather | Results from multiple sources flattened |
| `test_scatter_gather_dedupe_by_url` | scatter_gather | Duplicate URLs removed |
| `test_scatter_gather_partial_failure` | scatter_gather | Partial failures handled gracefully |
| `test_scatter_gather_timeout_handling` | scatter_gather | Timeouts handled correctly |
| `test_extract_pulls_nested_array` | aggregation_ops | Extract op pulls nested arrays |
| `test_flatten_combines_arrays` | aggregation_ops | Flatten combines arrays |
| `test_dedupe_removes_duplicates` | aggregation_ops | Dedupe removes duplicates |
| `test_aggregation_handles_empty_results` | aggregation_ops | Empty results handled |
| `test_aggregation_order_extract_flatten_dedupe` | aggregation_ops | Aggregation ops chain correctly |
| `test_fetch_and_extract_pipeline` | pipelines | URL fetch → extract pipeline works |
| `test_browse_taxonomy_forwards` | pipelines | Category browsing forwarding works |
| `test_tool_call_with_extra_params_ignored` | error_handling | Extra params don't cause errors |
| `test_empty_query_returns_empty_or_error` | error_handling | Empty query handled gracefully |
| `test_scatter_gather_continues_on_partial_failure` | error_handling | Partial failures don't block |
| `test_scatter_gather_all_targets_fail` | error_handling | All failures return empty |
| `test_special_characters_in_query` | error_handling | Special chars handled |
| `test_unicode_in_query` | error_handling | Unicode handled |

### Skipped Tests (2)

| Test | Reason |
|------|--------|
| `test_web_research_schema` | Requires Exa API key |
| `test_multi_source_search_all_four` | Requires Exa API key |

### Failing Tests (17)

Failures fall into a few categories:

#### Category 1: Entity Service Response Format (4 tests)

Entity service returns `{"success": true, "entity": {...}}` but tests expect `id` at top level:

```python
# Test expects:
result["id"]
# Actual response:
{"success": true, "entity": {"id": "...", "name": "..."}}
```

**Affected tests:**
- `test_store_research_finding_creates_entity`
- `test_link_entities_creates_relation`
- `test_explore_entity_network_merge`
- `test_merge_combines_objects`

**Fix needed:** Update tests to navigate `result["entity"]["id"]` or add output transform.

#### Category 2: Expected Exceptions Not Raised (6 tests)

Tests expect Python exceptions but gateway returns MCP error responses:

- `test_unknown_tool_returns_error` - Gateway returns 500, test expects specific error message
- `test_missing_required_param_returns_error` - Returns success, validation happens in backend
- `test_invalid_param_type_returns_error` - Returns success, validation happens in backend
- `test_pipeline_error_on_first_step` - Empty URL doesn't raise exception
- `test_pipeline_first_step_failure_propagates` - Empty URL doesn't raise exception
- `test_entity_not_found_error` - Returns response, doesn't raise exception
- `test_create_relation_missing_entities` - Returns response, doesn't raise exception

**Fix needed:** Update test assertions to check `result.get("isError")` instead of expecting exceptions.

#### Category 3: Backend Errors (3 tests)

- `test_get_knowledge_for_topic_merge` - 500 Internal Server Error
- `test_get_knowledge_merge_multiple_sources` - 500 Internal Server Error
- `test_find_or_create_category_forwards` - Backend SQL error: "A LIMIT constraint is required"

#### Category 4: Data-Dependent (2 tests)

- `test_fetch_and_extract_data_flow` - HN URL extraction returned 0 URLs
- `test_arraymap_coalesce_fallback` - HuggingFace snippet field is None

#### Category 5: Backend Validation (1 test)

- `test_very_large_num_results_capped` - Backend validates max=30, returns error instead of capping

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  pytest test session                                    │
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │ ServiceManager (session-scoped fixture)             ││
│  │   - search_service (8001)                           ││
│  │   - fetch_service (8002)                            ││
│  │   - entity_service (8003)                           ││
│  │   - category_service (8004)                         ││
│  │   - tag_service (8005)                              ││
│  └─────────────────────────────────────────────────────┘│
│                           │                             │
│                           ▼                             │
│  ┌─────────────────────────────────────────────────────┐│
│  │ GatewayManager (session-scoped fixture)             ││
│  │   - agentgateway binary (port 3000)                 ││
│  │   - loads config.yaml + research_registry.json      ││
│  └─────────────────────────────────────────────────────┘│
│                           │                             │
│                           ▼                             │
│  ┌─────────────────────────────────────────────────────┐│
│  │ McpHttpClient (session-scoped fixture)              ││
│  │   - MCP session initialized once                    ││
│  │   - call_tool fixture wraps tool invocations        ││
│  └─────────────────────────────────────────────────────┘│
│                           │                             │
│                           ▼                             │
│  ┌─────────────────────────────────────────────────────┐│
│  │ Test Functions                                      ││
│  │   - Use call_tool(name, args) helper                ││
│  │   - Assert on response structure                    ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

## MCP Session Protocol

The tests use HTTP-based MCP:

1. **Initialize** (no session header):
   ```
   POST /mcp
   Content-Type: application/json
   Accept: application/json, text/event-stream

   {"jsonrpc":"2.0","id":"...","method":"initialize",...}
   ```
   Response header: `mcp-session-id: <uuid>`

2. **Tool calls** (include session header):
   ```
   POST /mcp
   Content-Type: application/json
   Accept: application/json, text/event-stream
   mcp-session-id: <uuid>

   {"jsonrpc":"2.0","id":"...","method":"tools/call","params":{...}}
   ```

3. **Response parsing**: SSE format with `data:` lines containing JSON-RPC.
