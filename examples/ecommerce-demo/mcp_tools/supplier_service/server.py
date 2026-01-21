#!/usr/bin/env python3
"""Supplier MCP service for purchase order management."""

import asyncio
import threading
from pathlib import Path
from typing import Optional

from mcp.server.fastmcp import FastMCP
from pydantic import Field

from .database import SupplierDatabase

# Initialize FastMCP server
mcp = FastMCP(
    name="Supplier Service",
    instructions="Supplier and purchase order management for ecommerce",
    host="0.0.0.0",
    port=8005,
)

# Lazy-loaded globals
_db: Optional[SupplierDatabase] = None
_simulation_thread: Optional[threading.Thread] = None
_simulation_running: bool = False


def get_db() -> SupplierDatabase:
    """Get or create database instance."""
    global _db
    if _db is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
        _db = SupplierDatabase(data_dir)
    return _db


@mcp.tool()
def list_suppliers() -> dict:
    """
    Get all available suppliers.

    Returns supplier details including lead times and reliability scores.
    """
    db = get_db()
    suppliers = db.list_suppliers()

    return {
        "count": len(suppliers),
        "suppliers": suppliers,
    }


@mcp.tool()
def get_supplier(
    supplier_id: str = Field(description="Supplier ID to retrieve"),
) -> dict:
    """
    Get detailed information about a supplier.
    """
    db = get_db()
    supplier = db.get_supplier(supplier_id)

    if not supplier:
        return {"error": f"Supplier not found: {supplier_id}"}

    return supplier


@mcp.tool()
def create_purchase_order(
    product_id: str = Field(description="Product to order"),
    supplier_id: str = Field(description="Supplier to order from"),
    quantity: int = Field(description="Quantity to order"),
    notes: Optional[str] = Field(default=None, description="Optional notes"),
) -> dict:
    """
    Create a new purchase order to restock inventory.

    The expected delivery date is calculated based on supplier lead time
    and reliability score.
    """
    db = get_db()
    result = db.create_purchase_order(
        product_id=product_id,
        supplier_id=supplier_id,
        quantity=quantity,
        notes=notes,
    )

    if "error" in result:
        return result

    return {
        "success": True,
        "purchase_order": result,
        "message": f"Purchase order {result['id']} created",
    }


@mcp.tool()
def get_purchase_order(
    po_id: str = Field(description="Purchase order ID"),
) -> dict:
    """
    Get details of a purchase order.
    """
    db = get_db()
    po = db.get_purchase_order(po_id)

    if not po:
        return {"error": f"Purchase order not found: {po_id}"}

    return po


@mcp.tool()
def list_purchase_orders(
    status: Optional[str] = Field(default=None, description="Filter by status"),
    supplier_id: Optional[str] = Field(default=None, description="Filter by supplier"),
    page: int = Field(default=1, description="Page number"),
    limit: int = Field(default=20, description="Orders per page"),
) -> dict:
    """
    List purchase orders with optional filtering.

    Status can be: pending, confirmed, shipped, received, cancelled
    """
    db = get_db()
    pos = db.list_purchase_orders(
        status=status,
        supplier_id=supplier_id,
        page=page,
        limit=limit,
    )

    return {
        "page": page,
        "limit": limit,
        "filters": {"status": status, "supplier_id": supplier_id},
        "count": len(pos),
        "purchase_orders": pos,
    }


@mcp.tool()
def update_po_status(
    po_id: str = Field(description="Purchase order ID"),
    status: str = Field(description="New status"),
) -> dict:
    """
    Update the status of a purchase order.

    Valid statuses: pending, confirmed, shipped, received, cancelled
    """
    valid_statuses = ["pending", "confirmed", "shipped", "received", "cancelled"]
    if status not in valid_statuses:
        return {"error": f"Invalid status. Must be one of: {valid_statuses}"}

    db = get_db()
    po = db.update_purchase_order_status(po_id, status)

    if not po:
        return {"error": f"Purchase order not found: {po_id}"}

    return {
        "success": True,
        "purchase_order": po,
    }


@mcp.tool()
def receive_shipment(
    po_id: str = Field(description="Purchase order ID"),
    quantity_received: Optional[int] = Field(
        default=None,
        description="Quantity received (defaults to ordered quantity)",
    ),
) -> dict:
    """
    Mark a purchase order as received and add stock to inventory.

    This updates the product's stock quantity and marks the PO as complete.
    """
    db = get_db()
    result = db.receive_shipment(po_id, quantity_received)

    if "error" in result:
        return result

    return {
        "success": True,
        **result,
        "message": f"Received {result['quantity_received']} units of {result['product_name']}",
    }


@mcp.tool()
def advance_deliveries(
    days: int = Field(default=1, description="Number of days to simulate"),
) -> dict:
    """
    Manually trigger delivery simulation.

    Advances time by the specified number of days and checks for
    purchase orders that should arrive. Delivery success is based
    on supplier reliability scores.
    """
    db = get_db()
    delivered = db.advance_deliveries(days=days)

    return {
        "simulated_days": days,
        "deliveries_completed": len(delivered),
        "deliveries": delivered,
    }


def _run_simulation(interval_seconds: int):
    """Background simulation runner."""
    global _simulation_running
    while _simulation_running:
        db = get_db()
        db.advance_deliveries(days=1)
        for _ in range(interval_seconds):
            if not _simulation_running:
                break
            import time
            time.sleep(1)


@mcp.tool()
def start_auto_simulation(
    interval_seconds: int = Field(default=60, description="Seconds between simulation ticks"),
) -> dict:
    """
    Start automatic delivery simulation in background.

    Simulates one day passing every interval_seconds.
    Useful for extended demos where you want deliveries to arrive automatically.
    """
    global _simulation_thread, _simulation_running

    if _simulation_running:
        return {"error": "Simulation already running"}

    _simulation_running = True
    _simulation_thread = threading.Thread(target=_run_simulation, args=(interval_seconds,))
    _simulation_thread.daemon = True
    _simulation_thread.start()

    return {
        "success": True,
        "message": f"Auto-simulation started. 1 day simulated every {interval_seconds} seconds.",
    }


@mcp.tool()
def stop_auto_simulation() -> dict:
    """
    Stop the background delivery simulation.
    """
    global _simulation_running

    if not _simulation_running:
        return {"error": "No simulation running"}

    _simulation_running = False

    return {
        "success": True,
        "message": "Auto-simulation stopped",
    }


@mcp.tool()
def get_pending_po_summary() -> dict:
    """
    Get summary of pending purchase orders.

    Returns count of orders awaiting delivery.
    """
    db = get_db()
    count = db.get_pending_po_count()
    pos = db.list_purchase_orders(status=None)  # Get all

    # Group by status
    by_status = {}
    for po in pos:
        status = po["status"]
        if status not in by_status:
            by_status[status] = 0
        by_status[status] += 1

    return {
        "pending_count": count,
        "by_status": by_status,
        "total": len(pos),
    }


# ============== SCATTER-GATHER SUPPORT TOOLS ==============


@mcp.tool()
def get_all_quotes(
    product_id: str = Field(description="Product ID to get quotes for"),
    quantity: int = Field(default=100, description="Quantity for quote"),
) -> dict:
    """
    Get price quotes from all suppliers for a product.

    Returns quotes from each supplier with pricing, lead times, and reliability.
    Used by get_best_supplier_price scatter-gather composition.
    """
    import hashlib

    db = get_db()
    suppliers = db.list_suppliers()

    # Get product info for base cost
    from ..catalog_service.database import CatalogDatabase
    data_dir = Path(__file__).parent.parent.parent / "data"
    catalog_db = CatalogDatabase(data_dir)
    product = catalog_db.get_product(product_id)

    if not product:
        return {"error": f"Product not found: {product_id}"}

    base_cost = product["cost"]

    quotes = []
    for supplier in suppliers:
        # Calculate supplier-specific pricing
        # - High reliability suppliers charge more
        # - Faster lead times cost more
        # - Volume discounts apply
        reliability_premium = (supplier["reliability_score"] - 0.8) * 0.15
        speed_premium = max(0, (14 - supplier["lead_time_days"]) * 0.01)
        volume_discount = 0.05 if quantity >= 100 else (0.02 if quantity >= 50 else 0)

        # Add some supplier-specific variation based on supplier ID hash
        h = int(hashlib.md5(f"{supplier['id']}{product_id}".encode()).hexdigest()[:8], 16)
        supplier_variation = ((h % 100) - 50) / 500  # -10% to +10% variation

        price_multiplier = 1.0 + reliability_premium + speed_premium - volume_discount + supplier_variation
        unit_price = round(base_cost * price_multiplier, 2)
        total_price = round(unit_price * quantity, 2)

        quotes.append({
            "supplier_id": supplier["id"],
            "supplier_name": supplier["name"],
            "unit_price": unit_price,
            "total_price": total_price,
            "quantity": quantity,
            "lead_time_days": supplier["lead_time_days"],
            "reliability_score": supplier["reliability_score"],
            "estimated_delivery_days": supplier["lead_time_days"] + int((1 - supplier["reliability_score"]) * 3),
        })

    # Sort by unit_price ascending (best price first)
    quotes.sort(key=lambda x: x["unit_price"])

    return {
        "product_id": product_id,
        "product_name": product["name"],
        "base_cost": base_cost,
        "quantity_requested": quantity,
        "quotes": quotes,
        "best_price": quotes[0] if quotes else None,
        "fastest_delivery": min(quotes, key=lambda x: x["estimated_delivery_days"]) if quotes else None,
    }


def main():
    """Run the MCP server with HTTP transport."""
    mcp.run(transport="streamable-http")


if __name__ == "__main__":
    main()
