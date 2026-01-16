#!/usr/bin/env python3
"""
MCP User Service - SQLite-backed user/profile service with vector search.

This MCP server provides CRUD operations for user profiles and semantic
search using sqlite-vec embeddings.

Tools:
- create_user: Create a new user profile
- get_user: Retrieve a user by ID
- update_user: Update user profile fields
- delete_user: Remove a user
- search_users_by_bio: Semantic search over user bios using embeddings
- list_users: List all users with pagination
"""

from __future__ import annotations

import json
import os
import sqlite3
from datetime import datetime
from pathlib import Path
from typing import Any

from mcp.server.fastmcp import FastMCP
from pydantic import BaseModel, Field

# Initialize MCP server
mcp = FastMCP(
    name="user-service",
    version="0.1.0",
)

# Database configuration
DB_PATH = os.environ.get("USER_SERVICE_DB", "users.db")
EMBEDDING_DIM = 384  # all-MiniLM-L6-v2 dimension

# Feature flags
_sqlite_vec_available = None
_embeddings_available = None

# Embedding model (lazy loaded)
_embedding_model = None


def is_sqlite_vec_available() -> bool:
    """Check if sqlite-vec extension is available."""
    global _sqlite_vec_available
    if _sqlite_vec_available is None:
        try:
            conn = sqlite3.connect(":memory:")
            conn.enable_load_extension(True)
            try:
                import sqlite_vec
                sqlite_vec.load(conn)
                _sqlite_vec_available = True
            except ImportError:
                conn.load_extension("vec0")
                _sqlite_vec_available = True
        except Exception:
            _sqlite_vec_available = False
        finally:
            conn.close()
    return _sqlite_vec_available


def is_embeddings_available() -> bool:
    """Check if sentence-transformers is available."""
    global _embeddings_available
    if _embeddings_available is None:
        try:
            from sentence_transformers import SentenceTransformer
            _embeddings_available = True
        except ImportError:
            _embeddings_available = False
    return _embeddings_available


def get_embedding_model():
    """Lazy load the sentence transformer model."""
    global _embedding_model
    if _embedding_model is None:
        if not is_embeddings_available():
            raise RuntimeError(
                "sentence-transformers not installed. "
                "Install with: pip install sentence-transformers"
            )
        from sentence_transformers import SentenceTransformer
        _embedding_model = SentenceTransformer("all-MiniLM-L6-v2")
    return _embedding_model


def get_embedding(text: str) -> list[float]:
    """Generate embedding for text using sentence transformers."""
    model = get_embedding_model()
    embedding = model.encode(text, convert_to_numpy=True)
    return embedding.tolist()


def get_db_connection() -> sqlite3.Connection:
    """Get a database connection with sqlite-vec loaded if available."""
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row

    # Load sqlite-vec extension if available
    if is_sqlite_vec_available():
        conn.enable_load_extension(True)
        try:
            import sqlite_vec
            sqlite_vec.load(conn)
        except ImportError:
            conn.load_extension("vec0")
        conn.enable_load_extension(False)

    return conn


def run_migrations(conn: sqlite3.Connection) -> None:
    """Run database migrations to ensure schema is up to date."""
    cursor = conn.cursor()

    # Create migrations tracking table
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    """)

    # Get current version
    cursor.execute("SELECT MAX(version) FROM schema_migrations")
    row = cursor.fetchone()
    current_version = row[0] if row[0] is not None else 0

    # Build migrations list, conditionally including vector table
    migrations = [
        # Migration 1: Create users table
        (
            """
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                bio TEXT DEFAULT '',
                avatar_url TEXT DEFAULT '',
                location TEXT DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            """,
            True,  # Always run
        ),
        # Migration 2: Create user_embeddings virtual table for bio search
        (
            f"""
            CREATE VIRTUAL TABLE IF NOT EXISTS user_embeddings USING vec0(
                user_id INTEGER PRIMARY KEY,
                bio_embedding float[{EMBEDDING_DIM}]
            )
            """,
            is_sqlite_vec_available(),  # Only if sqlite-vec available
        ),
        # Migration 3: Create index on email
        (
            """
            CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)
            """,
            True,  # Always run
        ),
    ]

    for i, (migration, should_run) in enumerate(migrations, start=1):
        if i > current_version:
            if should_run:
                try:
                    cursor.execute(migration)
                    print(f"Applied migration {i}")
                except sqlite3.OperationalError as e:
                    # Ignore errors for already-existing objects
                    if "already exists" not in str(e):
                        raise
            else:
                print(f"Skipped migration {i} (dependency not available)")

            cursor.execute(
                "INSERT INTO schema_migrations (version) VALUES (?)",
                (i,)
            )

    conn.commit()


def init_db() -> None:
    """Initialize the database and run migrations."""
    conn = get_db_connection()
    run_migrations(conn)
    conn.close()


# Data models
class User(BaseModel):
    """User profile model."""
    id: int
    email: str
    name: str
    bio: str = ""
    avatar_url: str = ""
    location: str = ""
    created_at: str
    updated_at: str


class UserCreate(BaseModel):
    """Model for creating a new user."""
    email: str = Field(..., description="User's email address (must be unique)")
    name: str = Field(..., description="User's display name")
    bio: str = Field(default="", description="User's biography/description")
    avatar_url: str = Field(default="", description="URL to user's avatar image")
    location: str = Field(default="", description="User's location")


class UserUpdate(BaseModel):
    """Model for updating a user."""
    name: str | None = Field(default=None, description="New display name")
    bio: str | None = Field(default=None, description="New biography")
    avatar_url: str | None = Field(default=None, description="New avatar URL")
    location: str | None = Field(default=None, description="New location")


class SearchResult(BaseModel):
    """Search result with user and relevance score."""
    user: User
    distance: float = Field(..., description="Vector distance (lower is more similar)")


# MCP Tools

@mcp.tool()
def create_user(
    email: str,
    name: str,
    bio: str = "",
    avatar_url: str = "",
    location: str = "",
) -> User:
    """
    Create a new user profile.

    Args:
        email: User's email address (must be unique)
        name: User's display name
        bio: User's biography/description
        avatar_url: URL to user's avatar image
        location: User's location

    Returns:
        The created user profile
    """
    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        now = datetime.utcnow().isoformat()
        cursor.execute(
            """
            INSERT INTO users (email, name, bio, avatar_url, location, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            """,
            (email, name, bio, avatar_url, location, now, now)
        )
        user_id = cursor.lastrowid

        # Generate and store bio embedding if bio is provided and vector search available
        if bio and is_sqlite_vec_available() and is_embeddings_available():
            try:
                embedding = get_embedding(bio)
                embedding_json = json.dumps(embedding)
                cursor.execute(
                    """
                    INSERT INTO user_embeddings (user_id, bio_embedding)
                    VALUES (?, ?)
                    """,
                    (user_id, embedding_json)
                )
            except Exception as e:
                # Log but don't fail user creation if embedding fails
                print(f"Warning: Failed to create embedding for user {user_id}: {e}")

        conn.commit()

        # Fetch and return the created user
        cursor.execute("SELECT * FROM users WHERE id = ?", (user_id,))
        row = cursor.fetchone()
        return User(**dict(row))

    except sqlite3.IntegrityError as e:
        conn.rollback()
        raise ValueError(f"User with email '{email}' already exists") from e
    finally:
        conn.close()


@mcp.tool()
def get_user(user_id: int) -> User:
    """
    Retrieve a user by their ID.

    Args:
        user_id: The unique identifier of the user

    Returns:
        The user profile
    """
    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        cursor.execute("SELECT * FROM users WHERE id = ?", (user_id,))
        row = cursor.fetchone()

        if row is None:
            raise ValueError(f"User with ID {user_id} not found")

        return User(**dict(row))
    finally:
        conn.close()


@mcp.tool()
def get_user_by_email(email: str) -> User:
    """
    Retrieve a user by their email address.

    Args:
        email: The user's email address

    Returns:
        The user profile
    """
    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        cursor.execute("SELECT * FROM users WHERE email = ?", (email,))
        row = cursor.fetchone()

        if row is None:
            raise ValueError(f"User with email '{email}' not found")

        return User(**dict(row))
    finally:
        conn.close()


@mcp.tool()
def update_user(
    user_id: int,
    name: str | None = None,
    bio: str | None = None,
    avatar_url: str | None = None,
    location: str | None = None,
) -> User:
    """
    Update a user's profile fields.

    Args:
        user_id: The unique identifier of the user
        name: New display name (optional)
        bio: New biography (optional)
        avatar_url: New avatar URL (optional)
        location: New location (optional)

    Returns:
        The updated user profile
    """
    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        # Build update query dynamically
        updates = []
        params = []

        if name is not None:
            updates.append("name = ?")
            params.append(name)
        if bio is not None:
            updates.append("bio = ?")
            params.append(bio)
        if avatar_url is not None:
            updates.append("avatar_url = ?")
            params.append(avatar_url)
        if location is not None:
            updates.append("location = ?")
            params.append(location)

        if not updates:
            raise ValueError("No fields to update")

        updates.append("updated_at = ?")
        params.append(datetime.utcnow().isoformat())
        params.append(user_id)

        cursor.execute(
            f"UPDATE users SET {', '.join(updates)} WHERE id = ?",
            params
        )

        if cursor.rowcount == 0:
            raise ValueError(f"User with ID {user_id} not found")

        # Update bio embedding if bio changed and vector search available
        if bio is not None and is_sqlite_vec_available() and is_embeddings_available():
            try:
                embedding = get_embedding(bio)
                embedding_json = json.dumps(embedding)

                # Delete old embedding and insert new one
                cursor.execute(
                    "DELETE FROM user_embeddings WHERE user_id = ?",
                    (user_id,)
                )
                cursor.execute(
                    """
                    INSERT INTO user_embeddings (user_id, bio_embedding)
                    VALUES (?, ?)
                    """,
                    (user_id, embedding_json)
                )
            except Exception as e:
                # Log but don't fail user update if embedding fails
                print(f"Warning: Failed to update embedding for user {user_id}: {e}")

        conn.commit()

        # Fetch and return the updated user
        cursor.execute("SELECT * FROM users WHERE id = ?", (user_id,))
        row = cursor.fetchone()
        return User(**dict(row))

    finally:
        conn.close()


@mcp.tool()
def delete_user(user_id: int) -> dict[str, Any]:
    """
    Delete a user from the system.

    Args:
        user_id: The unique identifier of the user to delete

    Returns:
        Confirmation of deletion
    """
    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        # Delete embedding first if vector support available
        if is_sqlite_vec_available():
            try:
                cursor.execute(
                    "DELETE FROM user_embeddings WHERE user_id = ?",
                    (user_id,)
                )
            except Exception:
                pass  # Ignore if table doesn't exist

        cursor.execute("DELETE FROM users WHERE id = ?", (user_id,))

        if cursor.rowcount == 0:
            raise ValueError(f"User with ID {user_id} not found")

        conn.commit()
        return {"success": True, "deleted_user_id": user_id}

    finally:
        conn.close()


@mcp.tool()
def search_users_by_bio(
    query: str,
    limit: int = 10,
) -> list[SearchResult]:
    """
    Search for users by semantic similarity to their bio.

    Uses vector embeddings to find users whose bios are semantically
    similar to the query text.

    Args:
        query: Search query text to match against user bios
        limit: Maximum number of results to return (default: 10)

    Returns:
        List of users sorted by relevance (most similar first)
    """
    # Check dependencies
    if not is_sqlite_vec_available():
        raise RuntimeError(
            "Vector search not available: sqlite-vec extension not installed. "
            "Install with: pip install sqlite-vec"
        )
    if not is_embeddings_available():
        raise RuntimeError(
            "Vector search not available: sentence-transformers not installed. "
            "Install with: pip install sentence-transformers"
        )

    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        # Generate query embedding
        query_embedding = get_embedding(query)
        query_json = json.dumps(query_embedding)

        # Perform vector search using sqlite-vec
        cursor.execute(
            """
            SELECT
                u.*,
                e.distance
            FROM user_embeddings e
            JOIN users u ON e.user_id = u.id
            WHERE e.bio_embedding MATCH ?
            ORDER BY e.distance
            LIMIT ?
            """,
            (query_json, limit)
        )

        results = []
        for row in cursor.fetchall():
            row_dict = dict(row)
            distance = row_dict.pop("distance")
            user = User(**row_dict)
            results.append(SearchResult(user=user, distance=distance))

        return results

    finally:
        conn.close()


@mcp.tool()
def list_users(
    offset: int = 0,
    limit: int = 20,
) -> list[User]:
    """
    List all users with pagination.

    Args:
        offset: Number of users to skip (default: 0)
        limit: Maximum number of users to return (default: 20)

    Returns:
        List of user profiles
    """
    conn = get_db_connection()
    cursor = conn.cursor()

    try:
        cursor.execute(
            "SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?",
            (limit, offset)
        )

        return [User(**dict(row)) for row in cursor.fetchall()]

    finally:
        conn.close()


def seed_database() -> None:
    """Seed the database with sample users."""
    sample_users = [
        {
            "email": "alice@example.com",
            "name": "Alice Johnson",
            "bio": "Software engineer passionate about distributed systems and cloud architecture. Love building scalable microservices.",
            "location": "San Francisco, CA",
        },
        {
            "email": "bob@example.com",
            "name": "Bob Smith",
            "bio": "Data scientist working on machine learning and natural language processing. Interested in LLMs and AI agents.",
            "location": "New York, NY",
        },
        {
            "email": "carol@example.com",
            "name": "Carol Williams",
            "bio": "Frontend developer specializing in React and TypeScript. UX enthusiast who cares about accessibility.",
            "location": "Seattle, WA",
        },
        {
            "email": "david@example.com",
            "name": "David Brown",
            "bio": "DevOps engineer focused on Kubernetes, CI/CD pipelines, and infrastructure automation. GitOps advocate.",
            "location": "Austin, TX",
        },
        {
            "email": "eve@example.com",
            "name": "Eve Davis",
            "bio": "Product manager bridging technical and business teams. Experienced in agile methodologies and user research.",
            "location": "Boston, MA",
        },
        {
            "email": "frank@example.com",
            "name": "Frank Garcia",
            "bio": "Security researcher specializing in application security, penetration testing, and secure code review.",
            "location": "Denver, CO",
        },
        {
            "email": "grace@example.com",
            "name": "Grace Lee",
            "bio": "Backend developer with expertise in Python and Go. Building APIs and database systems for high-traffic applications.",
            "location": "Portland, OR",
        },
        {
            "email": "henry@example.com",
            "name": "Henry Miller",
            "bio": "Mobile developer creating iOS and Android apps. Passionate about cross-platform development with Flutter.",
            "location": "Miami, FL",
        },
    ]

    conn = get_db_connection()
    cursor = conn.cursor()

    # Check if we already have users
    cursor.execute("SELECT COUNT(*) FROM users")
    count = cursor.fetchone()[0]

    if count > 0:
        print(f"Database already has {count} users, skipping seed")
        conn.close()
        return

    print("Seeding database with sample users...")
    conn.close()

    for user_data in sample_users:
        try:
            create_user(**user_data)
            print(f"  Created user: {user_data['name']}")
        except ValueError as e:
            print(f"  Skipped {user_data['name']}: {e}")

    print("Seeding complete!")


def main():
    """Main entry point for the MCP server."""
    import argparse

    parser = argparse.ArgumentParser(description="User Service MCP Server")
    parser.add_argument(
        "--db",
        default="users.db",
        help="Path to SQLite database file",
    )
    parser.add_argument(
        "--seed",
        action="store_true",
        help="Seed the database with sample users",
    )
    parser.add_argument(
        "--transport",
        default="stdio",
        choices=["stdio", "streamable-http"],
        help="MCP transport to use",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=8000,
        help="Port for HTTP transport",
    )

    args = parser.parse_args()

    # Set database path
    global DB_PATH
    DB_PATH = args.db

    # Initialize database
    init_db()

    # Seed if requested
    if args.seed:
        seed_database()

    # Run the MCP server
    if args.transport == "stdio":
        mcp.run(transport="stdio")
    else:
        mcp.run(transport="streamable-http", port=args.port)


if __name__ == "__main__":
    main()
