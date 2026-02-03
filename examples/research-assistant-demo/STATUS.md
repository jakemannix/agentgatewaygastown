# Research Assistant Demo - Current Status

*Last updated: 2026-02-02*

## Summary

**42 tests passing** (2 skipped for API key requirements).

Major feature: **`research_and_fetch` mega-tool** - demonstrates advanced composition with inline scatter-gather inside a pipeline.

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

## Minimal Toolset for Claude Code

For the research workflow:

| Tool | Purpose |
|------|---------|
| `research_and_fetch` | Search + context + fetch in one call |
| `create_entity` | Add to knowledge graph |
| `create_category` | Add new category |
| `create_relation` | Link entities |
| `tag_content` | Tag content with categories |

## Commands

```bash
# Run tests
cd examples/research-assistant-demo
source .env
uv run pytest tests/ -v

# Test specific tool
uv run pytest tests/test_scatter_gather.py::test_research_and_fetch_mega_tool -v -s
```
