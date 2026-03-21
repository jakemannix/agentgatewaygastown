"""Research Assistant Demo - FastAPI server using ADK's standard pattern.

This is the standard ADK deployment pattern using get_fast_api_app.
It automatically discovers agents in the agents/ directory and serves them.

Usage:
    # Start with uvicorn directly
    uvicorn main:app --host 0.0.0.0 --port 9001

    # Or run this file
    python main.py

Environment variables:
    GATEWAY_URL: MCP gateway URL (default: http://localhost:3000)
    LLM_MODEL: Model to use (default: gemini-2.0-flash)
    GOOGLE_API_KEY: Required for Gemini models
    PORT: Server port (default: 9001)
"""

import os
from dotenv import load_dotenv

# Load .env BEFORE any other imports — override=True so project .env takes
# precedence over shell-level exports (e.g. stale keys in ~/.zshrc)
load_dotenv(override=True)

import uvicorn
from fastapi import FastAPI
from google.adk.cli.fast_api import get_fast_api_app

# Get the directory containing agent subdirectories
# Structure: agents/research_agent/agent.py exports root_agent
AGENTS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "agents")

# Session persistence - use SQLite with async driver
# For production, consider PostgreSQL or other persistent storage
SESSION_DB_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "sessions.db")
SESSION_SERVICE_URI = f"sqlite+aiosqlite:///{SESSION_DB_PATH}"

# CORS configuration for web UI access
ALLOWED_ORIGINS = [
    "http://localhost",
    "http://localhost:8080",
    "http://localhost:3000",
    "*",  # Allow all for development
]

# Create the FastAPI app using ADK's standard pattern
# This handles all the async context management properly
app: FastAPI = get_fast_api_app(
    agents_dir=AGENTS_DIR,
    session_service_uri=SESSION_SERVICE_URI,
    allow_origins=ALLOWED_ORIGINS,
    web=True,  # Enable ADK web UI at /dev-ui
)


if __name__ == "__main__":
    # Use PORT env var (Cloud Run convention) or default to 9001
    port = int(os.environ.get("PORT", 9001))

    print("=" * 60)
    print("RESEARCH ASSISTANT - ADK FastAPI Server")
    print("=" * 60)
    print(f"Gateway URL: {os.environ.get('GATEWAY_URL', 'http://localhost:3000')}")
    print(f"Model: {os.environ.get('LLM_MODEL', 'gemini-2.0-flash')}")
    print(f"Port: {port}")
    print(f"Agents dir: {AGENTS_DIR}")
    print(f"Session DB: {SESSION_DB_PATH}")
    print()
    print("Endpoints:")
    print(f"  - ADK Web UI: http://localhost:{port}/dev-ui")
    print(f"  - API docs:   http://localhost:{port}/docs")
    print("=" * 60)

    uvicorn.run(app, host="0.0.0.0", port=port, log_level="info")
