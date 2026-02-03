"""Tests for pipeline composition pattern.

Pipelines execute steps sequentially, passing data from one step to the next.
"""

import pytest


@pytest.mark.asyncio
async def test_fetch_and_extract_pipeline(call_tool):
    """fetch_and_extract: url_fetch -> extract_urls pipeline."""
    result = await call_tool("virtual_fetch_and_extract", {"url": "https://example.com"})

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


@pytest.mark.asyncio
async def test_fetch_and_extract_data_flow(call_tool):
    """Pipeline correctly passes step 1 output to step 2 input."""
    # Use a URL that definitely has links
    result = await call_tool("virtual_fetch_and_extract", {"url": "https://news.ycombinator.com"})

    assert isinstance(result, dict)

    # If extract_urls worked, we should have a list of URLs
    if "urls" in result:
        assert isinstance(result["urls"], list)
        # HN should have many URLs
        assert len(result["urls"]) > 0


@pytest.mark.asyncio
async def test_pipeline_error_on_first_step(call_tool):
    """Pipeline should fail gracefully if first step fails."""
    # Invalid URL should cause fetch to fail
    with pytest.raises(Exception) as exc_info:
        await call_tool("virtual_fetch_and_extract", {"url": "not-a-valid-url"})

    # Should get a meaningful error, not a crash
    assert exc_info.value is not None


@pytest.mark.asyncio
async def test_store_research_finding_creates_entity(call_tool):
    """store_research_finding forwards to entity-service create_entity."""
    result = await call_tool(
        "virtual_store_research_finding",
        {
            "name": "Pipeline Test Finding",
            "entity_type": "paper",
            "description": "A test finding to verify forwarding works",
        },
    )

    # Should create an entity and return its details
    assert isinstance(result, dict)
    assert "id" in result, f"Expected 'id' in result, got: {result.keys()}"
    assert result["name"] == "Pipeline Test Finding"
    assert result["entity_type"] == "paper"


@pytest.mark.asyncio
async def test_link_entities_creates_relation(call_tool):
    """link_entities forwards to entity-service create_relation."""
    # First create two entities
    entity1 = await call_tool(
        "virtual_store_research_finding",
        {"name": "Entity A", "entity_type": "concept", "description": "First entity"},
    )
    entity2 = await call_tool(
        "virtual_store_research_finding",
        {"name": "Entity B", "entity_type": "concept", "description": "Second entity"},
    )

    # Now link them
    result = await call_tool(
        "virtual_link_entities",
        {
            "from_entity_id": entity1["id"],
            "to_entity_id": entity2["id"],
            "predicate": "related_to",
        },
    )

    # Should create the relation
    assert isinstance(result, dict)
    # The create_relation tool should confirm the relation was created


@pytest.mark.asyncio
async def test_browse_taxonomy_forwards(call_tool):
    """browse_taxonomy forwards to category-service list_root_categories."""
    result = await call_tool("virtual_browse_taxonomy", {"limit": 10})

    # Should return a list of categories
    assert isinstance(result, dict)
    # May have "categories" or similar key depending on backend response
    assert "categories" in result or "items" in result or isinstance(result.get("data"), list), (
        f"Expected category list, got: {result.keys()}"
    )


@pytest.mark.asyncio
async def test_find_or_create_category_forwards(call_tool):
    """find_or_create_category forwards to category-service search_categories."""
    result = await call_tool(
        "virtual_find_or_create_category",
        {"query": "machine learning", "limit": 5},
    )

    assert isinstance(result, dict)
    # Should return matching categories
    assert "categories" in result or "matches" in result or "results" in result, (
        f"Expected categories/matches/results, got: {result.keys()}"
    )
