"""Tag database with SQLite for content-category associations."""

import json
import uuid
from datetime import datetime
from pathlib import Path
from urllib.parse import urlparse

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from mcp_tools.shared.db_utils import get_connection, get_db_path
from mcp_tools.shared.embeddings import embed_text, serialize_embedding


class TagDatabase:
    """Database for content items and their category tags."""

    def __init__(self, data_dir: Path | None = None):
        self.db_path = get_db_path("tags.db", data_dir)
        self._init_db()

    def _init_db(self):
        """Initialize database schema."""
        conn = get_connection(self.db_path)
        try:
            # Content items table
            conn.execute("""
                CREATE TABLE IF NOT EXISTS content (
                    id TEXT PRIMARY KEY,
                    url TEXT UNIQUE,
                    title TEXT,
                    content_type TEXT,  -- 'paper', 'repo', 'article', 'model', etc.
                    summary TEXT,
                    source TEXT,  -- 'arxiv', 'github', 'huggingface', 'web'
                    metadata TEXT,  -- JSON
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )
            """)

            # Content-category tags (many-to-many)
            conn.execute("""
                CREATE TABLE IF NOT EXISTS content_tags (
                    id TEXT PRIMARY KEY,
                    content_id TEXT NOT NULL,
                    category_id TEXT NOT NULL,
                    confidence REAL DEFAULT 1.0,
                    added_by TEXT,  -- 'user', 'agent', 'auto'
                    notes TEXT,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY (content_id) REFERENCES content(id),
                    UNIQUE(content_id, category_id)
                )
            """)

            # Vector embeddings for content summaries
            conn.execute("""
                CREATE VIRTUAL TABLE IF NOT EXISTS content_embeddings USING vec0(
                    content_id TEXT PRIMARY KEY,
                    summary_embedding FLOAT[384]
                )
            """)

            # Indices
            conn.execute("CREATE INDEX IF NOT EXISTS idx_content_url ON content(url)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_content_source ON content(source)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_content_type ON content(content_type)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_tag_content ON content_tags(content_id)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_tag_category ON content_tags(category_id)")

            conn.commit()
        finally:
            conn.close()

    def _normalize_url(self, url: str) -> str:
        """Normalize a URL for consistent lookups."""
        parsed = urlparse(url)
        # Remove trailing slash and fragments
        path = parsed.path.rstrip("/")
        return f"{parsed.scheme}://{parsed.netloc}{path}"

    def create_or_update_content(
        self,
        url: str,
        title: str | None = None,
        content_type: str | None = None,
        summary: str | None = None,
        source: str | None = None,
        metadata: dict | None = None,
    ) -> dict:
        """Create or update a content item."""
        normalized_url = self._normalize_url(url)
        now = datetime.utcnow().isoformat()

        conn = get_connection(self.db_path)
        try:
            # Check if exists
            existing = conn.execute(
                "SELECT * FROM content WHERE url = ?",
                (normalized_url,)
            ).fetchone()

            if existing:
                # Update
                content_id = existing["id"]
                updates = ["updated_at = ?"]
                values = [now]

                if title is not None:
                    updates.append("title = ?")
                    values.append(title)
                if content_type is not None:
                    updates.append("content_type = ?")
                    values.append(content_type)
                if summary is not None:
                    updates.append("summary = ?")
                    values.append(summary)
                if source is not None:
                    updates.append("source = ?")
                    values.append(source)
                if metadata is not None:
                    updates.append("metadata = ?")
                    values.append(json.dumps(metadata))

                values.append(content_id)
                conn.execute(
                    f"UPDATE content SET {', '.join(updates)} WHERE id = ?",
                    values
                )

                # Update embedding if summary changed
                if summary:
                    embedding = embed_text(summary)
                    conn.execute("DELETE FROM content_embeddings WHERE content_id = ?", (content_id,))
                    conn.execute(
                        "INSERT INTO content_embeddings (content_id, summary_embedding) VALUES (?, ?)",
                        (content_id, serialize_embedding(embedding))
                    )

                action = "updated"
            else:
                # Create
                content_id = str(uuid.uuid4())
                conn.execute(
                    """
                    INSERT INTO content (id, url, title, content_type, summary, source, metadata, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (content_id, normalized_url, title, content_type, summary, source,
                     json.dumps(metadata or {}), now, now)
                )

                # Create embedding if summary provided
                if summary:
                    embedding = embed_text(summary)
                    conn.execute(
                        "INSERT INTO content_embeddings (content_id, summary_embedding) VALUES (?, ?)",
                        (content_id, serialize_embedding(embedding))
                    )

                action = "created"

            conn.commit()

            return {
                "id": content_id,
                "url": normalized_url,
                "title": title,
                "content_type": content_type,
                "summary": summary,
                "source": source,
                "metadata": metadata or {},
                "action": action,
            }
        finally:
            conn.close()

    def get_content(self, content_id: str | None = None, url: str | None = None) -> dict | None:
        """Get content by ID or URL."""
        conn = get_connection(self.db_path)
        try:
            if content_id:
                row = conn.execute(
                    "SELECT * FROM content WHERE id = ?",
                    (content_id,)
                ).fetchone()
            elif url:
                normalized_url = self._normalize_url(url)
                row = conn.execute(
                    "SELECT * FROM content WHERE url = ?",
                    (normalized_url,)
                ).fetchone()
            else:
                return None

            if not row:
                return None

            return {
                "id": row["id"],
                "url": row["url"],
                "title": row["title"],
                "content_type": row["content_type"],
                "summary": row["summary"],
                "source": row["source"],
                "metadata": json.loads(row["metadata"]) if row["metadata"] else {},
                "created_at": row["created_at"],
                "updated_at": row["updated_at"],
            }
        finally:
            conn.close()

    def delete_content(self, content_id: str) -> bool:
        """Delete content and its tags."""
        conn = get_connection(self.db_path)
        try:
            conn.execute("DELETE FROM content_tags WHERE content_id = ?", (content_id,))
            conn.execute("DELETE FROM content_embeddings WHERE content_id = ?", (content_id,))
            result = conn.execute("DELETE FROM content WHERE id = ?", (content_id,))
            conn.commit()
            return result.rowcount > 0
        finally:
            conn.close()

    def tag_content(
        self,
        content_id: str,
        category_id: str,
        confidence: float = 1.0,
        added_by: str = "agent",
        notes: str | None = None,
    ) -> dict:
        """Add a category tag to content."""
        tag_id = str(uuid.uuid4())
        now = datetime.utcnow().isoformat()

        conn = get_connection(self.db_path)
        try:
            # Check if tag already exists
            existing = conn.execute(
                "SELECT id FROM content_tags WHERE content_id = ? AND category_id = ?",
                (content_id, category_id)
            ).fetchone()

            if existing:
                # Update existing tag
                conn.execute(
                    """
                    UPDATE content_tags
                    SET confidence = ?, notes = ?
                    WHERE content_id = ? AND category_id = ?
                    """,
                    (confidence, notes, content_id, category_id)
                )
                conn.commit()
                return {
                    "id": existing["id"],
                    "content_id": content_id,
                    "category_id": category_id,
                    "confidence": confidence,
                    "action": "updated",
                }

            conn.execute(
                """
                INSERT INTO content_tags (id, content_id, category_id, confidence, added_by, notes, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                """,
                (tag_id, content_id, category_id, confidence, added_by, notes, now)
            )
            conn.commit()

            return {
                "id": tag_id,
                "content_id": content_id,
                "category_id": category_id,
                "confidence": confidence,
                "added_by": added_by,
                "action": "created",
            }
        finally:
            conn.close()

    def untag_content(self, content_id: str, category_id: str) -> bool:
        """Remove a category tag from content."""
        conn = get_connection(self.db_path)
        try:
            result = conn.execute(
                "DELETE FROM content_tags WHERE content_id = ? AND category_id = ?",
                (content_id, category_id)
            )
            conn.commit()
            return result.rowcount > 0
        finally:
            conn.close()

    def get_content_tags(self, content_id: str) -> list[dict]:
        """Get all tags for a content item."""
        conn = get_connection(self.db_path)
        try:
            rows = conn.execute(
                """
                SELECT ct.*, c.url, c.title
                FROM content_tags ct
                JOIN content c ON c.id = ct.content_id
                WHERE ct.content_id = ?
                ORDER BY ct.confidence DESC
                """,
                (content_id,)
            ).fetchall()

            return [
                {
                    "tag_id": row["id"],
                    "category_id": row["category_id"],
                    "confidence": row["confidence"],
                    "added_by": row["added_by"],
                    "notes": row["notes"],
                }
                for row in rows
            ]
        finally:
            conn.close()

    def search_by_category(
        self,
        category_id: str,
        include_descendants: bool = False,
        limit: int = 50,
    ) -> list[dict]:
        """Find all content tagged with a category."""
        conn = get_connection(self.db_path)
        try:
            if include_descendants:
                # Would need category service to resolve descendants
                # For now, just do exact match
                pass

            rows = conn.execute(
                """
                SELECT c.*, ct.confidence, ct.category_id
                FROM content c
                JOIN content_tags ct ON ct.content_id = c.id
                WHERE ct.category_id = ?
                ORDER BY ct.confidence DESC, c.updated_at DESC
                LIMIT ?
                """,
                (category_id, limit)
            ).fetchall()

            return [
                {
                    "id": row["id"],
                    "url": row["url"],
                    "title": row["title"],
                    "content_type": row["content_type"],
                    "summary": row["summary"],
                    "source": row["source"],
                    "tag_confidence": row["confidence"],
                }
                for row in rows
            ]
        finally:
            conn.close()

    def search_content(
        self,
        query: str,
        source: str | None = None,
        content_type: str | None = None,
        limit: int = 20,
    ) -> list[dict]:
        """Search content using semantic similarity on summaries."""
        query_embedding = embed_text(query)

        conn = get_connection(self.db_path)
        try:
            filters = []
            params = [serialize_embedding(query_embedding)]

            if source:
                filters.append("c.source = ?")
                params.append(source)
            if content_type:
                filters.append("c.content_type = ?")
                params.append(content_type)

            where_clause = f"AND {' AND '.join(filters)}" if filters else ""
            params.append(limit)

            results = conn.execute(
                f"""
                SELECT c.*, ce.distance
                FROM content_embeddings ce
                JOIN content c ON c.id = ce.content_id
                WHERE ce.summary_embedding MATCH ?
                {where_clause}
                ORDER BY ce.distance
                LIMIT ?
                """,
                params
            ).fetchall()

            return [
                {
                    "id": row["id"],
                    "url": row["url"],
                    "title": row["title"],
                    "content_type": row["content_type"],
                    "summary": row["summary"],
                    "source": row["source"],
                    "similarity": 1.0 - row["distance"],
                }
                for row in results
            ]
        finally:
            conn.close()


# Global database instance
_db: TagDatabase | None = None


def get_db() -> TagDatabase:
    """Get the global database instance."""
    global _db
    if _db is None:
        _db = TagDatabase()
    return _db
