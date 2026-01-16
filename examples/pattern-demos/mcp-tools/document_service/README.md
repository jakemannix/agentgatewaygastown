# Document Service MCP Server

A Python MCP server providing SQLite-backed document storage with semantic search capabilities.

## Features

- **CRUD Operations**: Create, read, update, and delete documents
- **Automatic Chunking**: Large documents are automatically split into manageable chunks
- **Semantic Search**: Vector-based similarity search using sqlite-vec and sentence-transformers
- **Metadata Support**: Attach arbitrary JSON metadata to documents

## Installation

```bash
cd examples/pattern-demos/mcp-tools/document_service
pip install -e .
```

This installs the following dependencies:
- `mcp>=1.0.0` - Model Context Protocol Python SDK
- `sqlite-vec>=0.1.0` - SQLite vector extension
- `sentence-transformers>=2.2.0` - Embedding generation
- `numpy>=1.24.0` - Numerical operations

## Usage

### Standalone

Run the server directly:

```bash
python -m document_service --db-path ./documents.db
```

Options:
- `--db-path`: Path to SQLite database file (default: `:memory:` for in-memory)
- `--embedding-model`: Sentence-transformers model name (default: `all-MiniLM-L6-v2`)

### With Agentgateway

Run through the agentgateway proxy:

```bash
cargo run -- -f examples/pattern-demos/mcp-tools/document_service/config.yaml
```

Then connect with the MCP Inspector:

```bash
npx @modelcontextprotocol/inspector
```

Navigate to `http://localhost:3000/mcp` to interact with the document service.

## Available Tools

### create_document

Create a new document with automatic chunking and embedding.

**Parameters:**
- `title` (required): Document title
- `content` (required): Document content
- `metadata` (optional): JSON object with arbitrary metadata
- `chunk_size` (optional): Target chunk size in characters (default: 500)
- `chunk_overlap` (optional): Overlap between chunks (default: 50)

**Example:**
```json
{
  "title": "Introduction to RAG",
  "content": "Retrieval-Augmented Generation (RAG) is a technique...",
  "metadata": {"author": "Jane Doe", "category": "AI"}
}
```

### get_document

Retrieve a document by its ID.

**Parameters:**
- `id` (required): Document ID
- `include_chunks` (optional): Whether to include chunks (default: false)

### list_documents

List all documents with pagination.

**Parameters:**
- `limit` (optional): Maximum documents to return (default: 100)
- `offset` (optional): Number to skip (default: 0)

### update_document

Update an existing document.

**Parameters:**
- `id` (required): Document ID
- `title` (optional): New title
- `content` (optional): New content (triggers re-chunking/embedding)
- `metadata` (optional): New metadata

### delete_document

Delete a document and its chunks.

**Parameters:**
- `id` (required): Document ID

### search_documents

Semantic search across all documents.

**Parameters:**
- `query` (required): Search query text
- `limit` (optional): Maximum results (default: 10)

**Example:**
```json
{
  "query": "How does RAG work with vector databases?",
  "limit": 5
}
```

Returns chunks ranked by semantic similarity with scores.

## Architecture

```
document_service/
├── __init__.py        # Package exports
├── __main__.py        # CLI entry point
├── server.py          # MCP server and tool handlers
├── database.py        # SQLite + sqlite-vec storage
├── embeddings.py      # Sentence-transformers wrapper
└── chunking.py        # Text chunking utilities
```

### Database Schema

**documents**: Stores document metadata
- `id`: Primary key
- `title`: Document title
- `content`: Full content
- `metadata`: JSON metadata
- `created_at`, `updated_at`: Timestamps

**chunks**: Stores document chunks
- `id`: Primary key
- `document_id`: Foreign key to documents
- `content`: Chunk text
- `chunk_index`: Position in document

**chunk_embeddings**: sqlite-vec virtual table for vector search
- `chunk_id`: Reference to chunks
- `embedding`: 384-dimensional float vector

## Embedding Model

By default, uses `all-MiniLM-L6-v2` which produces 384-dimensional vectors.
This model offers a good balance of speed and quality for document search.

To use a different model:

```bash
python -m document_service --embedding-model paraphrase-multilingual-MiniLM-L12-v2
```

Note: Changing the embedding model requires re-creating the database since vector dimensions may differ.
