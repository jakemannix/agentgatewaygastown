# Research Assistant Demo - Current Status

*Last updated: 2026-02-02*

## Summary

**42 tests passing** (2 skipped for API key requirements).

Major feature: **`research_and_fetch` mega-tool** - demonstrates advanced composition with inline scatter-gather inside a pipeline.

> **Note:** Only the **virtual composite tools** (`research_and_fetch`, `multi_source_search`, `normalized_*`, and the KG write tools) have been deeply tested. The raw backend tools exposed by the 5 MCP services (entity-service, category-service, tag-service, fetch-service, search-service) are functional but may have rough edges, inconsistent error handling, or incomplete features. When using Claude Code, stick to the recommended tools listed below.

## Mega-Tool: research_and_fetch

A single tool that:
1. **Scatter-gather** (parallel): entity_search + search_categories + multi_source_search
2. **Batch fetch**: Get actual page content from search result URLs
3. **Merge**: Combine into structured output

**Output format:**
```json
{
  "relevant_entities": [...],
  "relevant_categories": [...],
  "search_results": [
    {"title": "...", "url": "...", "source": "exa|arxiv|github|huggingface", ...}
  ],
  "fetched_content": [
    {"url": "...", "content": "...", "success": true}
  ]
}
```

**Key syntax learnings:**
- Inline patterns: `"operation": {"pattern": {"scatterGather": {...}}}`
- Constants: `"constant": 3000` (not `{"numberValue": 3000}`)

## Test Status

```
======================== 42 passed, 2 skipped in 25.58s ========================
```

### Skipped Tests (require API keys)
- `test_normalized_exa_schema` - Exa API key required (but we have it in .env!)
- `test_multi_source_search_all_four` - Marked skip for CI

## Claude Code Integration

### Recommended Tools

| Tool | Purpose |
|------|---------|
| `research_and_fetch` | **PRIMARY** - Search + KG context + fetch in one call |
| `multi_source_search` | Search-only (no fetch) for browsing results first |
| `create_entity` | Save to knowledge graph |
| `create_category` | Add taxonomy category |
| `create_relation` | Link entities |
| `tag_content` | Tag content with categories |

### Tool Visibility

Claude Code sees **all tools** - both virtual compositions and raw backend tools. This is because Claude Code doesn't set the `X-Agent-Name` / `X-Agent-Version` headers that the gateway uses for tool filtering.

**Current behavior:**
- Gateway exposes virtual tools (from registry) + all backend tools
- Claude Code sees 40+ tools when using the full registry
- We rely on clear descriptions to guide tool selection (e.g., "PRIMARY RESEARCH TOOL")

**Available filtering (not used by Claude Code):**
- Agents can send `X-Agent-Name` and `X-Agent-Version` HTTP headers
- Gateway scopes tool visibility based on caller identity (see `examples/pattern-demos/` for examples)
- Registry v2 supports declaring which tools each agent depends on

**Future option:** Default visibility for anonymous callers
- Could configure gateway to only expose virtual tools when no agent identity is provided
- Would hide raw backend tools from generic MCP clients like Claude Code
- Not implemented yet, but straightforward to add

## Commands

```bash
# Run tests
cd examples/research-assistant-demo
source .env
uv run pytest tests/ -v

# Test specific tool
uv run pytest tests/test_scatter_gather.py::test_research_and_fetch_mega_tool -v -s
```
