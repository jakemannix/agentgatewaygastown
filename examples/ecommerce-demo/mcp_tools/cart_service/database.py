"""Database operations for the cart service."""

import uuid
from datetime import datetime
from pathlib import Path
from typing import Optional

from ..shared.db_utils import get_connection, get_db_path, rows_to_dicts

DB_NAME = "cart.db"

SCHEMA = """
CREATE TABLE IF NOT EXISTS carts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    status TEXT DEFAULT 'active',
    created_at TEXT,
    updated_at TEXT
);

CREATE TABLE IF NOT EXISTS cart_items (
    id TEXT PRIMARY KEY,
    cart_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    quantity INTEGER DEFAULT 1,
    FOREIGN KEY (cart_id) REFERENCES carts(id)
);

CREATE INDEX IF NOT EXISTS idx_carts_user ON carts(user_id);
CREATE INDEX IF NOT EXISTS idx_carts_status ON carts(status);
CREATE INDEX IF NOT EXISTS idx_cart_items_cart ON cart_items(cart_id);
"""


class CartDatabase:
    """Database operations for shopping carts."""

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

    def get_or_create_cart(self, user_id: str) -> dict:
        """Get active cart for user or create a new one."""
        conn = self._get_conn()
        try:
            # Check for existing active cart
            row = conn.execute(
                "SELECT * FROM carts WHERE user_id = ? AND status = 'active'",
                (user_id,),
            ).fetchone()

            if row:
                return dict(row)

            # Create new cart
            cart_id = str(uuid.uuid4())
            now = datetime.utcnow().isoformat()
            conn.execute(
                "INSERT INTO carts (id, user_id, status, created_at, updated_at) VALUES (?, ?, 'active', ?, ?)",
                (cart_id, user_id, now, now),
            )
            conn.commit()
            return {
                "id": cart_id,
                "user_id": user_id,
                "status": "active",
                "created_at": now,
                "updated_at": now,
            }
        finally:
            conn.close()

    def get_cart(self, cart_id: str) -> Optional[dict]:
        """Get a cart by ID."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                "SELECT * FROM carts WHERE id = ?", (cart_id,)
            ).fetchone()
            return dict(row) if row else None
        finally:
            conn.close()

    def get_cart_items(self, cart_id: str) -> list[dict]:
        """Get all items in a cart."""
        conn = self._get_conn()
        try:
            rows = conn.execute(
                "SELECT * FROM cart_items WHERE cart_id = ?", (cart_id,)
            ).fetchall()
            return rows_to_dicts(rows)
        finally:
            conn.close()

    def add_item(self, cart_id: str, product_id: str, quantity: int = 1) -> dict:
        """Add an item to the cart or update quantity if exists."""
        conn = self._get_conn()
        try:
            # Check if item already in cart
            existing = conn.execute(
                "SELECT * FROM cart_items WHERE cart_id = ? AND product_id = ?",
                (cart_id, product_id),
            ).fetchone()

            now = datetime.utcnow().isoformat()

            if existing:
                # Update quantity
                new_qty = existing["quantity"] + quantity
                conn.execute(
                    "UPDATE cart_items SET quantity = ? WHERE id = ?",
                    (new_qty, existing["id"]),
                )
                conn.execute(
                    "UPDATE carts SET updated_at = ? WHERE id = ?", (now, cart_id)
                )
                conn.commit()
                return {
                    "id": existing["id"],
                    "cart_id": cart_id,
                    "product_id": product_id,
                    "quantity": new_qty,
                }
            else:
                # Add new item
                item_id = str(uuid.uuid4())
                conn.execute(
                    "INSERT INTO cart_items (id, cart_id, product_id, quantity) VALUES (?, ?, ?, ?)",
                    (item_id, cart_id, product_id, quantity),
                )
                conn.execute(
                    "UPDATE carts SET updated_at = ? WHERE id = ?", (now, cart_id)
                )
                conn.commit()
                return {
                    "id": item_id,
                    "cart_id": cart_id,
                    "product_id": product_id,
                    "quantity": quantity,
                }
        finally:
            conn.close()

    def update_item_quantity(self, cart_item_id: str, quantity: int) -> Optional[dict]:
        """Update the quantity of a cart item."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                "SELECT * FROM cart_items WHERE id = ?", (cart_item_id,)
            ).fetchone()

            if not row:
                return None

            now = datetime.utcnow().isoformat()

            if quantity <= 0:
                # Remove item if quantity is 0 or less
                conn.execute("DELETE FROM cart_items WHERE id = ?", (cart_item_id,))
                conn.execute(
                    "UPDATE carts SET updated_at = ? WHERE id = ?",
                    (now, row["cart_id"]),
                )
                conn.commit()
                return {"deleted": True, "id": cart_item_id}

            conn.execute(
                "UPDATE cart_items SET quantity = ? WHERE id = ?",
                (quantity, cart_item_id),
            )
            conn.execute(
                "UPDATE carts SET updated_at = ? WHERE id = ?", (now, row["cart_id"])
            )
            conn.commit()

            return {
                "id": cart_item_id,
                "cart_id": row["cart_id"],
                "product_id": row["product_id"],
                "quantity": quantity,
            }
        finally:
            conn.close()

    def remove_item(self, cart_item_id: str) -> bool:
        """Remove an item from the cart."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                "SELECT cart_id FROM cart_items WHERE id = ?", (cart_item_id,)
            ).fetchone()

            if not row:
                return False

            now = datetime.utcnow().isoformat()
            conn.execute("DELETE FROM cart_items WHERE id = ?", (cart_item_id,))
            conn.execute(
                "UPDATE carts SET updated_at = ? WHERE id = ?", (now, row["cart_id"])
            )
            conn.commit()
            return True
        finally:
            conn.close()

    def clear_cart(self, user_id: str) -> bool:
        """Clear all items from a user's active cart."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                "SELECT id FROM carts WHERE user_id = ? AND status = 'active'",
                (user_id,),
            ).fetchone()

            if not row:
                return False

            now = datetime.utcnow().isoformat()
            conn.execute("DELETE FROM cart_items WHERE cart_id = ?", (row["id"],))
            conn.execute(
                "UPDATE carts SET updated_at = ? WHERE id = ?", (now, row["id"])
            )
            conn.commit()
            return True
        finally:
            conn.close()

    def update_cart_status(self, cart_id: str, status: str) -> Optional[dict]:
        """Update the status of a cart."""
        conn = self._get_conn()
        try:
            now = datetime.utcnow().isoformat()
            conn.execute(
                "UPDATE carts SET status = ?, updated_at = ? WHERE id = ?",
                (status, now, cart_id),
            )
            conn.commit()
            return self.get_cart(cart_id)
        finally:
            conn.close()
