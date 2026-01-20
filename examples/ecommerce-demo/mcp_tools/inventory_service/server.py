#!/usr/bin/env python3
"""Inventory MCP service for stock management."""

from pathlib import Path
from typing import Optional

from mcp.server.fastmcp import FastMCP
from pydantic import Field

from .database import InventoryDatabase
from ..catalog_service.database import CatalogDatabase

# Initialize FastMCP server
mcp = FastMCP(
    name="Inventory Service",
    instructions="Inventory and stock management for ecommerce",
    host="0.0.0.0",
    port=8004,
)

# Lazy-loaded globals
_db: Optional[InventoryDatabase] = None
_catalog_db: Optional[CatalogDatabase] = None


def get_db() -> InventoryDatabase:
    """Get or create database instance."""
    global _db
    if _db is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
        _db = InventoryDatabase(data_dir)
    return _db


def get_catalog_db() -> CatalogDatabase:
    """Get or create catalog database instance."""
    global _catalog_db
    if _catalog_db is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
        _catalog_db = CatalogDatabase(data_dir)
    return _catalog_db


@mcp.tool()
def get_inventory_report() -> dict:
    """
    Get full inventory status report.

    Returns summary statistics and per-product stock levels.
    """
    db = get_db()
    return db.get_inventory_report()


@mcp.tool()
def get_low_stock_alerts(
    threshold: Optional[int] = Field(
        default=None,
        description="Custom threshold (uses product-specific threshold if not provided)",
    ),
) -> dict:
    """
    Get products with stock below their reorder threshold.

    Returns list of products that need restocking with deficit amounts.
    """
    db = get_db()
    alerts = db.get_low_stock_alerts(threshold=threshold)

    return {
        "count": len(alerts),
        "threshold_type": "custom" if threshold else "per-product",
        "alerts": alerts,
    }


@mcp.tool()
def adjust_inventory(
    product_id: str = Field(description="Product ID to adjust"),
    quantity_change: int = Field(description="Amount to add (positive) or remove (negative)"),
    reason: str = Field(description="Reason for adjustment (e.g., 'damaged', 'found', 'correction')"),
) -> dict:
    """
    Manually adjust inventory levels with audit trail.

    Use positive values to add stock, negative to remove.
    """
    db = get_db()
    catalog_db = get_catalog_db()

    # Verify product exists
    product = catalog_db.get_product(product_id)
    if not product:
        return {"error": f"Product not found: {product_id}"}

    # Check we won't go negative
    if product["stock_quantity"] + quantity_change < 0:
        return {
            "error": f"Cannot reduce stock below 0. Current: {product['stock_quantity']}, change: {quantity_change}",
        }

    result = db.adjust_inventory(product_id, quantity_change, reason)

    return {
        "success": True,
        "product_id": product_id,
        "product_name": product["name"],
        "adjustment": quantity_change,
        "reason": reason,
        "new_stock": result["new_stock"],
    }


@mcp.tool()
def reserve_inventory(
    product_id: str = Field(description="Product ID to reserve"),
    quantity: int = Field(description="Quantity to reserve"),
    order_id: Optional[str] = Field(default=None, description="Associated order ID"),
) -> dict:
    """
    Reserve inventory for an order.

    Deducts from available stock and creates a reservation record.
    Used during checkout to prevent overselling.
    """
    db = get_db()
    result = db.reserve_inventory(product_id, quantity, order_id)

    if not result.get("success"):
        return result

    catalog_db = get_catalog_db()
    product = catalog_db.get_product(product_id)

    return {
        **result,
        "product_name": product["name"] if product else "Unknown",
    }


@mcp.tool()
def release_inventory(
    product_id: str = Field(description="Product ID to release"),
    quantity: int = Field(description="Quantity to release back to stock"),
    reservation_id: Optional[str] = Field(default=None, description="Reservation ID if known"),
) -> dict:
    """
    Release reserved inventory back to available stock.

    Used when an order is cancelled or a reservation expires.
    """
    db = get_db()
    result = db.release_inventory(product_id, quantity, reservation_id)

    catalog_db = get_catalog_db()
    product = catalog_db.get_product(product_id)

    return {
        **result,
        "product_name": product["name"] if product else "Unknown",
    }


@mcp.tool()
def reserve_inventory_batch(
    items: list[dict] = Field(description="List of {product_id, quantity} to reserve"),
    order_id: Optional[str] = Field(default=None, description="Associated order ID"),
) -> dict:
    """
    Reserve inventory for multiple products at once.

    Used during checkout for entire cart. Returns success only if ALL items
    can be reserved. If any fail, none are reserved.
    """
    db = get_db()
    catalog_db = get_catalog_db()

    # First, validate all items can be reserved
    for item in items:
        product = catalog_db.get_product(item["product_id"])
        if not product:
            return {"success": False, "error": f"Product not found: {item['product_id']}"}

        if product["stock_quantity"] < item["quantity"]:
            return {
                "success": False,
                "error": f"Insufficient stock for {product['name']}",
                "product_id": item["product_id"],
                "available": product["stock_quantity"],
                "requested": item["quantity"],
            }

    # All validated, now reserve
    reservations = []
    for item in items:
        result = db.reserve_inventory(item["product_id"], item["quantity"], order_id)
        if result.get("success"):
            reservations.append(result)
        else:
            # Rollback previous reservations
            for res in reservations:
                db.release_inventory(res["product_id"], res["quantity"], res["reservation_id"])
            return {"success": False, "error": "Reservation failed, rolled back", "details": result}

    return {
        "success": True,
        "reservations": reservations,
        "items_reserved": len(reservations),
    }


@mcp.tool()
def release_inventory_batch(
    items: list[dict] = Field(description="List of {product_id, quantity} to release"),
) -> dict:
    """
    Release inventory for multiple products at once.

    Used when cancelling an order to restore stock.
    """
    db = get_db()
    released = []

    for item in items:
        result = db.release_inventory(item["product_id"], item["quantity"])
        released.append(result)

    return {
        "success": True,
        "released": released,
        "items_released": len(released),
    }


@mcp.tool()
def get_adjustment_history(
    product_id: Optional[str] = Field(default=None, description="Filter by product"),
    limit: int = Field(default=50, description="Maximum records to return"),
) -> dict:
    """
    Get inventory adjustment history for auditing.

    Returns recent manual adjustments with reasons.
    """
    db = get_db()
    adjustments = db.get_adjustments(product_id=product_id, limit=limit)

    return {
        "count": len(adjustments),
        "product_id": product_id,
        "adjustments": adjustments,
    }


def main():
    """Run the MCP server with HTTP transport."""
    mcp.run(transport="streamable-http")


if __name__ == "__main__":
    main()
