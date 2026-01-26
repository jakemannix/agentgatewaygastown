"""Tag Service - Content tagging with categories.

This service manages content items (URLs, papers, repos) and their category tags.
Provides both storage and semantic search over tagged content.
"""

from typing import Annotated

from mcp.server.fastmcp import FastMCP
from pydantic import Field

from .database import get_db

mcp = FastMCP(
    name="tag-service",
    instructions="Content tagging service for associating URLs/content with categories",
)


@mcp.tool()
def register_content(
    url: Annotated[str, Field(description="URL of the content to register")],
    title: Annotated[str | None, Field(description="Title of the content")] = None,
    content_type: Annotated[str | None, Field(description="Type: 'paper', 'repo', 'article', 'model', 'dataset'")] = None,
    summary: Annotated[str | None, Field(description="Summary or description of the content")] = None,
    source: Annotated[str | None, Field(description="Source: 'arxiv', 'github', 'huggingface', 'web'")] = None,
    metadata: Annotated[dict | None, Field(description="Additional metadata")] = None,
) -> dict:
    """Register a content item for tagging.

    Creates or updates a content record that can be tagged with categories.
    The summary is embedded for semantic search.
    """
    db = get_db()
    result = db.create_or_update_content(
        url=url,
        title=title,
        content_type=content_type,
        summary=summary,
        source=source,
        metadata=metadata,
    )

    return {"success": True, "content": result}


@mcp.tool()
def get_content(
    content_id: Annotated[str | None, Field(description="Content ID to retrieve")] = None,
    url: Annotated[str | None, Field(description="URL to look up")] = None,
    include_tags: Annotated[bool, Field(description="Include the content's category tags")] = True,
) -> dict:
    """Get a content item by ID or URL.

    Retrieves content details, optionally including all category tags.
    """
    db = get_db()

    if not content_id and not url:
        return {"error": "Must provide either content_id or url", "found": False}

    content = db.get_content(content_id=content_id, url=url)

    if not content:
        return {"error": "Content not found", "found": False}

    result = {"found": True, "content": content}

    if include_tags:
        result["tags"] = db.get_content_tags(content["id"])

    return result


@mcp.tool()
def tag_content(
    content_id: Annotated[str, Field(description="ID of the content to tag")],
    category_id: Annotated[str, Field(description="ID of the category to tag with")],
    confidence: Annotated[float, Field(description="Confidence score for the tag", ge=0.0, le=1.0)] = 1.0,
    notes: Annotated[str | None, Field(description="Notes about why this tag was applied")] = None,
) -> dict:
    """Tag content with a category.

    Associates a content item with a category. If the tag already exists,
    updates the confidence and notes.
    """
    db = get_db()

    # Verify content exists
    content = db.get_content(content_id=content_id)
    if not content:
        return {"success": False, "error": f"Content not found: {content_id}"}

    result = db.tag_content(
        content_id=content_id,
        category_id=category_id,
        confidence=confidence,
        added_by="agent",
        notes=notes,
    )

    return {"success": True, "tag": result}


@mcp.tool()
def untag_content(
    content_id: Annotated[str, Field(description="ID of the content")],
    category_id: Annotated[str, Field(description="ID of the category tag to remove")],
) -> dict:
    """Remove a category tag from content.

    Removes the association between content and a category.
    """
    db = get_db()
    success = db.untag_content(content_id, category_id)

    return {
        "success": success,
        "message": "Tag removed" if success else "Tag not found",
    }


@mcp.tool()
def get_content_tags(
    content_id: Annotated[str, Field(description="ID of the content")],
) -> dict:
    """Get all category tags for a content item.

    Returns all categories the content is tagged with, ordered by confidence.
    """
    db = get_db()

    content = db.get_content(content_id=content_id)
    if not content:
        return {"error": f"Content not found: {content_id}", "found": False}

    tags = db.get_content_tags(content_id)

    return {
        "found": True,
        "content_id": content_id,
        "content_url": content["url"],
        "tags": tags,
        "total": len(tags),
    }


@mcp.tool()
def search_tagged_content(
    category_id: Annotated[str, Field(description="Category ID to search for")],
    limit: Annotated[int, Field(description="Maximum number of results", ge=1, le=100)] = 50,
) -> dict:
    """Find all content tagged with a category.

    Returns content items that have been tagged with the specified category.
    """
    db = get_db()
    results = db.search_by_category(category_id, limit=limit)

    return {
        "category_id": category_id,
        "total": len(results),
        "content": results,
    }


@mcp.tool()
def search_content(
    query: Annotated[str, Field(description="Search query")],
    source: Annotated[str | None, Field(description="Filter by source: 'arxiv', 'github', etc.")] = None,
    content_type: Annotated[str | None, Field(description="Filter by type: 'paper', 'repo', etc.")] = None,
    limit: Annotated[int, Field(description="Maximum number of results", ge=1, le=50)] = 20,
) -> dict:
    """Search tagged content using semantic similarity.

    Searches content summaries for items matching the query.
    """
    db = get_db()
    results = db.search_content(
        query=query,
        source=source,
        content_type=content_type,
        limit=limit,
    )

    return {
        "query": query,
        "filters": {"source": source, "content_type": content_type},
        "total": len(results),
        "results": results,
    }


@mcp.tool()
def bulk_tag(
    content_ids: Annotated[list[str], Field(description="List of content IDs to tag")],
    category_id: Annotated[str, Field(description="Category to tag all content with")],
    confidence: Annotated[float, Field(description="Confidence score", ge=0.0, le=1.0)] = 1.0,
) -> dict:
    """Tag multiple content items with a category.

    Efficiently tags a batch of content items with the same category.
    """
    db = get_db()
    results = []
    success_count = 0

    for content_id in content_ids:
        content = db.get_content(content_id=content_id)
        if content:
            tag_result = db.tag_content(
                content_id=content_id,
                category_id=category_id,
                confidence=confidence,
                added_by="agent",
            )
            results.append({"content_id": content_id, "success": True, "tag": tag_result})
            success_count += 1
        else:
            results.append({"content_id": content_id, "success": False, "error": "Content not found"})

    return {
        "total": len(content_ids),
        "success_count": success_count,
        "failure_count": len(content_ids) - success_count,
        "results": results,
    }


@mcp.tool()
def delete_content(
    content_id: Annotated[str, Field(description="ID of the content to delete")],
) -> dict:
    """Delete a content item and all its tags.

    Permanently removes a content record and its category associations.
    """
    db = get_db()
    success = db.delete_content(content_id)

    return {
        "success": success,
        "message": "Content deleted" if success else "Content not found",
    }


if __name__ == "__main__":
    import sys
    import os
    sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
    from mcp_tools.shared.http_runner import run_http_server
    run_http_server(mcp, default_port=8005)
