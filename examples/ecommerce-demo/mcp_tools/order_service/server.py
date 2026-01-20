#!/usr/bin/env python3
"""Order MCP service for order management."""

from pathlib import Path
from typing import Optional

from mcp.server.fastmcp import FastMCP
from pydantic import Field

from .database import OrderDatabase
from ..cart_service.database import CartDatabase
from ..catalog_service.database import CatalogDatabase

# Initialize FastMCP server
mcp = FastMCP(
    name="Order Service",
    instructions="Order management for ecommerce",
    host="0.0.0.0",
    port=8003,
)

# Lazy-loaded globals
_db: Optional[OrderDatabase] = None
_cart_db: Optional[CartDatabase] = None
_catalog_db: Optional[CatalogDatabase] = None


def get_db() -> OrderDatabase:
    """Get or create database instance."""
    global _db
    if _db is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
        _db = OrderDatabase(data_dir)
    return _db


def get_cart_db() -> CartDatabase:
    """Get or create cart database instance."""
    global _cart_db
    if _cart_db is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
        _cart_db = CartDatabase(data_dir)
    return _cart_db


def get_catalog_db() -> CatalogDatabase:
    """Get or create catalog database instance."""
    global _catalog_db
    if _catalog_db is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
        _catalog_db = CatalogDatabase(data_dir)
    return _catalog_db


def enrich_order_items(items: list[dict]) -> list[dict]:
    """Add product names to order items."""
    catalog_db = get_catalog_db()
    enriched = []
    for item in items:
        product = catalog_db.get_product(item["product_id"])
        enriched.append({
            **item,
            "product_name": product["name"] if product else "Unknown",
            "line_total": item["price_at_time"] * item["quantity"],
        })
    return enriched


@mcp.tool()
def checkout(
    user_id: str = Field(description="User identifier"),
    shipping_address: str = Field(description="Shipping address for the order"),
) -> dict:
    """
    Create an order from the user's cart.

    This converts the current cart into an order and marks the cart as checked out.
    Note: This does NOT deduct inventory - use safe_checkout via gateway for that.
    """
    db = get_db()
    cart_db = get_cart_db()
    catalog_db = get_catalog_db()

    # Get user's active cart
    cart = cart_db.get_or_create_cart(user_id)
    cart_items = cart_db.get_cart_items(cart["id"])

    if not cart_items:
        return {"error": "Cart is empty"}

    # Build order items with current prices
    order_items = []
    for item in cart_items:
        product = catalog_db.get_product(item["product_id"])
        if not product:
            return {"error": f"Product not found: {item['product_id']}"}

        order_items.append({
            "product_id": item["product_id"],
            "quantity": item["quantity"],
            "price": product["price"],
        })

    # Create order
    order = db.create_order(
        user_id=user_id,
        items=order_items,
        shipping_address=shipping_address,
        cart_id=cart["id"],
    )

    # Mark cart as checked out
    cart_db.update_cart_status(cart["id"], "checked_out")

    # Enrich order items
    order["items"] = enrich_order_items(order["items"])

    return {
        "success": True,
        "order": order,
        "message": f"Order {order['id']} created successfully",
    }


@mcp.tool()
def get_order(
    order_id: str = Field(description="Order ID to retrieve"),
) -> dict:
    """
    Get detailed information about an order.

    Returns order details including all items and current status.
    """
    db = get_db()
    order = db.get_order(order_id)

    if not order:
        return {"error": f"Order not found: {order_id}"}

    # Enrich items
    order["items"] = enrich_order_items(order["items"])

    return order


@mcp.tool()
def list_orders(
    user_id: Optional[str] = Field(default=None, description="Filter by user"),
    status: Optional[str] = Field(default=None, description="Filter by status"),
    page: int = Field(default=1, description="Page number"),
    limit: int = Field(default=20, description="Orders per page"),
) -> dict:
    """
    List orders with optional filtering.

    Can filter by user and/or status. Returns paginated results.
    """
    db = get_db()
    orders = db.list_orders(user_id=user_id, status=status, page=page, limit=limit)

    # Enrich items in each order
    for order in orders:
        order["items"] = enrich_order_items(order["items"])

    return {
        "page": page,
        "limit": limit,
        "filters": {"user_id": user_id, "status": status},
        "count": len(orders),
        "orders": orders,
    }


@mcp.tool()
def cancel_order(
    order_id: str = Field(description="Order ID to cancel"),
) -> dict:
    """
    Cancel an order if it's still in a cancellable state.

    Only pending or confirmed orders can be cancelled.
    Note: This does NOT restore inventory - use safe_cancel via gateway for that.
    """
    db = get_db()
    result = db.cancel_order(order_id)

    if not result:
        return {"error": f"Order not found: {order_id}"}

    if "error" in result:
        return result

    return {
        "success": True,
        "order": result,
        "message": f"Order {order_id} has been cancelled",
    }


@mcp.tool()
def update_order_status(
    order_id: str = Field(description="Order ID to update"),
    status: str = Field(description="New status (confirmed, shipped, delivered)"),
) -> dict:
    """
    Update the status of an order (admin/merchandiser function).

    Valid status transitions: pending -> confirmed -> shipped -> delivered
    """
    valid_statuses = ["pending", "confirmed", "shipped", "delivered", "cancelled"]
    if status not in valid_statuses:
        return {"error": f"Invalid status. Must be one of: {valid_statuses}"}

    db = get_db()
    order = db.update_order_status(order_id, status)

    if not order:
        return {"error": f"Order not found: {order_id}"}

    order["items"] = enrich_order_items(order["items"])

    return {
        "success": True,
        "order": order,
        "message": f"Order status updated to {status}",
    }


@mcp.tool()
def get_sales_report(
    start_date: Optional[str] = Field(default=None, description="Start date (ISO format)"),
    end_date: Optional[str] = Field(default=None, description="End date (ISO format)"),
) -> dict:
    """
    Get sales analytics report (merchandiser function).

    Returns revenue, units sold, and per-product breakdown.
    """
    db = get_db()
    catalog_db = get_catalog_db()

    report = db.get_sales_report(start_date=start_date, end_date=end_date)

    # Enrich with product names
    for item in report["items"]:
        product = catalog_db.get_product(item["product_id"])
        item["product_name"] = product["name"] if product else "Unknown"

    return report


def main():
    """Run the MCP server with HTTP transport."""
    mcp.run(transport="streamable-http")


if __name__ == "__main__":
    main()
