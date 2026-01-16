"""MCP server for document management with semantic search."""

from __future__ import annotations

import uuid
from typing import Any

from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import Tool, TextContent

from .database import DocumentDatabase, Chunk
from .embeddings import get_embedding_model
from .chunking import chunk_text, generate_chunk_id


def create_server(
    db_path: str = ":memory:",
    embedding_model: str = "all-MiniLM-L6-v2",
) -> Server:
    """Create and configure the MCP server.

    Args:
        db_path: Path to SQLite database file or ":memory:" for in-memory.
        embedding_model: Name of the sentence-transformers model.

    Returns:
        Configured MCP server.
    """
    # Initialize components
    embedder = get_embedding_model(embedding_model)
    db = DocumentDatabase(db_path, embedding_dim=embedder.embedding_dim)

    # Create server
    server = Server("document-service")

    @server.list_tools()
    async def list_tools() -> list[Tool]:
        """List available tools."""
        return [
            Tool(
                name="create_document",
                description="Create a new document with automatic chunking and embedding for semantic search.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "The document title",
                        },
                        "content": {
                            "type": "string",
                            "description": "The document content",
                        },
                        "metadata": {
                            "type": "object",
                            "description": "Optional metadata as key-value pairs",
                            "additionalProperties": True,
                        },
                        "chunk_size": {
                            "type": "integer",
                            "description": "Target chunk size in characters (default: 500)",
                            "default": 500,
                        },
                        "chunk_overlap": {
                            "type": "integer",
                            "description": "Overlap between chunks in characters (default: 50)",
                            "default": 50,
                        },
                    },
                    "required": ["title", "content"],
                },
            ),
            Tool(
                name="get_document",
                description="Retrieve a document by its ID.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The document ID",
                        },
                        "include_chunks": {
                            "type": "boolean",
                            "description": "Whether to include chunks in the response",
                            "default": False,
                        },
                    },
                    "required": ["id"],
                },
            ),
            Tool(
                name="list_documents",
                description="List all documents with pagination.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of documents to return (default: 100)",
                            "default": 100,
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Number of documents to skip (default: 0)",
                            "default": 0,
                        },
                    },
                },
            ),
            Tool(
                name="update_document",
                description="Update an existing document. Only provided fields will be updated.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The document ID to update",
                        },
                        "title": {
                            "type": "string",
                            "description": "New title (optional)",
                        },
                        "content": {
                            "type": "string",
                            "description": "New content (optional, triggers re-chunking and re-embedding)",
                        },
                        "metadata": {
                            "type": "object",
                            "description": "New metadata (optional)",
                            "additionalProperties": True,
                        },
                        "chunk_size": {
                            "type": "integer",
                            "description": "Chunk size if content is updated (default: 500)",
                            "default": 500,
                        },
                        "chunk_overlap": {
                            "type": "integer",
                            "description": "Chunk overlap if content is updated (default: 50)",
                            "default": 50,
                        },
                    },
                    "required": ["id"],
                },
            ),
            Tool(
                name="delete_document",
                description="Delete a document and all its chunks.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The document ID to delete",
                        },
                    },
                    "required": ["id"],
                },
            ),
            Tool(
                name="search_documents",
                description="Search documents using semantic similarity. Returns relevant chunks ranked by relevance.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query",
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results (default: 10)",
                            "default": 10,
                        },
                    },
                    "required": ["query"],
                },
            ),
        ]

    @server.call_tool()
    async def call_tool(name: str, arguments: dict[str, Any]) -> list[TextContent]:
        """Handle tool calls."""
        if name == "create_document":
            return await _create_document(db, embedder, arguments)
        elif name == "get_document":
            return await _get_document(db, arguments)
        elif name == "list_documents":
            return await _list_documents(db, arguments)
        elif name == "update_document":
            return await _update_document(db, embedder, arguments)
        elif name == "delete_document":
            return await _delete_document(db, arguments)
        elif name == "search_documents":
            return await _search_documents(db, embedder, arguments)
        else:
            return [TextContent(type="text", text=f"Unknown tool: {name}")]

    return server


async def _create_document(
    db: DocumentDatabase,
    embedder,
    args: dict[str, Any],
) -> list[TextContent]:
    """Create a new document."""
    title = args["title"]
    content = args["content"]
    metadata = args.get("metadata", {})
    chunk_size = args.get("chunk_size", 500)
    chunk_overlap = args.get("chunk_overlap", 50)

    # Generate document ID
    doc_id = str(uuid.uuid4())

    # Create document
    doc = db.create_document(doc_id, title, content, metadata)

    # Chunk the content
    text_chunks = chunk_text(
        content,
        chunk_size=chunk_size,
        chunk_overlap=chunk_overlap,
    )

    # Generate embeddings for chunks
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

    return [
        TextContent(
            type="text",
            text=f"Created document '{title}' with ID: {doc_id}\n"
            f"Created {len(text_chunks)} chunks for semantic search.",
        )
    ]


async def _get_document(
    db: DocumentDatabase,
    args: dict[str, Any],
) -> list[TextContent]:
    """Get a document by ID."""
    doc_id = args["id"]
    include_chunks = args.get("include_chunks", False)

    doc = db.get_document(doc_id)
    if doc is None:
        return [TextContent(type="text", text=f"Document not found: {doc_id}")]

    result = {
        "id": doc.id,
        "title": doc.title,
        "content": doc.content,
        "metadata": doc.metadata,
        "created_at": doc.created_at.isoformat(),
        "updated_at": doc.updated_at.isoformat(),
    }

    if include_chunks:
        chunks = db.get_chunks(doc_id)
        result["chunks"] = [
            {"id": c.id, "content": c.content, "index": c.chunk_index}
            for c in chunks
        ]

    import json
    return [TextContent(type="text", text=json.dumps(result, indent=2))]


async def _list_documents(
    db: DocumentDatabase,
    args: dict[str, Any],
) -> list[TextContent]:
    """List documents."""
    limit = args.get("limit", 100)
    offset = args.get("offset", 0)

    docs = db.list_documents(limit=limit, offset=offset)

    result = [
        {
            "id": doc.id,
            "title": doc.title,
            "metadata": doc.metadata,
            "created_at": doc.created_at.isoformat(),
            "updated_at": doc.updated_at.isoformat(),
        }
        for doc in docs
    ]

    import json
    return [
        TextContent(
            type="text",
            text=f"Found {len(result)} documents:\n{json.dumps(result, indent=2)}",
        )
    ]


async def _update_document(
    db: DocumentDatabase,
    embedder,
    args: dict[str, Any],
) -> list[TextContent]:
    """Update a document."""
    doc_id = args["id"]
    title = args.get("title")
    content = args.get("content")
    metadata = args.get("metadata")
    chunk_size = args.get("chunk_size", 500)
    chunk_overlap = args.get("chunk_overlap", 50)

    doc = db.update_document(doc_id, title=title, content=content, metadata=metadata)
    if doc is None:
        return [TextContent(type="text", text=f"Document not found: {doc_id}")]

    # Re-chunk and re-embed if content was updated
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
                    id=generate_chunk_id(doc_id, i),
                    document_id=doc_id,
                    content=text,
                    chunk_index=i,
                    embedding=embedding,
                )
                for i, (text, embedding) in enumerate(zip(text_chunks, embeddings))
            ]
            db.store_chunks(doc_id, chunks)

        return [
            TextContent(
                type="text",
                text=f"Updated document '{doc.title}' (ID: {doc_id})\n"
                f"Re-created {len(text_chunks)} chunks for semantic search.",
            )
        ]

    return [
        TextContent(
            type="text",
            text=f"Updated document '{doc.title}' (ID: {doc_id})",
        )
    ]


async def _delete_document(
    db: DocumentDatabase,
    args: dict[str, Any],
) -> list[TextContent]:
    """Delete a document."""
    doc_id = args["id"]

    deleted = db.delete_document(doc_id)
    if not deleted:
        return [TextContent(type="text", text=f"Document not found: {doc_id}")]

    return [
        TextContent(type="text", text=f"Deleted document: {doc_id}")
    ]


async def _search_documents(
    db: DocumentDatabase,
    embedder,
    args: dict[str, Any],
) -> list[TextContent]:
    """Search documents semantically."""
    query = args["query"]
    limit = args.get("limit", 10)

    # Generate query embedding
    query_embedding = embedder.embed(query)

    # Search for similar chunks
    results = db.search_similar(query_embedding, limit=limit)

    if not results:
        return [TextContent(type="text", text="No results found.")]

    # Format results
    formatted = []
    for r in results:
        formatted.append({
            "document_id": r.document_id,
            "document_title": r.document_title,
            "chunk_id": r.chunk_id,
            "score": round(r.score, 4),
            "content": r.content,
        })

    import json
    return [
        TextContent(
            type="text",
            text=f"Found {len(formatted)} results:\n{json.dumps(formatted, indent=2)}",
        )
    ]


async def run_server(db_path: str = ":memory:", embedding_model: str = "all-MiniLM-L6-v2"):
    """Run the MCP server."""
    server = create_server(db_path, embedding_model)
    async with stdio_server() as (read_stream, write_stream):
        await server.run(read_stream, write_stream, server.create_initialization_options())
