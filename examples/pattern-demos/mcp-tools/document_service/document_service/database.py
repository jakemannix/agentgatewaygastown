"""SQLite database with sqlite-vec for vector storage."""

from __future__ import annotations

import json
import sqlite3
import struct
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any

import sqlite_vec


@dataclass
class Document:
    """Represents a document in the store."""

    id: str
    title: str
    content: str
    metadata: dict[str, Any]
    created_at: datetime
    updated_at: datetime


@dataclass
class Chunk:
    """Represents a chunk of a document."""

    id: str
    document_id: str
    content: str
    chunk_index: int
    embedding: list[float] | None = None


@dataclass
class SearchResult:
    """Represents a search result."""

    document_id: str
    chunk_id: str
    content: str
    score: float
    document_title: str


def serialize_f32(vector: list[float]) -> bytes:
    """Serialize a float32 vector to bytes for sqlite-vec."""
    return struct.pack(f"{len(vector)}f", *vector)


class DocumentDatabase:
    """SQLite database for document storage with vector search."""

    def __init__(self, db_path: str | Path = ":memory:", embedding_dim: int = 384):
        """Initialize the database.

        Args:
            db_path: Path to the SQLite database file, or ":memory:" for in-memory.
            embedding_dim: Dimension of the embedding vectors.
        """
        self.db_path = str(db_path)
        self.embedding_dim = embedding_dim
        self._conn: sqlite3.Connection | None = None
        self._init_db()

    def _get_conn(self) -> sqlite3.Connection:
        """Get the database connection."""
        if self._conn is None:
            self._conn = sqlite3.connect(self.db_path)
            self._conn.row_factory = sqlite3.Row
            self._conn.enable_load_extension(True)
            sqlite_vec.load(self._conn)
            self._conn.enable_load_extension(False)
        return self._conn

    def _init_db(self) -> None:
        """Initialize the database schema."""
        conn = self._get_conn()

        # Create documents table
        conn.execute("""
            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        """)

        # Create chunks table
        conn.execute("""
            CREATE TABLE IF NOT EXISTS chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                content TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )
        """)

        # Create virtual table for vector search using sqlite-vec
        conn.execute(f"""
            CREATE VIRTUAL TABLE IF NOT EXISTS chunk_embeddings USING vec0(
                chunk_id TEXT PRIMARY KEY,
                embedding float[{self.embedding_dim}]
            )
        """)

        # Create indexes
        conn.execute("""
            CREATE INDEX IF NOT EXISTS idx_chunks_document_id
            ON chunks(document_id)
        """)

        conn.commit()

    def create_document(
        self,
        doc_id: str,
        title: str,
        content: str,
        metadata: dict[str, Any] | None = None,
    ) -> Document:
        """Create a new document.

        Args:
            doc_id: Unique document identifier.
            title: Document title.
            content: Document content.
            metadata: Optional metadata dictionary.

        Returns:
            The created document.
        """
        conn = self._get_conn()
        now = datetime.utcnow().isoformat()
        metadata = metadata or {}

        conn.execute(
            """
            INSERT INTO documents (id, title, content, metadata, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            """,
            (doc_id, title, content, json.dumps(metadata), now, now),
        )
        conn.commit()

        return Document(
            id=doc_id,
            title=title,
            content=content,
            metadata=metadata,
            created_at=datetime.fromisoformat(now),
            updated_at=datetime.fromisoformat(now),
        )

    def get_document(self, doc_id: str) -> Document | None:
        """Get a document by ID.

        Args:
            doc_id: The document ID.

        Returns:
            The document if found, None otherwise.
        """
        conn = self._get_conn()
        row = conn.execute(
            "SELECT * FROM documents WHERE id = ?", (doc_id,)
        ).fetchone()

        if row is None:
            return None

        return Document(
            id=row["id"],
            title=row["title"],
            content=row["content"],
            metadata=json.loads(row["metadata"]),
            created_at=datetime.fromisoformat(row["created_at"]),
            updated_at=datetime.fromisoformat(row["updated_at"]),
        )

    def list_documents(
        self, limit: int = 100, offset: int = 0
    ) -> list[Document]:
        """List all documents.

        Args:
            limit: Maximum number of documents to return.
            offset: Number of documents to skip.

        Returns:
            List of documents.
        """
        conn = self._get_conn()
        rows = conn.execute(
            "SELECT * FROM documents ORDER BY updated_at DESC LIMIT ? OFFSET ?",
            (limit, offset),
        ).fetchall()

        return [
            Document(
                id=row["id"],
                title=row["title"],
                content=row["content"],
                metadata=json.loads(row["metadata"]),
                created_at=datetime.fromisoformat(row["created_at"]),
                updated_at=datetime.fromisoformat(row["updated_at"]),
            )
            for row in rows
        ]

    def update_document(
        self,
        doc_id: str,
        title: str | None = None,
        content: str | None = None,
        metadata: dict[str, Any] | None = None,
    ) -> Document | None:
        """Update a document.

        Args:
            doc_id: The document ID.
            title: New title (optional).
            content: New content (optional).
            metadata: New metadata (optional).

        Returns:
            The updated document if found, None otherwise.
        """
        doc = self.get_document(doc_id)
        if doc is None:
            return None

        conn = self._get_conn()
        now = datetime.utcnow().isoformat()

        new_title = title if title is not None else doc.title
        new_content = content if content is not None else doc.content
        new_metadata = metadata if metadata is not None else doc.metadata

        conn.execute(
            """
            UPDATE documents
            SET title = ?, content = ?, metadata = ?, updated_at = ?
            WHERE id = ?
            """,
            (new_title, new_content, json.dumps(new_metadata), now, doc_id),
        )
        conn.commit()

        return Document(
            id=doc_id,
            title=new_title,
            content=new_content,
            metadata=new_metadata,
            created_at=doc.created_at,
            updated_at=datetime.fromisoformat(now),
        )

    def delete_document(self, doc_id: str) -> bool:
        """Delete a document and its chunks.

        Args:
            doc_id: The document ID.

        Returns:
            True if the document was deleted, False if not found.
        """
        conn = self._get_conn()

        # Delete chunk embeddings first
        chunk_ids = conn.execute(
            "SELECT id FROM chunks WHERE document_id = ?", (doc_id,)
        ).fetchall()
        for row in chunk_ids:
            conn.execute(
                "DELETE FROM chunk_embeddings WHERE chunk_id = ?",
                (row["id"],),
            )

        # Delete chunks
        conn.execute("DELETE FROM chunks WHERE document_id = ?", (doc_id,))

        # Delete document
        cursor = conn.execute("DELETE FROM documents WHERE id = ?", (doc_id,))
        conn.commit()

        return cursor.rowcount > 0

    def store_chunks(
        self, document_id: str, chunks: list[Chunk]
    ) -> list[Chunk]:
        """Store chunks for a document.

        Args:
            document_id: The document ID.
            chunks: List of chunks to store.

        Returns:
            The stored chunks.
        """
        conn = self._get_conn()

        # Delete existing chunks for this document
        existing_chunk_ids = conn.execute(
            "SELECT id FROM chunks WHERE document_id = ?", (document_id,)
        ).fetchall()
        for row in existing_chunk_ids:
            conn.execute(
                "DELETE FROM chunk_embeddings WHERE chunk_id = ?",
                (row["id"],),
            )
        conn.execute("DELETE FROM chunks WHERE document_id = ?", (document_id,))

        # Insert new chunks
        for chunk in chunks:
            conn.execute(
                """
                INSERT INTO chunks (id, document_id, content, chunk_index)
                VALUES (?, ?, ?, ?)
                """,
                (chunk.id, document_id, chunk.content, chunk.chunk_index),
            )

            # Store embedding if provided
            if chunk.embedding is not None:
                conn.execute(
                    """
                    INSERT INTO chunk_embeddings (chunk_id, embedding)
                    VALUES (?, ?)
                    """,
                    (chunk.id, serialize_f32(chunk.embedding)),
                )

        conn.commit()
        return chunks

    def get_chunks(self, document_id: str) -> list[Chunk]:
        """Get all chunks for a document.

        Args:
            document_id: The document ID.

        Returns:
            List of chunks ordered by chunk_index.
        """
        conn = self._get_conn()
        rows = conn.execute(
            """
            SELECT * FROM chunks
            WHERE document_id = ?
            ORDER BY chunk_index
            """,
            (document_id,),
        ).fetchall()

        return [
            Chunk(
                id=row["id"],
                document_id=row["document_id"],
                content=row["content"],
                chunk_index=row["chunk_index"],
            )
            for row in rows
        ]

    def search_similar(
        self, query_embedding: list[float], limit: int = 10
    ) -> list[SearchResult]:
        """Search for similar chunks using vector similarity.

        Args:
            query_embedding: The query embedding vector.
            limit: Maximum number of results.

        Returns:
            List of search results ordered by similarity.
        """
        conn = self._get_conn()

        # Use sqlite-vec to find similar embeddings
        rows = conn.execute(
            """
            SELECT
                ce.chunk_id,
                ce.distance,
                c.document_id,
                c.content,
                d.title as document_title
            FROM chunk_embeddings ce
            JOIN chunks c ON c.id = ce.chunk_id
            JOIN documents d ON d.id = c.document_id
            WHERE ce.embedding MATCH ?
            ORDER BY ce.distance
            LIMIT ?
            """,
            (serialize_f32(query_embedding), limit),
        ).fetchall()

        return [
            SearchResult(
                document_id=row["document_id"],
                chunk_id=row["chunk_id"],
                content=row["content"],
                score=1.0 - row["distance"],  # Convert distance to similarity
                document_title=row["document_title"],
            )
            for row in rows
        ]

    def close(self) -> None:
        """Close the database connection."""
        if self._conn is not None:
            self._conn.close()
            self._conn = None
