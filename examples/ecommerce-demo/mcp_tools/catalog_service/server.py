#!/usr/bin/env python3
"""Catalog MCP service for product management and semantic search."""

import sys
from pathlib import Path
from typing import Optional

from mcp.server.fastmcp import FastMCP
from pydantic import Field
from sentence_transformers import SentenceTransformer

from .database import CatalogDatabase

# Initialize FastMCP server
mcp = FastMCP(
    name="Catalog Service",
    instructions="Product catalog management with semantic search capabilities",
    host="0.0.0.0",
    port=8001,
)

# Lazy-loaded globals
_db: Optional[CatalogDatabase] = None
_model: Optional[SentenceTransformer] = None


def get_db() -> CatalogDatabase:
    """Get or create database instance."""
    global _db
    if _db is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
        _db = CatalogDatabase(data_dir)
    return _db


def get_model() -> SentenceTransformer:
    """Get or create embedding model instance."""
    global _model
    if _model is None:
        _model = SentenceTransformer("all-MiniLM-L6-v2")
    return _model


@mcp.tool()
def search_products(
    query: str = Field(description="Natural language search query"),
    category: Optional[str] = Field(default=None, description="Filter by category"),
    max_results: int = Field(default=10, description="Maximum number of results"),
) -> dict:
    """
    Search products using semantic similarity.

    Uses vector embeddings to find products matching the natural language query.
    Results are ranked by relevance.
    """
    model = get_model()
    db = get_db()

    # Generate embedding for query
    query_embedding = model.encode(query).tolist()

    # Search
    results = db.search_products(
        query_embedding=query_embedding,
        category=category,
        max_results=max_results,
    )

    # Format results for customer view (hide cost)
    products = []
    for r in results:
        products.append({
            "id": r["id"],
            "name": r["name"],
            "description": r["description"],
            "price": r["price"],
            "category": r["category"],
            "in_stock": r["stock_quantity"] > 0,
            "relevance_score": 1.0 - r.get("distance", 0),  # Convert distance to similarity
        })

    return {
        "query": query,
        "results_count": len(products),
        "products": products,
    }


@mcp.tool()
def get_product(
    product_id: str = Field(description="The product ID to retrieve"),
) -> dict:
    """
    Get detailed information about a specific product.

    Returns full product details including description and availability.
    """
    db = get_db()
    product = db.get_product(product_id)

    if not product:
        return {"error": f"Product not found: {product_id}"}

    return {
        "id": product["id"],
        "name": product["name"],
        "description": product["description"],
        "price": product["price"],
        "cost": product["cost"],  # Include for internal use
        "category": product["category"],
        "stock_quantity": product["stock_quantity"],
        "reorder_threshold": product["reorder_threshold"],
        "in_stock": product["stock_quantity"] > 0,
    }


@mcp.tool()
def list_products(
    category: Optional[str] = Field(default=None, description="Filter by category"),
    page: int = Field(default=1, description="Page number (1-based)"),
    limit: int = Field(default=20, description="Products per page"),
) -> dict:
    """
    Browse the product catalog with optional category filtering.

    Returns a paginated list of products.
    """
    db = get_db()
    products = db.list_products(category=category, page=page, limit=limit)

    # Format for customer view
    formatted = []
    for p in products:
        formatted.append({
            "id": p["id"],
            "name": p["name"],
            "description": p["description"],
            "price": p["price"],
            "category": p["category"],
            "in_stock": p["stock_quantity"] > 0,
        })

    return {
        "page": page,
        "limit": limit,
        "category": category,
        "count": len(formatted),
        "products": formatted,
    }


@mcp.tool()
def get_categories() -> dict:
    """
    Get all available product categories.

    Returns a list of category names that can be used to filter products.
    """
    db = get_db()
    categories = db.get_categories()
    return {
        "categories": categories,
        "count": len(categories),
    }


@mcp.tool()
def get_product_internal(
    product_id: str = Field(description="The product ID to retrieve"),
) -> dict:
    """
    Get full product information including cost (internal use only).

    This tool is for backend/merchandiser use - includes cost data.
    """
    db = get_db()
    product = db.get_product(product_id)

    if not product:
        return {"error": f"Product not found: {product_id}"}

    return product


@mcp.tool()
def get_inventory_stats() -> dict:
    """
    Get inventory statistics (merchandiser view).

    Returns summary statistics including total value, low stock counts.
    """
    db = get_db()
    return db.get_inventory_stats()


def main():
    """Run the MCP server with HTTP transport."""
    mcp.run(transport="streamable-http")


if __name__ == "__main__":
    main()
