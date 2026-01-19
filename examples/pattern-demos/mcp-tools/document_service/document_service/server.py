"""MCP server for document management with semantic search.

Uses FastMCP for consistent transport support (stdio, streamable-http).
"""

from __future__ import annotations

import json
import logging
import os
import uuid
from typing import Any

from mcp.server.fastmcp import FastMCP

from .database import DocumentDatabase, Chunk
from .embeddings import get_embedding_model
from .chunking import chunk_text, generate_chunk_id

logger = logging.getLogger(__name__)

# Initialize FastMCP server
mcp = FastMCP(
    name="document-service",
    version="0.1.0",
)

# Configuration from environment
DB_PATH = os.environ.get("DOCUMENT_SERVICE_DB", ":memory:")
EMBEDDING_MODEL = os.environ.get("DOCUMENT_SERVICE_MODEL", "all-MiniLM-L6-v2")

# Lazy initialization (happens on first tool call)
_db: DocumentDatabase | None = None
_embedder = None


def _get_db() -> DocumentDatabase:
    """Get or initialize the database."""
    global _db, _embedder
    if _db is None:
        _embedder = get_embedding_model(EMBEDDING_MODEL)
        _db = DocumentDatabase(DB_PATH, embedding_dim=_embedder.embedding_dim)
        logger.info(f"Initialized document database at {DB_PATH}")
    return _db


def _get_embedder():
    """Get the embedding model."""
    global _embedder
    if _embedder is None:
        _get_db()  # This also initializes the embedder
    return _embedder


@mcp.tool()
def create_document(
    title: str,
    content: str,
    metadata: dict[str, Any] | None = None,
    chunk_size: int = 500,
    chunk_overlap: int = 50,
) -> dict[str, Any]:
    """Create a new document with automatic chunking and embedding for semantic search.

    Args:
        title: The document title
        content: The document content
        metadata: Optional metadata as key-value pairs
        chunk_size: Target chunk size in characters (default: 500)
        chunk_overlap: Overlap between chunks in characters (default: 50)

    Returns:
        Created document info with ID and chunk count
    """
    db = _get_db()
    embedder = _get_embedder()

    # Generate document ID
    doc_id = str(uuid.uuid4())

    # Create document
    doc = db.create_document(doc_id, title, content, metadata or {})

    # Chunk the content
    text_chunks = chunk_text(
        content,
        chunk_size=chunk_size,
        chunk_overlap=chunk_overlap,
    )

    # Generate embeddings for chunks
    chunk_count = 0
    if text_chunks:
        embeddings = embedder.embed_batch(text_chunks)
        chunks = [
            Chunk(
                id=generate_chunk_id(doc_id, i),
                document_id=doc_id,
                content=text,
                chunk_index=i,
                embedding=embedding,
            )
            for i, (text, embedding) in enumerate(zip(text_chunks, embeddings))
        ]
        db.store_chunks(doc_id, chunks)
        chunk_count = len(chunks)

    return {
        "id": doc_id,
        "title": title,
        "chunks_created": chunk_count,
        "message": f"Created document '{title}' with {chunk_count} chunks for semantic search.",
    }


@mcp.tool()
def get_document(
    id: str,
    include_chunks: bool = False,
) -> dict[str, Any]:
    """Retrieve a document by its ID.

    Args:
        id: The document ID
        include_chunks: Whether to include chunks in the response

    Returns:
        Document details or error if not found
    """
    db = _get_db()
    doc = db.get_document(id)

    if doc is None:
        return {"error": f"Document not found: {id}"}

    result = {
        "id": doc.id,
        "title": doc.title,
        "content": doc.content,
        "metadata": doc.metadata,
        "created_at": doc.created_at.isoformat(),
        "updated_at": doc.updated_at.isoformat(),
    }

    if include_chunks:
        chunks = db.get_chunks(id)
        result["chunks"] = [
            {"id": c.id, "content": c.content, "index": c.chunk_index}
            for c in chunks
        ]

    return result


@mcp.tool()
def list_documents(
    limit: int = 100,
    offset: int = 0,
) -> dict[str, Any]:
    """List all documents with pagination.

    Args:
        limit: Maximum number of documents to return (default: 100)
        offset: Number of documents to skip (default: 0)

    Returns:
        List of documents with count
    """
    db = _get_db()
    docs = db.list_documents(limit=limit, offset=offset)

    return {
        "documents": [
            {
                "id": doc.id,
                "title": doc.title,
                "metadata": doc.metadata,
                "created_at": doc.created_at.isoformat(),
                "updated_at": doc.updated_at.isoformat(),
            }
            for doc in docs
        ],
        "count": len(docs),
        "limit": limit,
        "offset": offset,
    }


@mcp.tool()
def update_document(
    id: str,
    title: str | None = None,
    content: str | None = None,
    metadata: dict[str, Any] | None = None,
    chunk_size: int = 500,
    chunk_overlap: int = 50,
) -> dict[str, Any]:
    """Update an existing document. Only provided fields will be updated.

    Args:
        id: The document ID to update
        title: New title (optional)
        content: New content (optional, triggers re-chunking and re-embedding)
        metadata: New metadata (optional)
        chunk_size: Chunk size if content is updated (default: 500)
        chunk_overlap: Chunk overlap if content is updated (default: 50)

    Returns:
        Updated document info or error if not found
    """
    db = _get_db()
    embedder = _get_embedder()

    doc = db.update_document(id, title=title, content=content, metadata=metadata)
    if doc is None:
        return {"error": f"Document not found: {id}"}

    # Re-chunk and re-embed if content was updated
    chunk_count = 0
    if content is not None:
        text_chunks = chunk_text(
            content,
            chunk_size=chunk_size,
            chunk_overlap=chunk_overlap,
        )
        if text_chunks:
            embeddings = embedder.embed_batch(text_chunks)
            chunks = [
                Chunk(
                    id=generate_chunk_id(id, i),
                    document_id=id,
                    content=text,
                    chunk_index=i,
                    embedding=embedding,
                )
                for i, (text, embedding) in enumerate(zip(text_chunks, embeddings))
            ]
            db.store_chunks(id, chunks)
            chunk_count = len(chunks)

        return {
            "id": id,
            "title": doc.title,
            "chunks_updated": chunk_count,
            "message": f"Updated document '{doc.title}' with {chunk_count} chunks.",
        }

    return {
        "id": id,
        "title": doc.title,
        "message": f"Updated document '{doc.title}'.",
    }


@mcp.tool()
def delete_document(id: str) -> dict[str, Any]:
    """Delete a document and all its chunks.

    Args:
        id: The document ID to delete

    Returns:
        Deletion status
    """
    db = _get_db()
    deleted = db.delete_document(id)

    if not deleted:
        return {"error": f"Document not found: {id}", "deleted": False}

    return {"deleted": True, "id": id, "message": f"Deleted document {id}"}


@mcp.tool()
def search_documents(
    query: str,
    limit: int = 10,
) -> dict[str, Any]:
    """Search documents using semantic similarity.

    Uses vector embeddings to find document chunks that are semantically
    similar to the query text. Returns relevant chunks ranked by relevance.

    Args:
        query: The search query
        limit: Maximum number of results (default: 10)

    Returns:
        List of matching chunks with relevance scores
    """
    db = _get_db()
    embedder = _get_embedder()

    # Generate query embedding
    query_embedding = embedder.embed(query)

    # Search for similar chunks
    results = db.search_similar(query_embedding, limit=limit)

    if not results:
        return {"results": [], "count": 0, "message": "No matching documents found."}

    return {
        "results": [
            {
                "document_id": r.document_id,
                "document_title": r.document_title,
                "chunk_id": r.chunk_id,
                "score": round(r.score, 4),
                "content": r.content,
            }
            for r in results
        ],
        "count": len(results),
        "query": query,
    }


@mcp.resource("schema://documents")
def get_schema() -> str:
    """Get the document service database schema.

    Returns the SQLite schema for documents and chunks tables.
    """
    return """
-- Documents table: stores document metadata and content
CREATE TABLE documents (
    id TEXT PRIMARY KEY,           -- Unique document identifier
    title TEXT NOT NULL,           -- Document title
    content TEXT NOT NULL,         -- Full document content
    metadata TEXT DEFAULT '{}',    -- JSON metadata
    created_at TEXT NOT NULL,      -- ISO timestamp
    updated_at TEXT NOT NULL       -- ISO timestamp
);

-- Chunks table: stores document chunks for search
CREATE TABLE chunks (
    id TEXT PRIMARY KEY,           -- Chunk identifier
    document_id TEXT NOT NULL,     -- Parent document
    content TEXT NOT NULL,         -- Chunk text content
    chunk_index INTEGER NOT NULL,  -- Position in document
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- Vector embeddings for semantic search (sqlite-vec)
CREATE VIRTUAL TABLE chunk_embeddings USING vec0(
    chunk_id TEXT PRIMARY KEY,
    embedding float[384]           -- Embedding vector
);

-- Indexes
CREATE INDEX idx_chunks_document_id ON chunks(document_id);
"""


def run_server(transport: str = "stdio", port: int = 8001):
    """Run the MCP server.

    Args:
        transport: Transport type - "stdio" or "streamable-http".
        port: Port for HTTP transport.
    """
    if transport == "streamable-http":
        logger.info(f"Starting document-service on port {port}")
        mcp.run(transport="streamable-http", port=port)
    else:
        mcp.run(transport="stdio")
