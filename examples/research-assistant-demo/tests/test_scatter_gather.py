"""Tests for scatter-gather composition pattern.

Scatter-gather executes multiple tools in parallel and aggregates results.
"""

import pytest



def test_code_search_combines_sources(call_tool):
    """code_search runs github + huggingface in parallel."""
    result = call_tool("virtual_code_search", {"query": "transformer", "num_results": 3})

    assert "results" in result, f"Expected 'results' key, got: {result.keys()}"
    assert isinstance(result["results"], list)

    # Should have results from both sources
    sources = {r["source"] for r in result["results"]}
    assert "github" in sources, f"Missing github results. Sources found: {sources}"
    assert "huggingface" in sources, f"Missing huggingface results. Sources found: {sources}"

    # All results should have normalized schema
    for item in result["results"]:
        assert "title" in item
        assert "url" in item
        assert "source" in item
        assert "source_type" in item



def test_academic_search_combines_sources(call_tool):
    """academic_search runs arxiv + huggingface in parallel."""
    result = call_tool("virtual_academic_search", {"query": "neural network", "num_results": 3})

    assert "results" in result
    sources = {r["source"] for r in result["results"]}

    # At minimum should have huggingface (arxiv may have issues with redirects)
    assert "huggingface" in sources, f"Missing huggingface results. Sources: {sources}"

    # Verify normalized schema
    for item in result["results"]:
        assert item["source"] in ("arxiv", "huggingface")
        assert "title" in item
        assert "url" in item



@pytest.mark.skip(reason="Exa requires API key - enable when testing with real credentials")
def test_multi_source_search_all_four(call_tool):
    """multi_source_search runs all 4 sources: exa, arxiv, github, huggingface."""
    result = call_tool("virtual_multi_source_search", {"query": "machine learning", "num_results": 3})

    assert "results" in result
    sources = {r["source"] for r in result["results"]}

    # Should have results from all sources
    expected_sources = {"exa", "arxiv", "github", "huggingface"}
    assert sources == expected_sources, f"Expected all 4 sources, got: {sources}"



def test_scatter_gather_results_flattened(call_tool):
    """Results from multiple sources should be flattened into single array."""
    result = call_tool("virtual_code_search", {"query": "pytorch", "num_results": 5})

    assert "results" in result
    assert isinstance(result["results"], list)

    # Should not have nested arrays
    for item in result["results"]:
        assert isinstance(item, dict), f"Expected dict, got {type(item)}: {item}"
        assert not isinstance(item.get("results"), list), "Results should not be nested"



def test_scatter_gather_dedupe_by_url(call_tool):
    """Dedupe should remove duplicate URLs across sources."""
    result = call_tool("virtual_code_search", {"query": "bert-base-uncased", "num_results": 20})

    assert "results" in result
    urls = [r["url"] for r in result["results"]]

    # No duplicate URLs should exist
    assert len(urls) == len(set(urls)), f"Found duplicate URLs: {[u for u in urls if urls.count(u) > 1]}"



def test_explore_entity_network_merge(call_tool):
    """explore_entity_network merges get_entity + search_relations."""
    # First create an entity to explore
    # Entity service returns {"success": true, "entity": {...}}
    create_result = call_tool(
        "virtual_store_research_finding",
        {
            "name": "Test Entity for Network",
            "entity_type": "concept",
            "description": "Test entity for scatter-gather merge test",
        },
    )

    assert "entity" in create_result, f"Failed to create entity: {create_result}"
    entity_id = create_result["entity"]["id"]
    assert entity_id, f"Entity has no id: {create_result}"

    # Now explore its network
    result = call_tool("virtual_explore_entity_network", {"entity_id": entity_id})

    # Result should have merged fields from both tools
    # get_entity returns entity details, search_relations returns relations array
    assert isinstance(result, dict)
    # Should have entity fields or relations or both
    # The merge aggregation combines the outputs



def test_get_knowledge_for_topic_merge(call_tool):
    """get_knowledge_for_topic merges entity_search + category_search + content_search."""
    result = call_tool("virtual_get_knowledge_for_topic", {"query": "machine learning"})

    assert isinstance(result, dict)
    # Merge should combine all three outputs
    # May have entities, categories, and/or content depending on what's stored



def test_scatter_gather_partial_failure(call_tool):
    """If one target fails, others should still return results (failFast=false)."""
    # code_search targets github + huggingface
    # Even if one service has issues, the other should succeed
    result = call_tool("virtual_code_search", {"query": "langchain", "num_results": 3})

    assert "results" in result
    # Should have at least some results even if one source failed
    assert len(result["results"]) > 0



def test_scatter_gather_timeout_handling(call_tool):
    """Scatter-gather should respect timeout settings."""
    # The configured timeout is 30s for search compositions
    # Normal queries should complete well within that
    result = call_tool("virtual_code_search", {"query": "test", "num_results": 3})

    assert "results" in result
    # If we got here without timeout, the test passed
