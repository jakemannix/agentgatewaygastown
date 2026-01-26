"""Shared database utilities for SQLite + sqlite-vec."""

import json
import sqlite3
from pathlib import Path
from typing import Any

import sqlite_vec


def get_db_path(db_name: str, data_dir: Path | None = None) -> Path:
    """Get the path to a database file."""
    if data_dir is None:
        data_dir = Path(__file__).parent.parent.parent / "data"
    data_dir.mkdir(parents=True, exist_ok=True)
    return data_dir / db_name


def get_connection(db_path: Path) -> sqlite3.Connection:
    """Get a connection with sqlite-vec loaded."""
    conn = sqlite3.connect(str(db_path))
    conn.enable_load_extension(True)
    sqlite_vec.load(conn)
    conn.enable_load_extension(False)
    conn.row_factory = sqlite3.Row
    return conn


def serialize_json(obj: Any) -> str:
    """Serialize an object to JSON string."""
    return json.dumps(obj) if obj is not None else "null"


def deserialize_json(s: str | None) -> Any:
    """Deserialize a JSON string to an object."""
    if s is None:
        return None
    return json.loads(s)


def row_to_dict(row: sqlite3.Row) -> dict[str, Any]:
    """Convert a sqlite Row to a dictionary."""
    return dict(row)


def rows_to_list(rows: list[sqlite3.Row]) -> list[dict[str, Any]]:
    """Convert a list of sqlite Rows to a list of dictionaries."""
    return [row_to_dict(row) for row in rows]
