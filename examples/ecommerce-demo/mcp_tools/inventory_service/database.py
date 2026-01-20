"""Database operations for the inventory service.

Note: This service reads/writes to the catalog.db for product stock levels
but maintains its own adjustments table for audit trail.
"""

import uuid
from datetime import datetime
from pathlib import Path
from typing import Optional

from ..shared.db_utils import get_connection, get_db_path, rows_to_dicts

DB_NAME = "inventory.db"
CATALOG_DB_NAME = "catalog.db"

SCHEMA = """
CREATE TABLE IF NOT EXISTS inventory_adjustments (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL,
    quantity_change INTEGER NOT NULL,
    reason TEXT,
    created_at TEXT
);

CREATE TABLE IF NOT EXISTS inventory_reservations (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    order_id TEXT,
    created_at TEXT,
    released_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_adjustments_product ON inventory_adjustments(product_id);
CREATE INDEX IF NOT EXISTS idx_reservations_product ON inventory_reservations(product_id);
"""


class InventoryDatabase:
    """Database operations for inventory management."""

    def __init__(self, data_dir: Optional[Path] = None):
        if data_dir is None:
            data_dir = Path(__file__).parent.parent.parent / "data"
        self.data_dir = data_dir
        self.db_path = get_db_path(DB_NAME, data_dir)
        self.catalog_db_path = get_db_path(CATALOG_DB_NAME, data_dir)
        self._init_db()

    def _get_conn(self):
        return get_connection(self.db_path)

    def _get_catalog_conn(self):
        return get_connection(self.catalog_db_path)

    def _init_db(self):
        """Initialize the database schema."""
        conn = self._get_conn()
        try:
            conn.executescript(SCHEMA)
            conn.commit()
        finally:
            conn.close()

    def get_inventory_report(self) -> dict:
        """Get full inventory status from catalog database."""
        conn = self._get_catalog_conn()
        try:
            rows = conn.execute(
                """
                SELECT id, name, stock_quantity, reorder_threshold, cost
                FROM products
                ORDER BY name
                """
            ).fetchall()

            products = rows_to_dicts(rows)
            total_units = sum(p["stock_quantity"] for p in products)
            total_value = sum(p["stock_quantity"] * p["cost"] for p in products)
            low_stock = sum(
                1 for p in products if p["stock_quantity"] < p["reorder_threshold"]
            )
            out_of_stock = sum(1 for p in products if p["stock_quantity"] == 0)

            return {
                "total_products": len(products),
                "total_units": total_units,
                "total_value": total_value,
                "low_stock_count": low_stock,
                "out_of_stock_count": out_of_stock,
                "products": products,
            }
        finally:
            conn.close()

    def get_low_stock_alerts(self, threshold: Optional[int] = None) -> list[dict]:
        """Get products with stock below threshold."""
        conn = self._get_catalog_conn()
        try:
            if threshold is not None:
                rows = conn.execute(
                    """
                    SELECT id as product_id, name as product_name, stock_quantity as current_stock,
                           ? as reorder_threshold, (? - stock_quantity) as deficit
                    FROM products
                    WHERE stock_quantity < ?
                    ORDER BY deficit DESC
                    """,
                    (threshold, threshold, threshold),
                ).fetchall()
            else:
                rows = conn.execute(
                    """
                    SELECT id as product_id, name as product_name, stock_quantity as current_stock,
                           reorder_threshold, (reorder_threshold - stock_quantity) as deficit
                    FROM products
                    WHERE stock_quantity < reorder_threshold
                    ORDER BY deficit DESC
                    """
                ).fetchall()
            return rows_to_dicts(rows)
        finally:
            conn.close()

    def adjust_inventory(
        self, product_id: str, quantity_change: int, reason: str
    ) -> dict:
        """Adjust inventory with audit trail."""
        # Record the adjustment
        conn = self._get_conn()
        try:
            adjustment_id = str(uuid.uuid4())
            now = datetime.utcnow().isoformat()
            conn.execute(
                """
                INSERT INTO inventory_adjustments (id, product_id, quantity_change, reason, created_at)
                VALUES (?, ?, ?, ?, ?)
                """,
                (adjustment_id, product_id, quantity_change, reason, now),
            )
            conn.commit()
        finally:
            conn.close()

        # Update catalog stock
        cat_conn = self._get_catalog_conn()
        try:
            now = datetime.utcnow().isoformat()
            cat_conn.execute(
                """
                UPDATE products
                SET stock_quantity = stock_quantity + ?, updated_at = ?
                WHERE id = ?
                """,
                (quantity_change, now, product_id),
            )
            cat_conn.commit()

            row = cat_conn.execute(
                "SELECT id, name, stock_quantity FROM products WHERE id = ?",
                (product_id,),
            ).fetchone()

            return {
                "adjustment_id": adjustment_id,
                "product_id": product_id,
                "quantity_change": quantity_change,
                "reason": reason,
                "new_stock": row["stock_quantity"] if row else None,
            }
        finally:
            cat_conn.close()

    def reserve_inventory(
        self, product_id: str, quantity: int, order_id: Optional[str] = None
    ) -> dict:
        """Reserve inventory for an order."""
        # Check available stock
        cat_conn = self._get_catalog_conn()
        try:
            row = cat_conn.execute(
                "SELECT stock_quantity FROM products WHERE id = ?", (product_id,)
            ).fetchone()

            if not row:
                return {"success": False, "error": "Product not found"}

            if row["stock_quantity"] < quantity:
                return {
                    "success": False,
                    "error": f"Insufficient stock. Available: {row['stock_quantity']}, requested: {quantity}",
                }

            # Deduct from stock
            now = datetime.utcnow().isoformat()
            cat_conn.execute(
                """
                UPDATE products
                SET stock_quantity = stock_quantity - ?, updated_at = ?
                WHERE id = ?
                """,
                (quantity, now, product_id),
            )
            cat_conn.commit()
        finally:
            cat_conn.close()

        # Record reservation
        conn = self._get_conn()
        try:
            reservation_id = str(uuid.uuid4())
            now = datetime.utcnow().isoformat()
            conn.execute(
                """
                INSERT INTO inventory_reservations (id, product_id, quantity, order_id, created_at)
                VALUES (?, ?, ?, ?, ?)
                """,
                (reservation_id, product_id, quantity, order_id, now),
            )
            conn.commit()

            return {
                "success": True,
                "reservation_id": reservation_id,
                "product_id": product_id,
                "quantity": quantity,
            }
        finally:
            conn.close()

    def release_inventory(
        self, product_id: str, quantity: int, reservation_id: Optional[str] = None
    ) -> dict:
        """Release reserved inventory back to stock."""
        # Add back to stock
        cat_conn = self._get_catalog_conn()
        try:
            now = datetime.utcnow().isoformat()
            cat_conn.execute(
                """
                UPDATE products
                SET stock_quantity = stock_quantity + ?, updated_at = ?
                WHERE id = ?
                """,
                (quantity, now, product_id),
            )
            cat_conn.commit()
        finally:
            cat_conn.close()

        # Mark reservation as released
        if reservation_id:
            conn = self._get_conn()
            try:
                now = datetime.utcnow().isoformat()
                conn.execute(
                    "UPDATE inventory_reservations SET released_at = ? WHERE id = ?",
                    (now, reservation_id),
                )
                conn.commit()
            finally:
                conn.close()

        return {
            "success": True,
            "product_id": product_id,
            "quantity_released": quantity,
        }

    def get_adjustments(
        self, product_id: Optional[str] = None, limit: int = 50
    ) -> list[dict]:
        """Get inventory adjustment history."""
        conn = self._get_conn()
        try:
            if product_id:
                rows = conn.execute(
                    """
                    SELECT * FROM inventory_adjustments
                    WHERE product_id = ?
                    ORDER BY created_at DESC
                    LIMIT ?
                    """,
                    (product_id, limit),
                ).fetchall()
            else:
                rows = conn.execute(
                    """
                    SELECT * FROM inventory_adjustments
                    ORDER BY created_at DESC
                    LIMIT ?
                    """,
                    (limit,),
                ).fetchall()
            return rows_to_dicts(rows)
        finally:
            conn.close()
