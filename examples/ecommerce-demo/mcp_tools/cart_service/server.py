#!/usr/bin/env python3
"""Cart MCP service for shopping cart management."""

from pathlib import Path
from typing import Optional

from mcp.server.fastmcp import FastMCP
from pydantic import Field

from .database import CartDatabase
from ..catalog_service.database import CatalogDatabase

# Initialize FastMCP server
mcp = FastMCP(
    name="Cart Service",
    instructions="Shopping cart management for ecommerce",
    host="0.0.0.0",
    port=8002,
)

# Lazy-loaded globals
_db: Optional[CartDatabase] = None
_catalog_db: Optional[CatalogDatabase] = None


def get_db() -> CartDatabase:
    """Get or create database instance."""
    global _db
    if _db is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
        _db = CartDatabase(data_dir)
    return _db


def get_catalog_db() -> CatalogDatabase:
    """Get or create catalog database instance."""
    global _catalog_db
    if _catalog_db is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
        _catalog_db = CatalogDatabase(data_dir)
    return _catalog_db


def enrich_cart_items(items: list[dict]) -> list[dict]:
    """Add product details to cart items."""
    catalog_db = get_catalog_db()
    enriched = []
    for item in items:
        product = catalog_db.get_product(item["product_id"])
        enriched.append({
            "id": item["id"],
            "product_id": item["product_id"],
            "product_name": product["name"] if product else "Unknown",
            "product_price": product["price"] if product else 0,
            "quantity": item["quantity"],
            "line_total": (product["price"] if product else 0) * item["quantity"],
        })
    return enriched


@mcp.tool()
def get_or_create_cart(
    user_id: str = Field(description="User identifier"),
) -> dict:
    """
    Get the active cart for a user, or create one if none exists.

    Returns cart ID and status.
    """
    db = get_db()
    cart = db.get_or_create_cart(user_id)
    return {
        "cart_id": cart["id"],
        "user_id": cart["user_id"],
        "status": cart["status"],
        "created_at": cart["created_at"],
    }


@mcp.tool()
def add_to_cart(
    user_id: str = Field(description="User identifier"),
    product_id: str = Field(description="Product to add"),
    quantity: int = Field(default=1, description="Quantity to add"),
) -> dict:
    """
    Add a product to the user's cart.

    If the product is already in the cart, the quantity is increased.
    """
    db = get_db()
    catalog_db = get_catalog_db()

    # Check product exists and has stock
    product = catalog_db.get_product(product_id)
    if not product:
        return {"error": f"Product not found: {product_id}"}

    if product["stock_quantity"] < quantity:
        return {
            "error": f"Insufficient stock. Available: {product['stock_quantity']}",
            "available_quantity": product["stock_quantity"],
        }

    # Get or create cart
    cart = db.get_or_create_cart(user_id)

    # Add item
    item = db.add_item(cart["id"], product_id, quantity)

    return {
        "success": True,
        "cart_id": cart["id"],
        "item": {
            "id": item["id"],
            "product_id": item["product_id"],
            "product_name": product["name"],
            "quantity": item["quantity"],
            "unit_price": product["price"],
            "line_total": product["price"] * item["quantity"],
        },
    }


@mcp.tool()
def update_cart_item(
    cart_item_id: str = Field(description="Cart item ID to update"),
    quantity: int = Field(description="New quantity (0 to remove)"),
) -> dict:
    """
    Update the quantity of an item in the cart.

    Set quantity to 0 to remove the item.
    """
    db = get_db()
    result = db.update_item_quantity(cart_item_id, quantity)

    if not result:
        return {"error": f"Cart item not found: {cart_item_id}"}

    if result.get("deleted"):
        return {"success": True, "message": "Item removed from cart"}

    # Enrich with product details
    catalog_db = get_catalog_db()
    product = catalog_db.get_product(result["product_id"])

    return {
        "success": True,
        "item": {
            "id": result["id"],
            "product_id": result["product_id"],
            "product_name": product["name"] if product else "Unknown",
            "quantity": result["quantity"],
            "unit_price": product["price"] if product else 0,
            "line_total": (product["price"] if product else 0) * result["quantity"],
        },
    }


@mcp.tool()
def remove_from_cart(
    cart_item_id: str = Field(description="Cart item ID to remove"),
) -> dict:
    """
    Remove an item from the cart entirely.
    """
    db = get_db()
    success = db.remove_item(cart_item_id)

    if not success:
        return {"error": f"Cart item not found: {cart_item_id}"}

    return {"success": True, "message": "Item removed from cart"}


@mcp.tool()
def view_cart(
    user_id: str = Field(description="User identifier"),
) -> dict:
    """
    View the contents of a user's cart with totals.

    Returns all items with product details and cart total.
    """
    db = get_db()

    # Get or create cart
    cart = db.get_or_create_cart(user_id)
    items = db.get_cart_items(cart["id"])

    # Enrich items with product details
    enriched_items = enrich_cart_items(items)

    # Calculate total
    total = sum(item["line_total"] for item in enriched_items)

    return {
        "cart_id": cart["id"],
        "user_id": user_id,
        "status": cart["status"],
        "item_count": len(enriched_items),
        "items": enriched_items,
        "total": round(total, 2),
    }


@mcp.tool()
def clear_cart(
    user_id: str = Field(description="User identifier"),
) -> dict:
    """
    Remove all items from a user's cart.
    """
    db = get_db()
    success = db.clear_cart(user_id)

    if not success:
        return {"error": "No active cart found for user"}

    return {"success": True, "message": "Cart cleared"}


@mcp.tool()
def get_cart_for_checkout(
    user_id: str = Field(description="User identifier"),
) -> dict:
    """
    Get cart contents formatted for checkout process.

    Returns items with product IDs, quantities, and prices for order creation.
    """
    db = get_db()
    catalog_db = get_catalog_db()

    cart = db.get_or_create_cart(user_id)
    items = db.get_cart_items(cart["id"])

    if not items:
        return {"error": "Cart is empty"}

    # Build checkout items
    checkout_items = []
    for item in items:
        product = catalog_db.get_product(item["product_id"])
        if not product:
            return {"error": f"Product not found: {item['product_id']}"}

        if product["stock_quantity"] < item["quantity"]:
            return {
                "error": f"Insufficient stock for {product['name']}",
                "product_id": item["product_id"],
                "requested": item["quantity"],
                "available": product["stock_quantity"],
            }

        checkout_items.append({
            "product_id": item["product_id"],
            "product_name": product["name"],
            "quantity": item["quantity"],
            "price": product["price"],
            "line_total": product["price"] * item["quantity"],
        })

    total = sum(item["line_total"] for item in checkout_items)

    return {
        "cart_id": cart["id"],
        "user_id": user_id,
        "items": checkout_items,
        "total": round(total, 2),
        "ready_for_checkout": True,
    }


def main():
    """Run the MCP server with HTTP transport."""
    mcp.run(transport="streamable-http")


if __name__ == "__main__":
    main()
