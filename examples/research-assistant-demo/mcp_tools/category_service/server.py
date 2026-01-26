"""Category Service - Hierarchical taxonomy for research topics.

This service manages a tree of categories for organizing research content.
Categories can have parent-child relationships and are searchable via semantic similarity.
"""

from typing import Annotated

from mcp.server.fastmcp import FastMCP
from pydantic import Field

from .database import get_db

mcp = FastMCP(
    name="category-service",
    instructions="Hierarchical category taxonomy service with semantic search",
)


@mcp.tool()
def search_categories(
    query: Annotated[str, Field(description="Search query for finding relevant categories")],
    limit: Annotated[int, Field(description="Maximum number of results", ge=1, le=50)] = 10,
) -> dict:
    """Search for categories using semantic similarity.

    Finds categories whose names and descriptions are semantically similar to the query.
    Useful for finding appropriate categories to tag content with.
    """
    db = get_db()
    results = db.search_categories(query, limit=limit)

    return {
        "query": query,
        "total": len(results),
        "categories": results,
    }


@mcp.tool()
def get_category(
    category_id: Annotated[str, Field(description="The category ID to retrieve")],
    include_children: Annotated[bool, Field(description="Include direct children of this category")] = False,
) -> dict:
    """Get a category by its ID.

    Retrieves full details of a category including its ancestor chain.
    Optionally includes direct children.
    """
    db = get_db()
    category = db.get_category(category_id)

    if not category:
        return {"error": f"Category not found: {category_id}", "found": False}

    result = {"found": True, "category": category}

    if include_children:
        result["children"] = db.get_children(category_id)

    return result


@mcp.tool()
def create_category(
    name: Annotated[str, Field(description="Name of the category")],
    parent_id: Annotated[str | None, Field(description="ID of the parent category (null for root)")] = None,
    description: Annotated[str | None, Field(description="Description of what this category covers")] = None,
    properties: Annotated[dict | None, Field(description="Additional properties")] = None,
) -> dict:
    """Create a new category in the taxonomy.

    Creates a category as either a root category or as a child of an existing category.
    The category's name and description are embedded for semantic search.
    """
    db = get_db()

    # Validate parent exists if provided
    if parent_id:
        parent = db.get_category(parent_id)
        if not parent:
            return {"success": False, "error": f"Parent category not found: {parent_id}"}

    category = db.create_category(
        name=name,
        parent_id=parent_id,
        description=description,
        properties=properties,
    )

    return {"success": True, "category": category}


@mcp.tool()
def update_category(
    category_id: Annotated[str, Field(description="ID of the category to update")],
    name: Annotated[str | None, Field(description="New name for the category")] = None,
    description: Annotated[str | None, Field(description="New description for the category")] = None,
    properties: Annotated[dict | None, Field(description="New or updated properties")] = None,
) -> dict:
    """Update an existing category.

    Updates the specified fields of a category. Note: parent cannot be changed
    to prevent circular references.
    """
    db = get_db()
    category = db.update_category(
        category_id=category_id,
        name=name,
        description=description,
        properties=properties,
    )

    if not category:
        return {"success": False, "error": f"Category not found: {category_id}"}

    return {"success": True, "category": category}


@mcp.tool()
def delete_category(
    category_id: Annotated[str, Field(description="ID of the category to delete")],
    recursive: Annotated[bool, Field(description="Also delete all descendant categories")] = False,
) -> dict:
    """Delete a category from the taxonomy.

    By default, fails if the category has children. Set recursive=true to
    delete the category and all its descendants.
    """
    db = get_db()
    result = db.delete_category(category_id, recursive=recursive)
    return result


@mcp.tool()
def get_category_tree(
    root_id: Annotated[str | None, Field(description="Root category ID (null for entire tree)")] = None,
    max_depth: Annotated[int, Field(description="Maximum depth to traverse", ge=1, le=20)] = 10,
) -> dict:
    """Get the category tree structure.

    Returns a hierarchical tree of categories starting from the specified root
    or the entire tree if no root is specified.
    """
    db = get_db()
    tree = db.get_tree(root_id=root_id, max_depth=max_depth)
    return {"tree": tree}


@mcp.tool()
def list_root_categories() -> dict:
    """List all root-level categories.

    Returns categories that have no parent (top-level taxonomy entries).
    """
    db = get_db()
    roots = db.get_children(None)
    return {"categories": roots, "total": len(roots)}


@mcp.tool()
def get_category_path(
    category_id: Annotated[str, Field(description="Category ID to get path for")],
) -> dict:
    """Get the full path from root to a category.

    Returns the chain of categories from the root to the specified category,
    useful for breadcrumb navigation.
    """
    db = get_db()
    category = db.get_category(category_id)

    if not category:
        return {"error": f"Category not found: {category_id}", "found": False}

    # Build full path including the category itself
    path = category.get("ancestors", []) + [{"id": category["id"], "name": category["name"]}]

    return {
        "found": True,
        "category_id": category_id,
        "category_name": category["name"],
        "path": path,
        "depth": len(path),
    }


if __name__ == "__main__":
    import sys
    import os
    sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
    from mcp_tools.shared.http_runner import run_http_server
    run_http_server(mcp, default_port=8004)
