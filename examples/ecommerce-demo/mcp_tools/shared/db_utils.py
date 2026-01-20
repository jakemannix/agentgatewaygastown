"""Shared database utilities for ecommerce MCP services."""

import sqlite3
from pathlib import Path
from typing import Any, Optional


def get_db_path(db_name: str, data_dir: Optional[Path] = None) -> Path:
    """Get the path to a database file."""
    if data_dir is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
    data_dir.mkdir(parents=True, exist_ok=True)
    return data_dir / db_name


def get_connection(db_path: Path) -> sqlite3.Connection:
    """Get a SQLite connection with common settings."""
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    return conn


def dict_from_row(row: sqlite3.Row) -> dict[str, Any]:
    """Convert a sqlite3.Row to a dict."""
    return dict(row)


def rows_to_dicts(rows: list[sqlite3.Row]) -> list[dict[str, Any]]:
    """Convert a list of sqlite3.Row to a list of dicts."""
    return [dict_from_row(row) for row in rows]
