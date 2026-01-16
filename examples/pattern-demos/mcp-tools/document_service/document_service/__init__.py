"""SQLite-backed MCP document service with semantic search."""

from .server import create_server

__all__ = ["create_server"]
