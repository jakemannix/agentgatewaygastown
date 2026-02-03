# Research Assistant Demo - Current Status

*Last updated: 2026-02-02*

## Summary

Working on virtual tool compositions with MCP-compliant responses. Main scatter-gather response format issue is FIXED. Integration tests improved from 11→24 passing.

## Git Status

### Already Committed (this session)
```
28cf176 fix(registry): Add outputSchema and wrap op for MCP-compliant responses
```

### Unstaged Rust Changes (NEED TO COMMIT - all tests pass)

These changes support the virtual tool infrastructure:

| File | Changes |
|------|---------|
| `handler.rs` | +81 lines - composition detection and routing |
| `executor/mod.rs` | +68 lines - MockToolInvoker for testing |
| `patterns/scatter_gather.rs` | +134 lines - WrapOp, ExtractOp definitions |
| `types.rs` | +35 lines - new type definitions |
| `compiled.rs`, `execution_graph.rs`, `types_compat.rs` | Minor changes |
| `tests/registry.rs` | +6 lines - import updates |

**Status:** All 58 Rust unit tests pass, 6 ignored, 0 failed.

### Unstaged Python Test Changes (NEED TO COMMIT)

| File | Changes |
|------|---------|
| `conftest.py` | +176/-X - Sync HTTP MCP client, session management |
| `test_*.py` (5 files) | Converted from async to sync, updated assertions |

**Status:** 24 passed, 17 failed, 2 skipped

### Unstaged Demo App Changes (REVIEW BEFORE COMMIT)

| File | Purpose |
|------|---------|
| `agents/research_agent/__init__.py` | Agent refactoring |
| `agents/research_agent/__main__.py` | ADK integration |
| `agents/shared/a2a_server.py` | A2A server updates |
| `gateway-configs/config.yaml` | Config changes |
| `mcp_tools/search_service/server.py` | Backend service |
| `start_services.sh` | Startup script |
| `web_ui/chat_app.py` | Web UI |

### Untracked Files (REVIEW - may need .gitignore)

- `agentgateway` - binary, should be ignored
- `sessions.db` - SQLite DB, should be ignored
- `touch` - temp file, delete
- `.adk/` - ADK cache, should be ignored
- `minimal_*.json/yaml` - test configs, decide if needed
- `REGISTRY_FORMAT.md` - documentation, maybe commit

## Integration Test Failures - Root Causes

### Category 1: Entity Service Response Format (4 tests)

**Problem:** Entity service returns `{"success": true, "entity": {...}}` but tests expect `result["id"]`

**Affected tests:**
- `test_store_research_finding_creates_entity`
- `test_link_entities_creates_relation`
- `test_explore_entity_network_merge`
- `test_merge_combines_objects`

**Fix options:**
1. Update tests to use `result["entity"]["id"]`
2. Add `outputTransform` to virtual tools to flatten response

### Category 2: Exception Handling (6 tests)

**Problem:** Tests expect Python exceptions but gateway returns MCP error responses with `isError: true`

**Affected tests:**
- `test_unknown_tool_returns_error`
- `test_missing_required_param_returns_error`
- `test_invalid_param_type_returns_error`
- `test_pipeline_error_on_first_step`
- `test_entity_not_found_error`
- `test_create_relation_missing_entities`

**Fix:** Update `McpHttpClient._parse_response()` in conftest.py to raise exception when `isError: true`

### Category 3: Backend Errors (3 tests)

**Problem:** Actual 500 errors from gateway or backend services

**Affected tests:**
- `test_get_knowledge_for_topic_merge` - 500 error
- `test_get_knowledge_merge_multiple_sources` - 500 error
- `test_find_or_create_category_forwards` - SQL error: "A LIMIT constraint is required"

**Fix:** Debug the virtual tools `get_knowledge_for_topic` and `find_or_create_category` in registry

### Category 4: Data-Dependent (2 tests)

- `test_fetch_and_extract_data_flow` - HN returned 0 URLs (network/parsing issue)
- `test_arraymap_coalesce_fallback` - HuggingFace snippet is None

**Fix:** Make tests more resilient or use mock data

### Category 5: Backend Validation (1 test)

- `test_very_large_num_results_capped` - Backend rejects num_results > 30

**Fix:** Update test expectation (error is correct behavior)

## Next Steps (Priority Order)

1. **Commit Rust changes** - All tests pass, safe to commit
2. **Commit Python test infrastructure** - conftest.py and test files
3. **Fix exception handling in conftest.py** - Make `isError: true` raise exception
4. **Fix entity response tests** - Update to use `result["entity"]["id"]`
5. **Debug 500 errors** - Investigate `get_knowledge_for_topic` virtual tool
6. **Clean up .gitignore** - Add binaries, DBs, cache dirs

## Commands Reference

```bash
# Run Rust tests
cargo test -p agentgateway

# Run Python integration tests
cd examples/research-assistant-demo
uv run pytest tests/ -v

# Run registry validator
python scripts/validate-registry-schemas.py examples/research-assistant-demo/gateway-configs/research_registry.json

# Start demo services manually
cd examples/research-assistant-demo
./start_services.sh
```
