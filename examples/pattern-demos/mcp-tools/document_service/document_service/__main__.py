"""Entry point for the document service MCP server."""

from __future__ import annotations

import argparse
import asyncio
import sys


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

    args = parser.parse_args()

    from .server import run_server

    asyncio.run(run_server(args.db_path, args.embedding_model))


if __name__ == "__main__":
    main()
