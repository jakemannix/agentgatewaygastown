"""Entity database with SQLite and sqlite-vec for vector search."""

import json
import uuid
from datetime import datetime
from pathlib import Path

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from mcp_tools.shared.db_utils import get_connection, get_db_path
from mcp_tools.shared.embeddings import embed_text, serialize_embedding


class EntityDatabase:
    """Database for entities and relations with vector search."""

    def __init__(self, data_dir: Path | None = None):
        self.db_path = get_db_path("entities.db", data_dir)
        self._init_db()

    def _init_db(self):
        """Initialize database schema."""
        conn = get_connection(self.db_path)
        try:
            # Entities table
            conn.execute("""
                CREATE TABLE IF NOT EXISTS entities (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    entity_type TEXT NOT NULL,
                    description TEXT,
                    properties TEXT,  -- JSON
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )
            """)

            # Relations table (subject-predicate-object)
            conn.execute("""
                CREATE TABLE IF NOT EXISTS relations (
                    id TEXT PRIMARY KEY,
                    subject_id TEXT NOT NULL,
                    predicate TEXT NOT NULL,
                    object_id TEXT NOT NULL,
                    properties TEXT,  -- JSON
                    confidence REAL DEFAULT 1.0,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY (subject_id) REFERENCES entities(id),
                    FOREIGN KEY (object_id) REFERENCES entities(id)
                )
            """)

            # Vector embeddings for entity descriptions
            conn.execute("""
                CREATE VIRTUAL TABLE IF NOT EXISTS entity_embeddings USING vec0(
                    entity_id TEXT PRIMARY KEY,
                    description_embedding FLOAT[384]
                )
            """)

            # Indices
            conn.execute("CREATE INDEX IF NOT EXISTS idx_entity_type ON entities(entity_type)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_entity_name ON entities(name)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_relation_subject ON relations(subject_id)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_relation_object ON relations(object_id)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_relation_predicate ON relations(predicate)")

            conn.commit()
        finally:
            conn.close()

    def create_entity(
        self,
        name: str,
        entity_type: str,
        description: str | None = None,
        properties: dict | None = None,
    ) -> dict:
        """Create a new entity."""
        entity_id = str(uuid.uuid4())
        now = datetime.utcnow().isoformat()

        conn = get_connection(self.db_path)
        try:
            conn.execute(
                """
                INSERT INTO entities (id, name, entity_type, description, properties, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                """,
                (entity_id, name, entity_type, description, json.dumps(properties or {}), now, now)
            )

            # Create embedding for description if provided
            if description:
                embedding = embed_text(description)
                conn.execute(
                    "INSERT INTO entity_embeddings (entity_id, description_embedding) VALUES (?, ?)",
                    (entity_id, serialize_embedding(embedding))
                )

            conn.commit()

            return {
                "id": entity_id,
                "name": name,
                "entity_type": entity_type,
                "description": description,
                "properties": properties or {},
                "created_at": now,
            }
        finally:
            conn.close()

    def get_entity(self, entity_id: str) -> dict | None:
        """Get an entity by ID."""
        conn = get_connection(self.db_path)
        try:
            row = conn.execute(
                "SELECT * FROM entities WHERE id = ?",
                (entity_id,)
            ).fetchone()

            if row:
                return {
                    "id": row["id"],
                    "name": row["name"],
                    "entity_type": row["entity_type"],
                    "description": row["description"],
                    "properties": json.loads(row["properties"]) if row["properties"] else {},
                    "created_at": row["created_at"],
                    "updated_at": row["updated_at"],
                }
            return None
        finally:
            conn.close()

    def update_entity(
        self,
        entity_id: str,
        name: str | None = None,
        description: str | None = None,
        properties: dict | None = None,
    ) -> dict | None:
        """Update an existing entity."""
        conn = get_connection(self.db_path)
        try:
            existing = self.get_entity(entity_id)
            if not existing:
                return None

            now = datetime.utcnow().isoformat()
            updates = []
            values = []

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
            values.append(entity_id)

            conn.execute(
                f"UPDATE entities SET {', '.join(updates)} WHERE id = ?",
                values
            )

            # Update embedding if description changed
            if description is not None:
                embedding = embed_text(description)
                conn.execute(
                    "DELETE FROM entity_embeddings WHERE entity_id = ?",
                    (entity_id,)
                )
                conn.execute(
                    "INSERT INTO entity_embeddings (entity_id, description_embedding) VALUES (?, ?)",
                    (entity_id, serialize_embedding(embedding))
                )

            conn.commit()
            return self.get_entity(entity_id)
        finally:
            conn.close()

    def delete_entity(self, entity_id: str) -> bool:
        """Delete an entity and its relations."""
        conn = get_connection(self.db_path)
        try:
            # Delete relations
            conn.execute("DELETE FROM relations WHERE subject_id = ? OR object_id = ?", (entity_id, entity_id))
            # Delete embedding
            conn.execute("DELETE FROM entity_embeddings WHERE entity_id = ?", (entity_id,))
            # Delete entity
            result = conn.execute("DELETE FROM entities WHERE id = ?", (entity_id,))
            conn.commit()
            return result.rowcount > 0
        finally:
            conn.close()

    def search_entities(
        self,
        query: str,
        entity_type: str | None = None,
        limit: int = 10,
    ) -> list[dict]:
        """Search entities using vector similarity on descriptions."""
        query_embedding = embed_text(query)

        conn = get_connection(self.db_path)
        try:
            # Vector similarity search
            type_filter = "AND e.entity_type = ?" if entity_type else ""
            type_value = [entity_type] if entity_type else []

            results = conn.execute(
                f"""
                SELECT e.*, ee.distance
                FROM entity_embeddings ee
                JOIN entities e ON e.id = ee.entity_id
                WHERE ee.description_embedding MATCH ?
                {type_filter}
                ORDER BY ee.distance
                LIMIT ?
                """,
                [serialize_embedding(query_embedding)] + type_value + [limit]
            ).fetchall()

            return [
                {
                    "id": row["id"],
                    "name": row["name"],
                    "entity_type": row["entity_type"],
                    "description": row["description"],
                    "properties": json.loads(row["properties"]) if row["properties"] else {},
                    "similarity": 1.0 - row["distance"],  # Convert distance to similarity
                }
                for row in results
            ]
        finally:
            conn.close()

    def create_relation(
        self,
        subject_id: str,
        predicate: str,
        object_id: str,
        properties: dict | None = None,
        confidence: float = 1.0,
    ) -> dict | None:
        """Create a relation between two entities."""
        # Verify both entities exist
        if not self.get_entity(subject_id) or not self.get_entity(object_id):
            return None

        relation_id = str(uuid.uuid4())
        now = datetime.utcnow().isoformat()

        conn = get_connection(self.db_path)
        try:
            conn.execute(
                """
                INSERT INTO relations (id, subject_id, predicate, object_id, properties, confidence, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                """,
                (relation_id, subject_id, predicate, object_id, json.dumps(properties or {}), confidence, now)
            )
            conn.commit()

            return {
                "id": relation_id,
                "subject_id": subject_id,
                "predicate": predicate,
                "object_id": object_id,
                "properties": properties or {},
                "confidence": confidence,
                "created_at": now,
            }
        finally:
            conn.close()

    def get_relations(
        self,
        entity_id: str,
        direction: str = "both",  # "outgoing", "incoming", "both"
        predicate: str | None = None,
    ) -> list[dict]:
        """Get relations for an entity."""
        conn = get_connection(self.db_path)
        try:
            results = []

            if direction in ("outgoing", "both"):
                query = "SELECT r.*, e.name as object_name FROM relations r JOIN entities e ON e.id = r.object_id WHERE r.subject_id = ?"
                params = [entity_id]
                if predicate:
                    query += " AND r.predicate = ?"
                    params.append(predicate)

                for row in conn.execute(query, params).fetchall():
                    results.append({
                        "id": row["id"],
                        "subject_id": row["subject_id"],
                        "predicate": row["predicate"],
                        "object_id": row["object_id"],
                        "object_name": row["object_name"],
                        "direction": "outgoing",
                        "properties": json.loads(row["properties"]) if row["properties"] else {},
                        "confidence": row["confidence"],
                    })

            if direction in ("incoming", "both"):
                query = "SELECT r.*, e.name as subject_name FROM relations r JOIN entities e ON e.id = r.subject_id WHERE r.object_id = ?"
                params = [entity_id]
                if predicate:
                    query += " AND r.predicate = ?"
                    params.append(predicate)

                for row in conn.execute(query, params).fetchall():
                    results.append({
                        "id": row["id"],
                        "subject_id": row["subject_id"],
                        "subject_name": row["subject_name"],
                        "predicate": row["predicate"],
                        "object_id": row["object_id"],
                        "direction": "incoming",
                        "properties": json.loads(row["properties"]) if row["properties"] else {},
                        "confidence": row["confidence"],
                    })

            return results
        finally:
            conn.close()

    def delete_relation(self, relation_id: str) -> bool:
        """Delete a relation."""
        conn = get_connection(self.db_path)
        try:
            result = conn.execute("DELETE FROM relations WHERE id = ?", (relation_id,))
            conn.commit()
            return result.rowcount > 0
        finally:
            conn.close()

    def list_entity_types(self) -> list[str]:
        """List all entity types in the database."""
        conn = get_connection(self.db_path)
        try:
            rows = conn.execute("SELECT DISTINCT entity_type FROM entities ORDER BY entity_type").fetchall()
            return [row["entity_type"] for row in rows]
        finally:
            conn.close()

    def list_predicates(self) -> list[str]:
        """List all relation predicates in the database."""
        conn = get_connection(self.db_path)
        try:
            rows = conn.execute("SELECT DISTINCT predicate FROM relations ORDER BY predicate").fetchall()
            return [row["predicate"] for row in rows]
        finally:
            conn.close()


# Global database instance
_db: EntityDatabase | None = None


def get_db() -> EntityDatabase:
    """Get the global database instance."""
    global _db
    if _db is None:
        _db = EntityDatabase()
    return _db
