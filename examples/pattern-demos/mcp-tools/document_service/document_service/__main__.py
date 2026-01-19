"""Entry point for the document service MCP server."""

from __future__ import annotations

import argparse
import logging
import os
import sys

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)


def main():
    """Run the document service MCP server."""
    parser = argparse.ArgumentParser(
        description="SQLite-backed MCP document service with semantic search"
    )
    parser.add_argument(
        "--db-path",
        default=":memory:",
        help="Path to SQLite database (default: in-memory)",
    )
    parser.add_argument(
        "--embedding-model",
        default="all-MiniLM-L6-v2",
        help="Sentence-transformers model name (default: all-MiniLM-L6-v2)",
    )
    parser.add_argument(
        "--transport",
        choices=["stdio", "streamable-http"],
        default="stdio",
        help="Transport type (default: stdio)",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=8001,
        help="Port for HTTP transport (default: 8001)",
    )

    args = parser.parse_args()

    # Set environment variables for the server module
    os.environ["DOCUMENT_SERVICE_DB"] = args.db_path
    os.environ["DOCUMENT_SERVICE_MODEL"] = args.embedding_model

    # Import after setting env vars
    from .server import run_server

    if args.transport == "streamable-http":
        logger.info(f"Starting document-service on http://0.0.0.0:{args.port}")
        logger.info(f"Database: {args.db_path}")
        logger.info(f"Embedding model: {args.embedding_model}")

    run_server(transport=args.transport, port=args.port)


if __name__ == "__main__":
    main()
