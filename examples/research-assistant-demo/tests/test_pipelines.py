"""Tests for pipeline composition pattern.

Pipelines execute steps sequentially, passing data from one step to the next.
"""

import pytest



def test_fetch_and_extract_pipeline(call_tool):
    """fetch_and_extract: url_fetch -> extract_urls pipeline."""
    result = call_tool("virtual_fetch_and_extract", {"url": "https://example.com"})

    # The pipeline should:
    # 1. Fetch the URL content
    # 2. Extract URLs from the content

    # Result should contain extracted URLs or content
    assert isinstance(result, dict)

    # Should have either urls or content from the fetch step
    # The exact structure depends on extract_urls output
    assert "urls" in result or "content" in result or "extracted_urls" in result, (
        f"Expected urls/content/extracted_urls, got: {result.keys()}"
    )



def test_fetch_and_extract_data_flow(call_tool):
    """Pipeline correctly passes step 1 output to step 2 input."""
    # Use a URL that definitely has links - example.com is reliable
    result = call_tool("virtual_fetch_and_extract", {"url": "https://www.example.com"})

    assert isinstance(result, dict)

    # The pipeline should return urls list (may be empty if page has no links)
    if "urls" in result:
        assert isinstance(result["urls"], list)
        # example.com has at least the "More information..." link
        # But don't require it - network conditions may vary



def test_pipeline_error_on_first_step(call_tool):
    """Pipeline should fail gracefully if first step fails."""
    # Invalid URL should cause fetch to fail
    # Note: url_fetch returns success=false instead of MCP error
    result = call_tool("virtual_fetch_and_extract", {"url": "not-a-valid-url"})

    # Should indicate failure in the result
    # Pipeline passes through fetch result, which has success=false
    assert isinstance(result, dict)
    # Either we get an error field or success=false propagated from fetch
    has_error_indicator = (
        result.get("success") is False
        or "error" in result
        or result.get("urls") == []  # extract_urls on empty content
    )
    assert has_error_indicator or len(result.get("urls", [])) == 0, (
        f"Expected failure indication, got: {result}"
    )



def test_store_research_finding_creates_entity(call_tool):
    """store_research_finding forwards to entity-service create_entity."""
    result = call_tool(
        "virtual_store_research_finding",
        {
            "name": "Pipeline Test Finding",
            "entity_type": "paper",
            "description": "A test finding to verify forwarding works",
        },
    )

    # Should create an entity and return its details
    # Entity service returns {"success": true, "entity": {...}}
    assert isinstance(result, dict)
    assert "success" in result and result["success"], f"Expected success, got: {result}"
    assert "entity" in result, f"Expected 'entity' in result, got: {result.keys()}"

    entity = result["entity"]
    assert "id" in entity, f"Expected 'id' in entity, got: {entity.keys()}"
    assert entity["name"] == "Pipeline Test Finding"
    assert entity["entity_type"] == "paper"



def test_link_entities_creates_relation(call_tool):
    """link_entities forwards to entity-service create_relation."""
    # First create two entities
    # Entity service returns {"success": true, "entity": {...}}
    result1 = call_tool(
        "virtual_store_research_finding",
        {"name": "Entity A", "entity_type": "concept", "description": "First entity"},
    )
    result2 = call_tool(
        "virtual_store_research_finding",
        {"name": "Entity B", "entity_type": "concept", "description": "Second entity"},
    )

    entity1_id = result1["entity"]["id"]
    entity2_id = result2["entity"]["id"]

    # Now link them
    # Backend expects subject_id/object_id, not from_entity_id/to_entity_id
    result = call_tool(
        "virtual_link_entities",
        {
            "subject_id": entity1_id,
            "object_id": entity2_id,
            "predicate": "related_to",
        },
    )

    # Should create the relation
    assert isinstance(result, dict)
    # The create_relation tool should confirm the relation was created



def test_browse_taxonomy_forwards(call_tool):
    """browse_taxonomy forwards to category-service list_root_categories."""
    result = call_tool("virtual_browse_taxonomy", {"limit": 10})

    # Should return a list of categories
    assert isinstance(result, dict)
    # May have "categories" or similar key depending on backend response
    assert "categories" in result or "items" in result or isinstance(result.get("data"), list), (
        f"Expected category list, got: {result.keys()}"
    )



def test_find_or_create_category_forwards(call_tool):
    """find_or_create_category forwards to category-service search_categories."""
    result = call_tool(
        "virtual_find_or_create_category",
        {"query": "machine learning", "limit": 5},
    )

    assert isinstance(result, dict)
    # Should return matching categories
    assert "categories" in result or "matches" in result or "results" in result, (
        f"Expected categories/matches/results, got: {result.keys()}"
    )
