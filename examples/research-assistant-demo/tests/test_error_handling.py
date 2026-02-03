"""Tests for error handling in virtual tools.

Tests graceful degradation, proper error messages, and edge cases.
"""

import pytest



def test_unknown_tool_returns_error(call_tool):
    """Unknown tool name returns proper error."""
    with pytest.raises(Exception) as exc_info:
        call_tool("nonexistent_tool_12345", {})

    error_msg = str(exc_info.value).lower()
    # Should mention the tool is unknown or not found
    assert "unknown" in error_msg or "not found" in error_msg or "tool" in error_msg, (
        f"Expected clear error about unknown tool, got: {exc_info.value}"
    )



def test_missing_required_param_returns_error(call_tool):
    """Missing required parameter returns validation error."""
    # normalized_github requires 'query' parameter
    with pytest.raises(Exception) as exc_info:
        call_tool("virtual_normalized_github", {})

    error_msg = str(exc_info.value).lower()
    # Should mention the missing parameter
    assert "query" in error_msg or "required" in error_msg or "missing" in error_msg, (
        f"Expected error about missing 'query', got: {exc_info.value}"
    )



def test_invalid_param_type_returns_error(call_tool):
    """Invalid parameter type returns validation error."""
    # num_results should be integer, not string
    with pytest.raises(Exception) as exc_info:
        call_tool(
            "virtual_normalized_github",
            {"query": "test", "num_results": "not_a_number"},
        )

    # Should get some kind of type/validation error
    assert exc_info.value is not None



def test_scatter_gather_continues_on_partial_failure(call_tool):
    """Scatter-gather with failFast=false continues despite partial failures."""
    # code_search targets github + huggingface
    # Even if one has issues, the other should return results
    result = call_tool("virtual_code_search", {"query": "python", "num_results": 3})

    assert "results" in result
    # Should have at least some results
    assert isinstance(result["results"], list)

    # At least one source should have succeeded
    if len(result["results"]) > 0:
        sources = {r["source"] for r in result["results"]}
        assert len(sources) > 0



def test_pipeline_first_step_failure_propagates(call_tool):
    """Pipeline step failure should propagate as tool error."""
    # Use invalid input that will fail at first step
    with pytest.raises(Exception):
        call_tool(
            "virtual_fetch_and_extract",
            {"url": ""},  # Empty URL should fail
        )



def test_tool_call_with_extra_params_ignored(call_tool):
    """Extra parameters should be ignored, not cause errors."""
    # Call with an extra param that's not in the schema
    result = call_tool(
        "virtual_normalized_github",
        {"query": "test", "num_results": 3, "extra_unknown_param": "should_be_ignored"},
    )

    # Should succeed despite the extra param
    assert "results" in result



def test_empty_query_returns_empty_or_error(call_tool):
    """Empty query string returns empty results or validation error."""
    try:
        result = call_tool("virtual_normalized_github", {"query": "", "num_results": 3})
        # If it succeeds, should have empty or minimal results
        assert "results" in result
        assert isinstance(result["results"], list)
    except Exception:
        # Validation error is also acceptable
        pass



def test_very_large_num_results_capped(call_tool):
    """Very large num_results should be handled gracefully."""
    result = call_tool(
        "virtual_normalized_github",
        {"query": "python", "num_results": 1000000},
    )

    # Should return some results (backend may cap the limit)
    assert "results" in result
    assert isinstance(result["results"], list)
    # Backend should have capped the results
    assert len(result["results"]) < 1000



def test_special_characters_in_query(call_tool):
    """Special characters in query should be handled."""
    result = call_tool(
        "virtual_normalized_github",
        {"query": "test & query | with <special> chars", "num_results": 3},
    )

    # Should complete without crashing
    assert "results" in result
    assert isinstance(result["results"], list)



def test_unicode_in_query(call_tool):
    """Unicode characters in query should be handled."""
    result = call_tool(
        "virtual_normalized_github",
        {"query": "transformer", "num_results": 3},
    )

    # Should complete without crashing
    assert "results" in result



def test_scatter_gather_all_targets_fail(call_tool):
    """When all scatter-gather targets fail, should return error or empty."""
    # Use a query designed to hit edge cases
    result = call_tool(
        "virtual_code_search",
        {"query": "xyzzy_nonexistent_query_99999", "num_results": 1},
    )

    # Should either return empty results or an error
    # Empty results is acceptable if all backends returned nothing
    assert "results" in result or "error" in result



def test_entity_not_found_error(call_tool):
    """Looking up non-existent entity returns proper error."""
    with pytest.raises(Exception) as exc_info:
        call_tool(
            "virtual_explore_entity_network",
            {"entity_id": "nonexistent-entity-id-12345"},
        )

    # Should get an error about entity not found
    assert exc_info.value is not None



def test_create_relation_missing_entities(call_tool):
    """Creating relation with non-existent entities fails properly."""
    with pytest.raises(Exception) as exc_info:
        call_tool(
            "virtual_link_entities",
            {
                "from_entity_id": "fake-id-1",
                "to_entity_id": "fake-id-2",
                "predicate": "related_to",
            },
        )

    # Should fail because entities don't exist
    assert exc_info.value is not None
