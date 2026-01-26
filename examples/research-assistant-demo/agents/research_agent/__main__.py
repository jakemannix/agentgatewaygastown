"""Entry point for research agent - runs as A2A server."""

import argparse
import asyncio
import logging
import os
import sys
from contextlib import asynccontextmanager

import uvicorn
from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse

# Add parent to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from agents.research_agent.agent import create_research_agent, SYSTEM_PROMPT
from agents.shared.llm_config import get_llm_config

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Global agent instance
_agent = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Initialize agent on startup."""
    global _agent
    gateway_url = os.getenv("GATEWAY_URL", "http://localhost:3000/mcp")
    logger.info(f"Initializing research agent with gateway: {gateway_url}")
    _agent = create_research_agent(gateway_url)
    yield
    logger.info("Shutting down research agent")


app = FastAPI(
    title="Research Agent",
    description="A2A-compatible research assistant agent",
    lifespan=lifespan,
)


@app.get("/.well-known/agent.json")
async def agent_card():
    """Return A2A agent card."""
    llm_config = get_llm_config()
    return {
        "name": "research-agent",
        "version": "1.0.0",
        "description": "Research assistant for discovering and organizing technical knowledge",
        "url": f"http://localhost:{os.getenv('PORT', '9001')}",
        "capabilities": {
            "streaming": False,
            "pushNotifications": False,
        },
        "skills": [
            {
                "id": "research_topic",
                "name": "Research Topic",
                "description": "Research a technical topic across multiple sources",
                "tags": ["research", "search", "knowledge"],
                "examples": [
                    "Research transformer alternatives for 2025-2026",
                    "Find papers about state space models",
                    "Search for implementations of Mamba architecture",
                ],
            },
            {
                "id": "organize_knowledge",
                "name": "Organize Knowledge",
                "description": "Store and organize research findings",
                "tags": ["knowledge", "organize", "tag"],
                "examples": [
                    "Save this paper to my knowledge base",
                    "Create a category for attention mechanisms",
                    "Link these two concepts together",
                ],
            },
            {
                "id": "explore_knowledge",
                "name": "Explore Knowledge",
                "description": "Query stored knowledge and relationships",
                "tags": ["knowledge", "query", "explore"],
                "examples": [
                    "What do I know about transformers?",
                    "Show me the category tree",
                    "How is Mamba related to state space models?",
                ],
            },
        ],
        "defaultInputModes": ["text"],
        "defaultOutputModes": ["text"],
        "provider": {
            "organization": "AgentGateway Demo",
        },
        "metadata": {
            "llm_provider": llm_config.provider,
            "llm_model": llm_config.model,
        },
    }


@app.post("/chat")
async def chat(request: Request):
    """Simple chat endpoint for testing."""
    global _agent

    if _agent is None:
        return JSONResponse(
            status_code=503,
            content={"error": "Agent not initialized"},
        )

    try:
        body = await request.json()
        message = body.get("message", "")
        session_id = body.get("session_id", "default")

        if not message:
            return JSONResponse(
                status_code=400,
                content={"error": "Message required"},
            )

        logger.info(f"Chat request [session={session_id}]: {message[:100]}...")

        # Run the agent
        # Note: This is a simplified implementation
        # In production, you'd want proper session management
        response = await _agent.run(message)

        return {
            "session_id": session_id,
            "response": response.text if hasattr(response, 'text') else str(response),
        }

    except Exception as e:
        logger.exception("Error in chat endpoint")
        return JSONResponse(
            status_code=500,
            content={"error": str(e)},
        )


@app.get("/health")
async def health():
    """Health check endpoint."""
    return {"status": "healthy", "agent": "research-agent"}


def main():
    parser = argparse.ArgumentParser(description="Run Research Agent A2A Server")
    parser.add_argument("--port", type=int, default=int(os.getenv("PORT", "9001")))
    parser.add_argument("--host", type=str, default="0.0.0.0")
    parser.add_argument("--gateway-url", type=str, default="http://localhost:3000/mcp")
    args = parser.parse_args()

    # Set gateway URL in environment for lifespan to pick up
    os.environ["GATEWAY_URL"] = args.gateway_url

    logger.info(f"Starting Research Agent on {args.host}:{args.port}")
    logger.info(f"Gateway URL: {args.gateway_url}")

    uvicorn.run(
        app,
        host=args.host,
        port=args.port,
        log_level="info",
    )


if __name__ == "__main__":
    main()
