"""Research Agent A2A Server - Entry point.

This agent uses Google ADK with LiteLLM for multi-provider support.
Requires one of: ANTHROPIC_API_KEY, OPENAI_API_KEY, or GOOGLE_API_KEY
"""

import argparse
import logging
import os
import sys

# Add parent to path for imports
sys.path.insert(0, str(__file__).rsplit("/", 3)[0])

from agents.shared.a2a_server import A2AServer

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)

# Configuration
AGENT_NAME = "Research Assistant Agent"
AGENT_PORT = int(os.environ.get("RESEARCH_AGENT_PORT", 9001))
GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:3000")

# Agent skills for A2A discovery
SKILLS = [
    {
        "id": "research-topic",
        "name": "Research Topic",
        "description": "Research a technical topic across multiple sources (web, arXiv, GitHub, HuggingFace)",
        "tags": ["research", "search", "papers", "code"],
        "examples": [
            "Research transformer alternatives for 2025-2026",
            "Find papers about state space models",
            "Search for implementations of Mamba architecture",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
    {
        "id": "store-knowledge",
        "name": "Store Knowledge",
        "description": "Store research findings in the knowledge graph with entities and relationships",
        "tags": ["knowledge", "store", "organize"],
        "examples": [
            "Save this paper to my knowledge base",
            "Create an entity for this concept",
            "Link these two research topics together",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
    {
        "id": "explore-knowledge",
        "name": "Explore Knowledge",
        "description": "Query stored knowledge, explore relationships, and browse categories",
        "tags": ["knowledge", "query", "explore"],
        "examples": [
            "What do I know about transformers?",
            "Show me the category tree",
            "How is Mamba related to state space models?",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
    {
        "id": "deep-research",
        "name": "Deep Research",
        "description": "Perform comprehensive research with content fetching and extraction",
        "tags": ["research", "fetch", "analyze"],
        "examples": [
            "Do a deep dive on attention mechanisms",
            "Fetch and analyze the top papers on this topic",
            "Extract key information from these URLs",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
]


async def handle_research_message(message_text: str, context: dict) -> str:
    """Handle incoming research messages using the Google ADK agent.

    Args:
        message_text: The user's message
        context: Request context including session info

    Returns:
        Agent's response

    Raises:
        RuntimeError: If no LLM API key is set or agent fails
    """
    from .agent import run_research_query

    # Extract session info (handle both A2A context_id and REST session_id)
    session_id = context.get("session_id") or context.get("context_id") or "default-session"
    user_id = context.get("user_id", session_id[:8])

    logger.info(f"Processing message for user={user_id}, session={session_id}: {message_text[:100]}...")

    # Run the agent - let errors propagate
    response = await run_research_query(
        query=message_text,
        user_id=user_id,
        session_id=session_id,
    )

    logger.info(f"Agent response: {response[:100]}...")
    return response


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Research Assistant Agent (Google ADK)")
    parser.add_argument("--port", type=int, default=AGENT_PORT, help="Port to run on")
    parser.add_argument("--gateway", default=GATEWAY_URL, help="Gateway URL")
    parser.add_argument("--host", default="0.0.0.0", help="Host to bind to")
    args = parser.parse_args()

    # Check for LLM provider
    from .agent import detect_llm_provider, get_configured_model, API_KEY_ENV_VARS

    provider = detect_llm_provider()
    if not provider:
        logger.error(
            "No LLM API key found.\n"
            "The Research Agent requires one of:\n"
            f"  {', '.join(API_KEY_ENV_VARS.values())}\n"
            "Example: export ANTHROPIC_API_KEY='your-key-here'"
        )
        sys.exit(1)

    # Get the model that will be used
    model = get_configured_model()

    # Update environment
    os.environ["GATEWAY_URL"] = args.gateway

    # Create and configure A2A server
    server = A2AServer(
        name=AGENT_NAME,
        description="AI research assistant that helps discover, organize, and connect technical knowledge (powered by Google ADK)",
        port=args.port,
        skills=SKILLS,
    )

    # Set the message handler
    server.set_message_handler(handle_research_message)

    # Run the server
    logger.info(f"Research Agent starting on port {args.port}")
    logger.info(f"Gateway URL: {args.gateway}")
    logger.info(f"LLM Provider: {provider}, Model: {model}")
    server.run(host=args.host)


if __name__ == "__main__":
    main()
