"""Customer Agent A2A Server - Entry point.

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
AGENT_NAME = "Customer Shopping Agent"
AGENT_PORT = int(os.environ.get("CUSTOMER_AGENT_PORT", 9001))
GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:3000")

# Agent skills for A2A discovery
SKILLS = [
    {
        "id": "product-search",
        "name": "Product Search",
        "description": "Search for products using natural language queries",
        "tags": ["search", "products", "catalog"],
        "examples": [
            "Find wireless headphones under $100",
            "Show me camping gear",
            "Search for kitchen appliances",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
    {
        "id": "cart-management",
        "name": "Cart Management",
        "description": "Add products to cart, update quantities, view cart",
        "tags": ["cart", "shopping"],
        "examples": [
            "Add this to my cart",
            "Show my cart",
            "Remove item from cart",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
    {
        "id": "checkout",
        "name": "Checkout",
        "description": "Complete purchase with shipping address",
        "tags": ["checkout", "order", "purchase"],
        "examples": [
            "Checkout with my items",
            "Place order to 123 Main St",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
    {
        "id": "order-tracking",
        "name": "Order Tracking",
        "description": "View order history and track order status",
        "tags": ["orders", "tracking", "history"],
        "examples": [
            "Show my orders",
            "Track order status",
            "What's the status of my last order",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
]


async def handle_customer_message(message_text: str, context: dict) -> str:
    """Handle incoming customer messages using the Google ADK agent.

    Args:
        message_text: The user's message
        context: Request context including session info

    Returns:
        Agent's response

    Raises:
        RuntimeError: If GOOGLE_API_KEY is not set or agent fails
    """
    from .agent import run_customer_query

    # Extract session info (handle both A2A context_id and REST session_id)
    session_id = context.get("session_id") or context.get("context_id") or "default-session"
    user_id = context.get("user_id", session_id[:8])

    logger.info(f"Processing message for user={user_id}, session={session_id}: {message_text[:100]}...")

    # Run the agent - let errors propagate
    response = await run_customer_query(
        query=message_text,
        user_id=user_id,
        session_id=session_id,
    )

    logger.info(f"Agent response: {response[:100]}...")
    return response


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Customer Shopping Agent (Google ADK)")
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
            "The Customer Agent requires one of:\n"
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
        description="AI shopping assistant that helps customers find products, manage cart, and place orders (powered by Google ADK)",
        port=args.port,
        skills=SKILLS,
    )

    # Set the message handler
    server.set_message_handler(handle_customer_message)

    # Run the server
    logger.info(f"Customer Agent starting on port {args.port}")
    logger.info(f"Gateway URL: {args.gateway}")
    logger.info(f"LLM Provider: {provider}, Model: {model}")
    server.run(host=args.host)


if __name__ == "__main__":
    main()
