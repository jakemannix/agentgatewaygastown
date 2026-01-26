"""Category database with SQLite and sqlite-vec for vector search."""

import json
import uuid
from datetime import datetime
from pathlib import Path

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from mcp_tools.shared.db_utils import get_connection, get_db_path
from mcp_tools.shared.embeddings import embed_text, serialize_embedding


class CategoryDatabase:
    """Database for hierarchical categories with semantic search."""

    def __init__(self, data_dir: Path | None = None):
        self.db_path = get_db_path("categories.db", data_dir)
        self._init_db()

    def _init_db(self):
        """Initialize database schema."""
        conn = get_connection(self.db_path)
        try:
            # Categories table (hierarchical)
            conn.execute("""
                CREATE TABLE IF NOT EXISTS categories (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    parent_id TEXT,
                    description TEXT,
                    properties TEXT,  -- JSON
                    level INTEGER DEFAULT 0,
                    path TEXT,  -- Materialized path for fast ancestor queries
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    FOREIGN KEY (parent_id) REFERENCES categories(id)
                )
            """)

            # Vector embeddings for category descriptions
            conn.execute("""
                CREATE VIRTUAL TABLE IF NOT EXISTS category_embeddings USING vec0(
                    category_id TEXT PRIMARY KEY,
                    description_embedding FLOAT[384]
                )
            """)

            # Indices
            conn.execute("CREATE INDEX IF NOT EXISTS idx_category_parent ON categories(parent_id)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_category_name ON categories(name)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_category_path ON categories(path)")

            conn.commit()
        finally:
            conn.close()

    def _get_path(self, parent_id: str | None, category_id: str) -> str:
        """Generate materialized path for a category."""
        if not parent_id:
            return category_id

        conn = get_connection(self.db_path)
        try:
            parent = conn.execute("SELECT path FROM categories WHERE id = ?", (parent_id,)).fetchone()
            if parent:
                return f"{parent['path']}/{category_id}"
            return category_id
        finally:
            conn.close()

    def _get_level(self, parent_id: str | None) -> int:
        """Get the level for a category based on its parent."""
        if not parent_id:
            return 0

        conn = get_connection(self.db_path)
        try:
            parent = conn.execute("SELECT level FROM categories WHERE id = ?", (parent_id,)).fetchone()
            return (parent["level"] + 1) if parent else 0
        finally:
            conn.close()

    def create_category(
        self,
        name: str,
        parent_id: str | None = None,
        description: str | None = None,
        properties: dict | None = None,
    ) -> dict:
        """Create a new category."""
        category_id = str(uuid.uuid4())
        now = datetime.utcnow().isoformat()
        level = self._get_level(parent_id)
        path = self._get_path(parent_id, category_id)

        # Combine name and description for embedding
        text_to_embed = f"{name}. {description}" if description else name

        conn = get_connection(self.db_path)
        try:
            conn.execute(
                """
                INSERT INTO categories (id, name, parent_id, description, properties, level, path, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (category_id, name, parent_id, description, json.dumps(properties or {}), level, path, now, now)
            )

            # Create embedding
            embedding = embed_text(text_to_embed)
            conn.execute(
                "INSERT INTO category_embeddings (category_id, description_embedding) VALUES (?, ?)",
                (category_id, serialize_embedding(embedding))
            )

            conn.commit()

            return {
                "id": category_id,
                "name": name,
                "parent_id": parent_id,
                "description": description,
                "properties": properties or {},
                "level": level,
                "path": path,
                "created_at": now,
            }
        finally:
            conn.close()

    def get_category(self, category_id: str) -> dict | None:
        """Get a category by ID with its ancestors."""
        conn = get_connection(self.db_path)
        try:
            row = conn.execute(
                "SELECT * FROM categories WHERE id = ?",
                (category_id,)
            ).fetchone()

            if not row:
                return None

            category = {
                "id": row["id"],
                "name": row["name"],
                "parent_id": row["parent_id"],
                "description": row["description"],
                "properties": json.loads(row["properties"]) if row["properties"] else {},
                "level": row["level"],
                "path": row["path"],
                "created_at": row["created_at"],
                "updated_at": row["updated_at"],
            }

            # Get ancestor chain
            ancestors = []
            current_parent_id = row["parent_id"]
            while current_parent_id:
                parent = conn.execute(
                    "SELECT id, name, parent_id FROM categories WHERE id = ?",
                    (current_parent_id,)
                ).fetchone()
                if parent:
                    ancestors.insert(0, {"id": parent["id"], "name": parent["name"]})
                    current_parent_id = parent["parent_id"]
                else:
                    break

            category["ancestors"] = ancestors
            return category
        finally:
            conn.close()

    def update_category(
        self,
        category_id: str,
        name: str | None = None,
        description: str | None = None,
        properties: dict | None = None,
    ) -> dict | None:
        """Update an existing category."""
        conn = get_connection(self.db_path)
        try:
            existing = self.get_category(category_id)
            if not existing:
                return None

            now = datetime.utcnow().isoformat()
            updates = []
            values = []

            new_name = name or existing["name"]
            new_description = description if description is not None else existing.get("description")

            if name is not None:
                updates.append("name = ?")
                values.append(name)
            if description is not None:
                updates.append("description = ?")
                values.append(description)
            if properties is not None:
                updates.append("properties = ?")
                values.append(json.dumps(properties))

            updates.append("updated_at = ?")
            values.append(now)
            values.append(category_id)

            conn.execute(
                f"UPDATE categories SET {', '.join(updates)} WHERE id = ?",
                values
            )

            # Update embedding
            text_to_embed = f"{new_name}. {new_description}" if new_description else new_name
            embedding = embed_text(text_to_embed)
            conn.execute("DELETE FROM category_embeddings WHERE category_id = ?", (category_id,))
            conn.execute(
                "INSERT INTO category_embeddings (category_id, description_embedding) VALUES (?, ?)",
                (category_id, serialize_embedding(embedding))
            )

            conn.commit()
            return self.get_category(category_id)
        finally:
            conn.close()

    def delete_category(self, category_id: str, recursive: bool = False) -> dict:
        """Delete a category. If recursive, also delete all descendants."""
        conn = get_connection(self.db_path)
        try:
            category = self.get_category(category_id)
            if not category:
                return {"success": False, "error": "Category not found", "deleted_count": 0}

            deleted_ids = []

            if recursive:
                # Find all descendants using path prefix
                path_prefix = category["path"]
                descendants = conn.execute(
                    "SELECT id FROM categories WHERE path LIKE ?",
                    (f"{path_prefix}/%",)
                ).fetchall()

                for desc in descendants:
                    conn.execute("DELETE FROM category_embeddings WHERE category_id = ?", (desc["id"],))
                    conn.execute("DELETE FROM categories WHERE id = ?", (desc["id"],))
                    deleted_ids.append(desc["id"])

            # Check for remaining children if not recursive
            if not recursive:
                children = conn.execute(
                    "SELECT COUNT(*) as count FROM categories WHERE parent_id = ?",
                    (category_id,)
                ).fetchone()
                if children["count"] > 0:
                    return {
                        "success": False,
                        "error": "Category has children. Use recursive=true to delete all.",
                        "children_count": children["count"],
                    }

            # Delete the category itself
            conn.execute("DELETE FROM category_embeddings WHERE category_id = ?", (category_id,))
            conn.execute("DELETE FROM categories WHERE id = ?", (category_id,))
            deleted_ids.append(category_id)

            conn.commit()

            return {
                "success": True,
                "deleted_count": len(deleted_ids),
                "deleted_ids": deleted_ids,
            }
        finally:
            conn.close()

    def search_categories(
        self,
        query: str,
        limit: int = 10,
    ) -> list[dict]:
        """Search categories using semantic similarity."""
        query_embedding = embed_text(query)

        conn = get_connection(self.db_path)
        try:
            results = conn.execute(
                """
                SELECT c.*, ce.distance
                FROM category_embeddings ce
                JOIN categories c ON c.id = ce.category_id
                WHERE ce.description_embedding MATCH ?
                ORDER BY ce.distance
                LIMIT ?
                """,
                [serialize_embedding(query_embedding), limit]
            ).fetchall()

            return [
                {
                    "id": row["id"],
                    "name": row["name"],
                    "parent_id": row["parent_id"],
                    "description": row["description"],
                    "level": row["level"],
                    "path": row["path"],
                    "similarity": 1.0 - row["distance"],
                }
                for row in results
            ]
        finally:
            conn.close()

    def get_children(self, parent_id: str | None = None) -> list[dict]:
        """Get direct children of a category (or root categories if parent_id is None)."""
        conn = get_connection(self.db_path)
        try:
            if parent_id is None:
                rows = conn.execute(
                    "SELECT * FROM categories WHERE parent_id IS NULL ORDER BY name"
                ).fetchall()
            else:
                rows = conn.execute(
                    "SELECT * FROM categories WHERE parent_id = ? ORDER BY name",
                    (parent_id,)
                ).fetchall()

            return [
                {
                    "id": row["id"],
                    "name": row["name"],
                    "parent_id": row["parent_id"],
                    "description": row["description"],
                    "level": row["level"],
                }
                for row in rows
            ]
        finally:
            conn.close()

    def get_tree(self, root_id: str | None = None, max_depth: int = 10) -> dict:
        """Get a category tree starting from a root."""
        conn = get_connection(self.db_path)
        try:
            def build_tree(parent_id: str | None, current_depth: int) -> list[dict]:
                if current_depth >= max_depth:
                    return []

                children = self.get_children(parent_id)
                for child in children:
                    child["children"] = build_tree(child["id"], current_depth + 1)
                return children

            if root_id:
                root = self.get_category(root_id)
                if not root:
                    return {"error": "Root category not found"}
                root["children"] = build_tree(root_id, 0)
                return root
            else:
                return {"roots": build_tree(None, 0)}
        finally:
            conn.close()


# Global database instance
_db: CategoryDatabase | None = None


def get_db() -> CategoryDatabase:
    """Get the global database instance."""
    global _db
    if _db is None:
        _db = CategoryDatabase()
    return _db
