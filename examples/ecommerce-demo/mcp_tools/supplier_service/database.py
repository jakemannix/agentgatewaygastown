"""Database operations for the supplier service."""

import random
import uuid
from datetime import datetime, timedelta
from pathlib import Path
from typing import Optional

from ..shared.db_utils import get_connection, get_db_path, rows_to_dicts

DB_NAME = "supplier.db"
CATALOG_DB_NAME = "catalog.db"

SCHEMA = """
CREATE TABLE IF NOT EXISTS suppliers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    lead_time_days INTEGER DEFAULT 7,
    reliability_score REAL DEFAULT 0.9,
    contact_email TEXT
);

CREATE TABLE IF NOT EXISTS purchase_orders (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL,
    supplier_id TEXT NOT NULL,
    quantity_ordered INTEGER NOT NULL,
    unit_cost REAL NOT NULL,
    status TEXT DEFAULT 'pending',
    expected_delivery TEXT,
    actual_delivery TEXT,
    created_at TEXT,
    notes TEXT,
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id)
);

CREATE INDEX IF NOT EXISTS idx_pos_status ON purchase_orders(status);
CREATE INDEX IF NOT EXISTS idx_pos_supplier ON purchase_orders(supplier_id);
CREATE INDEX IF NOT EXISTS idx_pos_product ON purchase_orders(product_id);
"""


class SupplierDatabase:
    """Database operations for supplier and purchase order management."""

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

    def list_suppliers(self) -> list[dict]:
        """List all suppliers."""
        conn = self._get_conn()
        try:
            rows = conn.execute(
                "SELECT * FROM suppliers ORDER BY name"
            ).fetchall()
            return rows_to_dicts(rows)
        finally:
            conn.close()

    def get_supplier(self, supplier_id: str) -> Optional[dict]:
        """Get a supplier by ID."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                "SELECT * FROM suppliers WHERE id = ?", (supplier_id,)
            ).fetchone()
            return dict(row) if row else None
        finally:
            conn.close()

    def create_supplier(
        self,
        supplier_id: str,
        name: str,
        lead_time_days: int = 7,
        reliability_score: float = 0.9,
        contact_email: Optional[str] = None,
    ) -> dict:
        """Create a new supplier."""
        conn = self._get_conn()
        try:
            conn.execute(
                """
                INSERT INTO suppliers (id, name, lead_time_days, reliability_score, contact_email)
                VALUES (?, ?, ?, ?, ?)
                """,
                (supplier_id, name, lead_time_days, reliability_score, contact_email),
            )
            conn.commit()
            return self.get_supplier(supplier_id)
        finally:
            conn.close()

    def create_purchase_order(
        self,
        product_id: str,
        supplier_id: str,
        quantity: int,
        notes: Optional[str] = None,
    ) -> dict:
        """Create a new purchase order."""
        conn = self._get_conn()
        try:
            # Get supplier info
            supplier = conn.execute(
                "SELECT * FROM suppliers WHERE id = ?", (supplier_id,)
            ).fetchone()

            if not supplier:
                return {"error": "Supplier not found"}

            # Get product cost from catalog
            cat_conn = self._get_catalog_conn()
            product = cat_conn.execute(
                "SELECT id, name, cost FROM products WHERE id = ?", (product_id,)
            ).fetchone()
            cat_conn.close()

            if not product:
                return {"error": "Product not found"}

            po_id = str(uuid.uuid4())
            now = datetime.utcnow()

            # Calculate expected delivery based on supplier lead time and reliability
            base_lead_time = supplier["lead_time_days"]
            # Add some variability based on reliability (lower reliability = more variability)
            variability = int((1 - supplier["reliability_score"]) * base_lead_time)
            expected_days = base_lead_time + random.randint(0, variability)
            expected_delivery = (now + timedelta(days=expected_days)).isoformat()

            conn.execute(
                """
                INSERT INTO purchase_orders
                    (id, product_id, supplier_id, quantity_ordered, unit_cost, status,
                     expected_delivery, created_at, notes)
                VALUES (?, ?, ?, ?, ?, 'pending', ?, ?, ?)
                """,
                (
                    po_id,
                    product_id,
                    supplier_id,
                    quantity,
                    product["cost"],
                    expected_delivery,
                    now.isoformat(),
                    notes,
                ),
            )
            conn.commit()

            return {
                "id": po_id,
                "product_id": product_id,
                "product_name": product["name"],
                "supplier_id": supplier_id,
                "supplier_name": supplier["name"],
                "quantity_ordered": quantity,
                "unit_cost": product["cost"],
                "total_cost": quantity * product["cost"],
                "status": "pending",
                "expected_delivery": expected_delivery,
                "created_at": now.isoformat(),
            }
        finally:
            conn.close()

    def get_purchase_order(self, po_id: str) -> Optional[dict]:
        """Get a purchase order by ID with enriched details."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                """
                SELECT po.*, s.name as supplier_name
                FROM purchase_orders po
                JOIN suppliers s ON po.supplier_id = s.id
                WHERE po.id = ?
                """,
                (po_id,),
            ).fetchone()

            if not row:
                return None

            po = dict(row)

            # Get product name from catalog
            cat_conn = self._get_catalog_conn()
            product = cat_conn.execute(
                "SELECT name FROM products WHERE id = ?", (po["product_id"],)
            ).fetchone()
            cat_conn.close()

            po["product_name"] = product["name"] if product else "Unknown"
            return po
        finally:
            conn.close()

    def list_purchase_orders(
        self,
        status: Optional[str] = None,
        supplier_id: Optional[str] = None,
        page: int = 1,
        limit: int = 20,
    ) -> list[dict]:
        """List purchase orders with optional filtering."""
        conn = self._get_conn()
        try:
            offset = (page - 1) * limit
            conditions = []
            params = []

            if status:
                conditions.append("po.status = ?")
                params.append(status)
            if supplier_id:
                conditions.append("po.supplier_id = ?")
                params.append(supplier_id)

            where_clause = " AND ".join(conditions) if conditions else "1=1"
            params.extend([limit, offset])

            rows = conn.execute(
                f"""
                SELECT po.*, s.name as supplier_name
                FROM purchase_orders po
                JOIN suppliers s ON po.supplier_id = s.id
                WHERE {where_clause}
                ORDER BY po.created_at DESC
                LIMIT ? OFFSET ?
                """,
                params,
            ).fetchall()

            pos = rows_to_dicts(rows)

            # Enrich with product names
            cat_conn = self._get_catalog_conn()
            for po in pos:
                product = cat_conn.execute(
                    "SELECT name FROM products WHERE id = ?", (po["product_id"],)
                ).fetchone()
                po["product_name"] = product["name"] if product else "Unknown"
            cat_conn.close()

            return pos
        finally:
            conn.close()

    def update_purchase_order_status(
        self, po_id: str, status: str
    ) -> Optional[dict]:
        """Update the status of a purchase order."""
        conn = self._get_conn()
        try:
            conn.execute(
                "UPDATE purchase_orders SET status = ? WHERE id = ?",
                (status, po_id),
            )
            conn.commit()
            return self.get_purchase_order(po_id)
        finally:
            conn.close()

    def receive_shipment(
        self, po_id: str, quantity_received: Optional[int] = None
    ) -> dict:
        """Mark a purchase order as received and add stock."""
        conn = self._get_conn()
        try:
            po = conn.execute(
                "SELECT * FROM purchase_orders WHERE id = ?", (po_id,)
            ).fetchone()

            if not po:
                return {"error": "Purchase order not found"}

            if po["status"] == "received":
                return {"error": "Purchase order already received"}

            # Use ordered quantity if not specified
            qty = quantity_received if quantity_received is not None else po["quantity_ordered"]

            now = datetime.utcnow().isoformat()
            conn.execute(
                """
                UPDATE purchase_orders
                SET status = 'received', actual_delivery = ?
                WHERE id = ?
                """,
                (now, po_id),
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
                (qty, now, po["product_id"]),
            )
            cat_conn.commit()

            product = cat_conn.execute(
                "SELECT name, stock_quantity FROM products WHERE id = ?",
                (po["product_id"],),
            ).fetchone()
        finally:
            cat_conn.close()

        return {
            "success": True,
            "po_id": po_id,
            "product_id": po["product_id"],
            "product_name": product["name"] if product else "Unknown",
            "quantity_received": qty,
            "new_stock": product["stock_quantity"] if product else None,
        }

    def advance_deliveries(self, days: int = 1) -> list[dict]:
        """
        Simulate time passing - check for purchase orders that should arrive.
        Returns list of POs that were delivered.
        """
        conn = self._get_conn()
        try:
            # Get pending/shipped POs with expected delivery in the past
            cutoff = datetime.utcnow() + timedelta(days=days)
            rows = conn.execute(
                """
                SELECT po.*, s.reliability_score
                FROM purchase_orders po
                JOIN suppliers s ON po.supplier_id = s.id
                WHERE po.status IN ('pending', 'confirmed', 'shipped')
                AND po.expected_delivery <= ?
                """,
                (cutoff.isoformat(),),
            ).fetchall()
        finally:
            conn.close()

        delivered = []
        for row in rows:
            # Simulate delivery based on reliability
            if random.random() < row["reliability_score"]:
                result = self.receive_shipment(row["id"])
                if result.get("success"):
                    delivered.append(result)
            else:
                # Delay the order
                new_expected = (
                    datetime.fromisoformat(row["expected_delivery"]) + timedelta(days=2)
                ).isoformat()
                conn = self._get_conn()
                try:
                    conn.execute(
                        """
                        UPDATE purchase_orders
                        SET expected_delivery = ?, notes = COALESCE(notes || ' ', '') || 'Delayed.'
                        WHERE id = ?
                        """,
                        (new_expected, row["id"]),
                    )
                    conn.commit()
                finally:
                    conn.close()

        return delivered

    def get_pending_po_count(self) -> int:
        """Get count of pending purchase orders."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                "SELECT COUNT(*) as count FROM purchase_orders WHERE status IN ('pending', 'confirmed', 'shipped')"
            ).fetchone()
            return row["count"]
        finally:
            conn.close()
