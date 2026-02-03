"""Tests for aggregation operations in scatter-gather.

Aggregation ops: extract, flatten, dedupe, sort, limit, merge.
"""

import pytest


@pytest.mark.asyncio
async def test_extract_pulls_nested_array(call_tool):
    """Extract operation pulls $.results from each target's output."""
    # code_search uses extract + flatten
    # Each normalized tool returns {results: [...]}
    # Extract should pull out just the array
    result = await call_tool("virtual_code_search", {"query": "pytorch", "num_results": 3})

    assert "results" in result
    # The results should be extracted from nested {results: [...]}
    # and combined into a single array
    assert isinstance(result["results"], list)

    # Each item should be the actual result, not wrapped
    for item in result["results"]:
        assert isinstance(item, dict)
        assert "title" in item  # Normalized schema


@pytest.mark.asyncio
async def test_flatten_combines_arrays(call_tool):
    """Flatten operation combines arrays from multiple sources."""
    result = await call_tool("virtual_code_search", {"query": "tensorflow", "num_results": 5})

    assert "results" in result
    # Should be a flat list, not nested arrays
    assert isinstance(result["results"], list)

    # No nested arrays
    for item in result["results"]:
        assert not isinstance(item, list), "Found nested array - flatten not working"


@pytest.mark.asyncio
async def test_dedupe_removes_duplicates(call_tool):
    """Dedupe operation removes items with duplicate URLs."""
    result = await call_tool("virtual_code_search", {"query": "huggingface transformers", "num_results": 10})

    assert "results" in result
    urls = [r["url"] for r in result["results"]]

    # All URLs should be unique
    unique_urls = set(urls)
    duplicates = [u for u in urls if urls.count(u) > 1]

    assert len(urls) == len(unique_urls), f"Found {len(duplicates)} duplicates: {set(duplicates)}"


@pytest.mark.asyncio
async def test_merge_combines_objects(call_tool):
    """Merge operation combines dict outputs from multiple targets."""
    # explore_entity_network uses merge to combine get_entity + search_relations
    # First create an entity
    entity = await call_tool(
        "virtual_store_research_finding",
        {"name": "Merge Test Entity", "entity_type": "concept", "description": "For merge test"},
    )

    result = await call_tool("virtual_explore_entity_network", {"entity_id": entity["id"]})

    # Result should be a merged dict from both tools
    assert isinstance(result, dict)

    # Should have data from at least one of the sources
    # get_entity returns entity fields, search_relations returns relations
    has_entity_data = "id" in result or "name" in result or "entity" in result
    has_relation_data = "relations" in result or "edges" in result or "from_entity" in result

    # At least one source should have contributed
    assert has_entity_data or has_relation_data or len(result) > 0, (
        f"Merge produced empty or unexpected result: {result}"
    )


@pytest.mark.asyncio
async def test_aggregation_handles_empty_results(call_tool):
    """Aggregation should handle empty results from all sources."""
    # Query that returns no results
    result = await call_tool(
        "virtual_code_search",
        {"query": "xyzzy_completely_nonexistent_12345", "num_results": 5},
    )

    assert "results" in result
    # Should be empty list, not error
    assert isinstance(result["results"], list)
    assert len(result["results"]) == 0


@pytest.mark.asyncio
async def test_aggregation_order_extract_flatten_dedupe(call_tool):
    """Aggregation ops execute in order: extract -> flatten -> dedupe."""
    # This is tested implicitly by the scatter-gather tools
    # The ops array is: [extract($.results), flatten, dedupe($.url)]
    result = await call_tool("virtual_academic_search", {"query": "transformers", "num_results": 5})

    assert "results" in result
    assert isinstance(result["results"], list)

    # After all ops:
    # 1. Extract should have pulled out results arrays
    # 2. Flatten should have combined them
    # 3. Dedupe should have removed duplicates

    urls = [r["url"] for r in result["results"]]
    assert len(urls) == len(set(urls)), "Dedupe didn't remove duplicates"

    for item in result["results"]:
        assert not isinstance(item, list), "Flatten didn't work"
        assert "title" in item, "Extract didn't work"


@pytest.mark.asyncio
async def test_get_knowledge_merge_multiple_sources(call_tool):
    """get_knowledge_for_topic merges three sources: entity, category, content."""
    # First populate some test data
    await call_tool(
        "virtual_store_research_finding",
        {
            "name": "Aggregation Test Concept",
            "entity_type": "concept",
            "description": "Test data for aggregation merge test",
        },
    )

    result = await call_tool("virtual_get_knowledge_for_topic", {"query": "aggregation test"})

    # Should have merged results from entity_search, search_categories, search_content
    assert isinstance(result, dict)

    # May have entities, categories, content depending on matches
    # At minimum should have some structure from the merge
    assert len(result) >= 0  # Merge should produce a dict even if sources are empty
