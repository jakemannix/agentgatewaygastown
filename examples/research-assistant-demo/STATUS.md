# Research Assistant Demo - Current Status

*Last updated: 2026-02-02*

## Summary

Integration tests improved from 24→32 passing (out of 43 total, 2 skipped). Key fixes:
- `McpToolError` exception class for `isError: true` responses
- Entity response format handling (`result["entity"]["id"]`)
- Parameter name fixes (`subject_id`/`object_id` vs `from_entity_id`/`to_entity_id`)
- Test expectation fixes (backend validation is correct behavior)

## Current Test Status

**32 passed, 9 failed, 2 skipped**

### Remaining Failures

| Category | Test | Issue |
|----------|------|-------|
| Gateway 500 | `test_get_knowledge_for_topic_merge` | Virtual tool composition error |
| Gateway 500 | `test_get_knowledge_merge_multiple_sources` | Same virtual tool |
| Backend | `test_find_or_create_category_forwards` | SQLite vec0 query needs LIMIT |
| Backend | `test_entity_not_found_error` | Entity service doesn't error on missing |
| Data | `test_fetch_and_extract_data_flow` | HN URL extraction returns empty |
| Data | `test_arraymap_coalesce_fallback` | HuggingFace snippet is None |
| Error propagation | `test_unknown_tool_returns_error` | Gateway returns HTTP 500 not MCP error |
| Error propagation | `test_pipeline_first_step_failure_propagates` | Empty URL doesn't fail |
| Error propagation | `test_pipeline_error_on_first_step` | Invalid URL doesn't fail |

## Next Steps

1. **Debug `get_knowledge_for_topic` virtual tool** - Gateway 500 errors
2. **Fix category service** - Add LIMIT to vec0 queries
3. **Improve error propagation** - Gateway should return MCP errors not HTTP 500
4. **Make data tests resilient** - Handle empty/null responses gracefully

## Commands Reference

```bash
# Run Python integration tests
cd examples/research-assistant-demo
uv run pytest tests/ -v

# Run single test
uv run pytest tests/test_scatter_gather.py::test_code_search_combines_sources -v

# Run Rust tests
cargo test -p agentgateway
```
