"""Database operations for the catalog service."""

import sqlite3
import struct
from datetime import datetime
from pathlib import Path
from typing import Optional

import sqlite_vec

from ..shared.db_utils import get_connection, get_db_path, rows_to_dicts

DB_NAME = "catalog.db"

SCHEMA = """
CREATE TABLE IF NOT EXISTS products (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    price REAL NOT NULL,
    cost REAL NOT NULL,
    category TEXT,
    stock_quantity INTEGER DEFAULT 0,
    reorder_threshold INTEGER DEFAULT 10,
    created_at TEXT,
    updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_products_category ON products(category);
CREATE INDEX IF NOT EXISTS idx_products_stock ON products(stock_quantity);
"""


def serialize_f32(vector: list[float]) -> bytes:
    """Serialize a list of floats to bytes for sqlite-vec."""
    return struct.pack(f"{len(vector)}f", *vector)


def deserialize_f32(data: bytes) -> list[float]:
    """Deserialize bytes to a list of floats from sqlite-vec."""
    n = len(data) // 4
    return list(struct.unpack(f"{n}f", data))


class CatalogDatabase:
    """Database operations for product catalog."""

    def __init__(self, data_dir: Optional[Path] = None):
        self.db_path = get_db_path(DB_NAME, data_dir)
        self._init_db()

    def _get_conn(self) -> sqlite3.Connection:
        conn = get_connection(self.db_path)
        conn.enable_load_extension(True)
        sqlite_vec.load(conn)
        conn.enable_load_extension(False)
        return conn

    def _init_db(self):
        """Initialize the database schema."""
        conn = self._get_conn()
        try:
            conn.executescript(SCHEMA)
            # Create vector table for embeddings
            conn.execute("""
                CREATE VIRTUAL TABLE IF NOT EXISTS product_embeddings USING vec0(
                    product_id TEXT PRIMARY KEY,
                    embedding float[384]
                )
            """)
            conn.commit()
        finally:
            conn.close()

    def get_product(self, product_id: str) -> Optional[dict]:
        """Get a product by ID."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                "SELECT * FROM products WHERE id = ?", (product_id,)
            ).fetchone()
            return dict(row) if row else None
        finally:
            conn.close()

    def list_products(
        self,
        category: Optional[str] = None,
        page: int = 1,
        limit: int = 20,
    ) -> list[dict]:
        """List products with optional filtering."""
        conn = self._get_conn()
        try:
            offset = (page - 1) * limit
            if category:
                rows = conn.execute(
                    "SELECT * FROM products WHERE category = ? ORDER BY name LIMIT ? OFFSET ?",
                    (category, limit, offset),
                ).fetchall()
            else:
                rows = conn.execute(
                    "SELECT * FROM products ORDER BY name LIMIT ? OFFSET ?",
                    (limit, offset),
                ).fetchall()
            return rows_to_dicts(rows)
        finally:
            conn.close()

    def search_products(
        self,
        query_embedding: list[float],
        category: Optional[str] = None,
        max_results: int = 10,
    ) -> list[dict]:
        """Search products using vector similarity."""
        conn = self._get_conn()
        try:
            query_bytes = serialize_f32(query_embedding)

            if category:
                # sqlite-vec requires k=? for KNN queries
                rows = conn.execute(
                    """
                    SELECT p.*, e.distance
                    FROM product_embeddings e
                    JOIN products p ON e.product_id = p.id
                    WHERE p.category = ?
                    AND e.embedding MATCH ?
                    AND k = ?
                    ORDER BY e.distance
                    """,
                    (category, query_bytes, max_results),
                ).fetchall()
            else:
                # sqlite-vec requires k=? for KNN queries
                rows = conn.execute(
                    """
                    SELECT p.*, e.distance
                    FROM product_embeddings e
                    JOIN products p ON e.product_id = p.id
                    WHERE e.embedding MATCH ?
                    AND k = ?
                    ORDER BY e.distance
                    """,
                    (query_bytes, max_results),
                ).fetchall()
            return rows_to_dicts(rows)
        finally:
            conn.close()

    def get_categories(self) -> list[str]:
        """Get all distinct categories."""
        conn = self._get_conn()
        try:
            rows = conn.execute(
                "SELECT DISTINCT category FROM products WHERE category IS NOT NULL ORDER BY category"
            ).fetchall()
            return [row["category"] for row in rows]
        finally:
            conn.close()

    def create_product(
        self,
        product_id: str,
        name: str,
        price: float,
        cost: float,
        description: Optional[str] = None,
        category: Optional[str] = None,
        stock_quantity: int = 0,
        reorder_threshold: int = 10,
        embedding: Optional[list[float]] = None,
    ) -> dict:
        """Create a new product."""
        conn = self._get_conn()
        try:
            now = datetime.utcnow().isoformat()
            conn.execute(
                """
                INSERT INTO products (id, name, description, price, cost, category,
                                     stock_quantity, reorder_threshold, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    product_id,
                    name,
                    description,
                    price,
                    cost,
                    category,
                    stock_quantity,
                    reorder_threshold,
                    now,
                    now,
                ),
            )
            if embedding:
                conn.execute(
                    "INSERT INTO product_embeddings (product_id, embedding) VALUES (?, ?)",
                    (product_id, serialize_f32(embedding)),
                )
            conn.commit()
            return self.get_product(product_id)
        finally:
            conn.close()

    def update_product_embedding(self, product_id: str, embedding: list[float]):
        """Update or insert a product embedding."""
        conn = self._get_conn()
        try:
            # Delete existing embedding if present
            conn.execute(
                "DELETE FROM product_embeddings WHERE product_id = ?", (product_id,)
            )
            # Insert new embedding
            conn.execute(
                "INSERT INTO product_embeddings (product_id, embedding) VALUES (?, ?)",
                (product_id, serialize_f32(embedding)),
            )
            conn.commit()
        finally:
            conn.close()

    def update_stock(self, product_id: str, quantity_change: int) -> Optional[dict]:
        """Update product stock quantity."""
        conn = self._get_conn()
        try:
            now = datetime.utcnow().isoformat()
            conn.execute(
                """
                UPDATE products
                SET stock_quantity = stock_quantity + ?, updated_at = ?
                WHERE id = ?
                """,
                (quantity_change, now, product_id),
            )
            conn.commit()
            return self.get_product(product_id)
        finally:
            conn.close()

    def get_low_stock_products(self, threshold: Optional[int] = None) -> list[dict]:
        """Get products with stock below threshold."""
        conn = self._get_conn()
        try:
            if threshold is not None:
                rows = conn.execute(
                    """
                    SELECT *, (? - stock_quantity) as deficit
                    FROM products
                    WHERE stock_quantity < ?
                    ORDER BY deficit DESC
                    """,
                    (threshold, threshold),
                ).fetchall()
            else:
                rows = conn.execute(
                    """
                    SELECT *, (reorder_threshold - stock_quantity) as deficit
                    FROM products
                    WHERE stock_quantity < reorder_threshold
                    ORDER BY deficit DESC
                    """
                ).fetchall()
            return rows_to_dicts(rows)
        finally:
            conn.close()

    def get_inventory_stats(self) -> dict:
        """Get inventory statistics."""
        conn = self._get_conn()
        try:
            row = conn.execute(
                """
                SELECT
                    COUNT(*) as total_products,
                    SUM(stock_quantity) as total_units,
                    SUM(stock_quantity * cost) as total_value,
                    SUM(CASE WHEN stock_quantity < reorder_threshold THEN 1 ELSE 0 END) as low_stock_count,
                    SUM(CASE WHEN stock_quantity = 0 THEN 1 ELSE 0 END) as out_of_stock_count
                FROM products
                """
            ).fetchone()
            return dict(row)
        finally:
            conn.close()
