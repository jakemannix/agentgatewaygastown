# User Service MCP Server

A SQLite-backed MCP server for user/profile management with vector search capabilities using `sqlite-vec`.

## Features

- **CRUD Operations**: Create, read, update, and delete user profiles
- **Semantic Search**: Search users by bio using vector embeddings (sqlite-vec + sentence-transformers)
- **Schema Migrations**: Automatic database migrations on startup
- **Seed Data**: Sample users for testing

## Installation

```bash
cd examples/pattern-demos/mcp-tools/user_service
pip install -e .
```

Or install dependencies directly:

```bash
pip install mcp sqlite-vec pydantic sentence-transformers
```

## Usage

### Start the Server (stdio mode)

```bash
# With default database (users.db)
python server.py

# With custom database path
python server.py --db /path/to/database.db

# Seed with sample data on startup
python server.py --seed
```

### Start the Server (HTTP mode)

```bash
python server.py --transport streamable-http --port 8000
```

### With AgentGateway

Add to your gateway config:

```yaml
targets:
  - name: user-service
    stdio:
      cmd: python
      args: ["examples/pattern-demos/mcp-tools/user_service/server.py", "--seed"]
```

## MCP Tools

### create_user

Create a new user profile.

**Parameters:**
- `email` (required): User's email address (must be unique)
- `name` (required): User's display name
- `bio`: User's biography/description
- `avatar_url`: URL to user's avatar image
- `location`: User's location

**Returns:** Created user object

### get_user

Retrieve a user by their ID.

**Parameters:**
- `user_id` (required): The unique identifier of the user

**Returns:** User object

### get_user_by_email

Retrieve a user by their email address.

**Parameters:**
- `email` (required): The user's email address

**Returns:** User object

### update_user

Update a user's profile fields.

**Parameters:**
- `user_id` (required): The unique identifier of the user
- `name`: New display name
- `bio`: New biography
- `avatar_url`: New avatar URL
- `location`: New location

**Returns:** Updated user object

### delete_user

Delete a user from the system.

**Parameters:**
- `user_id` (required): The unique identifier of the user to delete

**Returns:** Confirmation object

### search_users_by_bio

Search for users by semantic similarity to their bio using vector embeddings.

**Parameters:**
- `query` (required): Search query text to match against user bios
- `limit`: Maximum number of results (default: 10)

**Returns:** List of users sorted by relevance with distance scores

### list_users

List all users with pagination.

**Parameters:**
- `offset`: Number of users to skip (default: 0)
- `limit`: Maximum number of users to return (default: 20)

**Returns:** List of user objects

## Database Schema

### users table

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER | Primary key |
| email | TEXT | Unique email address |
| name | TEXT | Display name |
| bio | TEXT | Biography |
| avatar_url | TEXT | Avatar URL |
| location | TEXT | Location |
| created_at | TEXT | ISO timestamp |
| updated_at | TEXT | ISO timestamp |

### user_embeddings virtual table (sqlite-vec)

| Column | Type | Description |
|--------|------|-------------|
| user_id | INTEGER | Foreign key to users |
| bio_embedding | float[384] | Bio embedding vector |

## Example Usage

```python
# Create a user
user = create_user(
    email="test@example.com",
    name="Test User",
    bio="Software developer interested in AI and machine learning"
)

# Search by bio
results = search_users_by_bio("machine learning engineer", limit=5)

# Update user
updated = update_user(user_id=1, bio="Updated bio text")

# Delete user
delete_user(user_id=1)
```

## Architecture

```
┌─────────────────────────────────────────────────┐
│                 MCP Client (Agent)               │
└─────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│              User Service MCP Server             │
│  ┌─────────────────────────────────────────┐    │
│  │        FastMCP Tool Handlers             │    │
│  │  - create_user    - delete_user          │    │
│  │  - get_user       - search_users_by_bio  │    │
│  │  - update_user    - list_users           │    │
│  └─────────────────────────────────────────┘    │
│                       │                          │
│  ┌─────────────────────────────────────────┐    │
│  │         SQLite + sqlite-vec              │    │
│  │  - users table (CRUD)                    │    │
│  │  - user_embeddings (vector search)       │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

## Embedding Model

This service uses the `all-MiniLM-L6-v2` model from sentence-transformers for generating bio embeddings. The model produces 384-dimensional vectors and is efficient for semantic similarity tasks.

On first run, the model will be downloaded (~90MB). Subsequent runs use the cached model.
