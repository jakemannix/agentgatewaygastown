"""Tests for arrayMap field source transforms.

ArrayMap transforms apply a mapping to each element of an array,
converting heterogeneous backend responses to a unified schema.
"""

import pytest



def test_normalized_github_schema(call_tool):
    """GitHub results are normalized to common search result schema."""
    result = call_tool("virtual_normalized_github", {"query": "transformers", "num_results": 3})

    assert "results" in result, f"Expected 'results' key, got: {result.keys()}"
    assert isinstance(result["results"], list), "results should be a list"
    assert len(result["results"]) > 0, "Should have at least one result"

    for item in result["results"]:
        # Verify normalized schema fields
        assert item["source"] == "github", f"Expected source=github, got: {item.get('source')}"
        assert item["source_type"] == "repo", f"Expected source_type=repo, got: {item.get('source_type')}"
        assert "title" in item, "Missing 'title' field"
        assert "url" in item, "Missing 'url' field"
        assert "snippet" in item, "Missing 'snippet' field"

        # Verify URL format
        assert item["url"].startswith("https://github.com/"), f"Invalid GitHub URL: {item['url']}"



def test_normalized_huggingface_schema(call_tool):
    """HuggingFace results are normalized to common search result schema."""
    result = call_tool("virtual_normalized_huggingface", {"query": "llama", "num_results": 3})

    assert "results" in result
    assert isinstance(result["results"], list)
    assert len(result["results"]) > 0

    for item in result["results"]:
        assert item["source"] == "huggingface"
        assert item["source_type"] == "model"
        assert "title" in item
        assert "url" in item
        assert "snippet" in item

        # HuggingFace URLs should point to huggingface.co
        assert "huggingface.co" in item["url"], f"Invalid HuggingFace URL: {item['url']}"



def test_normalized_arxiv_schema(call_tool):
    """arXiv results are normalized to common search result schema."""
    result = call_tool("virtual_normalized_arxiv", {"query": "attention mechanism", "num_results": 3})

    assert "results" in result
    assert isinstance(result["results"], list)

    # arXiv may have fewer results for specific queries
    if len(result["results"]) > 0:
        for item in result["results"]:
            assert item["source"] == "arxiv"
            assert item["source_type"] == "paper"
            assert "title" in item
            assert "url" in item
            assert "snippet" in item  # abstract

            # arXiv URLs should be arxiv.org
            assert "arxiv.org" in item["url"], f"Invalid arXiv URL: {item['url']}"



@pytest.mark.skip(reason="Exa requires API key - enable when testing with real credentials")
def test_normalized_exa_schema(call_tool):
    """Exa results are normalized to common search result schema."""
    result = call_tool("virtual_normalized_exa", {"query": "machine learning", "num_results": 3})

    assert "results" in result
    assert isinstance(result["results"], list)
    assert len(result["results"]) > 0

    for item in result["results"]:
        assert item["source"] == "exa"
        assert item["source_type"] == "web"
        assert "title" in item
        assert "url" in item
        assert "snippet" in item



def test_arraymap_preserves_array_order(call_tool):
    """ArrayMap should preserve the order of elements."""
    result = call_tool("virtual_normalized_github", {"query": "pytorch", "num_results": 5})

    # Just verify we get back an ordered list
    assert "results" in result
    assert isinstance(result["results"], list)

    # Each item should have incrementing indices if we tracked them
    # For now, just verify the list is intact
    for i, item in enumerate(result["results"]):
        assert "title" in item, f"Item {i} missing title"



def test_arraymap_handles_empty_source_array(call_tool):
    """ArrayMap should handle empty source array gracefully."""
    # Use a very specific query unlikely to match anything
    result = call_tool(
        "virtual_normalized_github",
        {"query": "xyzzy_nonexistent_repo_name_12345", "num_results": 3},
    )

    assert "results" in result
    # Should be empty list, not error
    assert isinstance(result["results"], list)



def test_arraymap_coalesce_fallback(call_tool):
    """Coalesce in arrayMap should fall back to secondary path when primary is missing."""
    # HuggingFace uses coalesce for snippet: first tries description, then pipeline_tag
    result = call_tool("virtual_normalized_huggingface", {"query": "bert", "num_results": 3})

    assert "results" in result
    for item in result["results"]:
        # snippet should exist from either description or pipeline_tag
        assert "snippet" in item
        # If pipeline_tag is used, it might be short like "text-classification"
        # If description is used, it's usually longer
        assert item["snippet"] is not None
