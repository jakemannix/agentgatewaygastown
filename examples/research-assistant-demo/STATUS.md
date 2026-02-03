# Research Assistant Demo - Current Status

*Last updated: 2026-02-02*

## Summary

**All 41 integration tests now pass** (2 skipped for API key requirements).

Key fixes made this session:
- `McpToolError` exception class for `isError: true` MCP responses
- Entity response format handling (`result["entity"]["id"]`)
- Parameter name mapping (`subject_id`/`object_id` vs `from_entity_id`/`to_entity_id`)
- sqlite-vec vec0 KNN query syntax (`k = ?` constraint)
- Test expectations aligned with actual backend behavior patterns

## Test Status

```
======================== 41 passed, 2 skipped in 24.09s ========================
```

### Skipped Tests (require API keys)
- `test_normalized_exa_schema` - Exa API key required
- `test_multi_source_search_all_four` - Exa API key required

## Architecture Notes

### Backend Response Patterns

The backend services use two patterns for error handling:

1. **MCP-level errors** (`isError: true`): Used for validation errors, missing required params
2. **Application-level success/failure** (`success: false`): Used by url_fetch for network failures

The test infrastructure handles both via `McpToolError` for (1) and result inspection for (2).

### sqlite-vec KNN Queries

The vec0 extension requires `k = ?` in WHERE clause for KNN queries:
```sql
WHERE description_embedding MATCH ? AND k = ?
```
Not just `LIMIT ?` at the end.

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
