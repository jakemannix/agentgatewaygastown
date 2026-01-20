"""Database operations for the order service."""

import uuid
from datetime import datetime
from pathlib import Path
from typing import Optional

from ..shared.db_utils import get_connection, get_db_path, rows_to_dicts

DB_NAME = "order.db"

SCHEMA = """
CREATE TABLE IF NOT EXISTS orders (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    cart_id TEXT,
    total REAL NOT NULL,
    status TEXT DEFAULT 'pending',
    shipping_address TEXT,
    created_at TEXT,
    updated_at TEXT
);

CREATE TABLE IF NOT EXISTS order_items (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    price_at_time REAL NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders(id)
);

CREATE INDEX IF NOT EXISTS idx_orders_user ON orders(user_id);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_order_items_order ON order_items(order_id);
"""


class OrderDatabase:
    """Database operations for orders."""

    def __init__(self, data_dir: Optional[Path] = None):
        self.db_path = get_db_path(DB_NAME, data_dir)
        self._init_db()

    def _get_conn(self):
        return get_connection(self.db_path)

    def _init_db(self):
        """Initialize the database schema."""
        conn = self._get_conn()
        try:
            conn.executescript(SCHEMA)
            conn.commit()
        finally:
            conn.close()

    def create_order(
        self,
        user_id: str,
        items: list[dict],  # [{product_id, quantity, price}]
        shipping_address: Optional[str] = None,
        cart_id: Optional[str] = None,
    ) -> dict:
        """Create a new order."""
        conn = self._get_conn()
        try:
            order_id = str(uuid.uuid4())
            now = datetime.utcnow().isoformat()

            # Calculate total
            total = sum(item["quantity"] * item["price"] for item in items)

            conn.execute(
                """
                INSERT INTO orders (id, user_id, cart_id, total, status, shipping_address, created_at, updated_at)
                VALUES (?, ?, ?, ?, 'pending', ?, ?, ?)
                """,
                (order_id, user_id, cart_id, total, shipping_address, now, now),
            )

            # Insert order items
            for item in items:
                item_id = str(uuid.uuid4())
                conn.execute(
                    """
                    INSERT INTO order_items (id, order_id, product_id, quantity, price_at_time)
                    VALUES (?, ?, ?, ?, ?)
                    """,
                    (
                        item_id,
                        order_id,
                        item["product_id"],
                        item["quantity"],
                        item["price"],
                    ),
                )

            conn.commit()
            return self.get_order(order_id)
        finally:
            conn.close()

    def get_order(self, order_id: str) -> Optional[dict]:
        """Get an order by ID with its items."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                "SELECT * FROM orders WHERE id = ?", (order_id,)
            ).fetchone()

            if not row:
                return None

            order = dict(row)
            items = conn.execute(
                "SELECT * FROM order_items WHERE order_id = ?", (order_id,)
            ).fetchall()
            order["items"] = rows_to_dicts(items)
            return order
        finally:
            conn.close()

    def list_orders(
        self,
        user_id: Optional[str] = None,
        status: Optional[str] = None,
        page: int = 1,
        limit: int = 20,
    ) -> list[dict]:
        """List orders with optional filtering."""
        conn = self._get_conn()
        try:
            offset = (page - 1) * limit
            conditions = []
            params = []

            if user_id:
                conditions.append("user_id = ?")
                params.append(user_id)
            if status:
                conditions.append("status = ?")
                params.append(status)

            where_clause = " AND ".join(conditions) if conditions else "1=1"
            params.extend([limit, offset])

            rows = conn.execute(
                f"SELECT * FROM orders WHERE {where_clause} ORDER BY created_at DESC LIMIT ? OFFSET ?",
                params,
            ).fetchall()

            orders = []
            for row in rows:
                order = dict(row)
                items = conn.execute(
                    "SELECT * FROM order_items WHERE order_id = ?", (order["id"],)
                ).fetchall()
                order["items"] = rows_to_dicts(items)
                orders.append(order)
            return orders
        finally:
            conn.close()

    def update_order_status(self, order_id: str, status: str) -> Optional[dict]:
        """Update the status of an order."""
        conn = self._get_conn()
        try:
            now = datetime.utcnow().isoformat()
            conn.execute(
                "UPDATE orders SET status = ?, updated_at = ? WHERE id = ?",
                (status, now, order_id),
            )
            conn.commit()
            return self.get_order(order_id)
        finally:
            conn.close()

    def cancel_order(self, order_id: str) -> Optional[dict]:
        """Cancel an order if it's still pending."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                "SELECT status FROM orders WHERE id = ?", (order_id,)
            ).fetchone()

            if not row:
                return None

            if row["status"] not in ("pending", "confirmed"):
                return {"error": f"Cannot cancel order with status: {row['status']}"}

            now = datetime.utcnow().isoformat()
            conn.execute(
                "UPDATE orders SET status = 'cancelled', updated_at = ? WHERE id = ?",
                (now, order_id),
            )
            conn.commit()
            return self.get_order(order_id)
        finally:
            conn.close()

    def get_sales_report(
        self,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
    ) -> dict:
        """Get sales analytics report."""
        conn = self._get_conn()
        try:
            conditions = ["status NOT IN ('cancelled', 'pending')"]
            params = []

            if start_date:
                conditions.append("created_at >= ?")
                params.append(start_date)
            if end_date:
                conditions.append("created_at <= ?")
                params.append(end_date)

            where_clause = " AND ".join(conditions)

            # Get overall stats
            stats_row = conn.execute(
                f"""
                SELECT
                    COALESCE(SUM(total), 0) as total_revenue,
                    COUNT(*) as total_orders
                FROM orders
                WHERE {where_clause}
                """,
                params,
            ).fetchone()

            # Get per-product breakdown
            product_rows = conn.execute(
                f"""
                SELECT
                    oi.product_id,
                    SUM(oi.quantity) as units_sold,
                    SUM(oi.quantity * oi.price_at_time) as revenue,
                    AVG(oi.price_at_time) as avg_price
                FROM order_items oi
                JOIN orders o ON oi.order_id = o.id
                WHERE {where_clause}
                GROUP BY oi.product_id
                ORDER BY revenue DESC
                """,
                params,
            ).fetchall()

            return {
                "start_date": start_date,
                "end_date": end_date,
                "total_revenue": stats_row["total_revenue"],
                "total_orders": stats_row["total_orders"],
                "total_units": sum(row["units_sold"] for row in product_rows),
                "items": rows_to_dicts(product_rows),
            }
        finally:
            conn.close()
