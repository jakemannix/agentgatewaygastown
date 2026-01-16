"""Entry point for running the MCP Task Service."""

import argparse
import os
import sys


def main():
    parser = argparse.ArgumentParser(
        description="MCP Task Service - SQLite-backed task/todo management"
    )
    parser.add_argument(
        "--transport",
        choices=["stdio", "streamable-http"],
        default="stdio",
        help="Transport type (default: stdio)",
    )
    parser.add_argument(
        "--db",
        default=":memory:",
        help="SQLite database path (default: in-memory)",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=8000,
        help="Port for HTTP transport (default: 8000)",
    )

    args = parser.parse_args()

    # Set database path in environment for server to use
    os.environ["TASK_SERVICE_DB"] = args.db

    # Import here to ensure environment is set first
    from .server import mcp

    if args.transport == "streamable-http":
        mcp.run(transport="streamable-http", port=args.port)
    else:
        mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
