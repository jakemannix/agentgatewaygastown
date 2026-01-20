"""Merchandiser Agent A2A Server - Entry point.

This agent uses LangGraph with multi-provider support.
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
AGENT_NAME = "Merchandiser Agent"
AGENT_PORT = int(os.environ.get("MERCHANDISER_AGENT_PORT", 9002))
GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:3000")

# Agent skills for A2A discovery
SKILLS = [
    {
        "id": "inventory-monitoring",
        "name": "Inventory Monitoring",
        "description": "Monitor stock levels and identify low stock items",
        "tags": ["inventory", "stock", "alerts"],
        "examples": [
            "Show inventory status",
            "What items are low on stock?",
            "Get inventory report",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
    {
        "id": "purchase-orders",
        "name": "Purchase Order Management",
        "description": "Create and manage purchase orders with suppliers",
        "tags": ["orders", "suppliers", "restock"],
        "examples": [
            "Create a purchase order",
            "Show pending purchase orders",
            "What needs to be restocked?",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
    {
        "id": "supplier-management",
        "name": "Supplier Management",
        "description": "View and manage supplier relationships",
        "tags": ["suppliers", "vendors"],
        "examples": [
            "List suppliers",
            "Which supplier is most reliable?",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
    {
        "id": "sales-analytics",
        "name": "Sales Analytics",
        "description": "Analyze sales data and trends",
        "tags": ["sales", "analytics", "reports"],
        "examples": [
            "Show sales report",
            "What are the top selling products?",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
    {
        "id": "order-fulfillment",
        "name": "Order Fulfillment",
        "description": "Manage customer order status and fulfillment",
        "tags": ["orders", "fulfillment", "shipping"],
        "examples": [
            "Update order status to shipped",
            "Show pending customer orders",
        ],
        "inputModes": ["text"],
        "outputModes": ["text"],
    },
]


async def handle_merchandiser_message(message_text: str, context: dict) -> str:
    """Handle incoming merchandiser messages using the LangGraph agent.

    Args:
        message_text: The user's message
        context: Request context including session info

    Returns:
        Agent's response

    Raises:
        RuntimeError: If no LLM API key is configured or agent fails
    """
    from .agent import run_merchandiser_query

    logger.info(f"Processing merchandiser message: {message_text[:100]}...")

    # Run the agent - let errors propagate
    response = await run_merchandiser_query(message_text)
    logger.info(f"Agent response: {response[:100]}...")
    return response


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Merchandiser Agent (LangGraph)")
    parser.add_argument("--port", type=int, default=AGENT_PORT, help="Port to run on")
    parser.add_argument("--gateway", default=GATEWAY_URL, help="Gateway URL")
    parser.add_argument("--host", default="0.0.0.0", help="Host to bind to")
    args = parser.parse_args()

    # Check for LLM provider
    from .agent import detect_llm_provider, API_KEY_ENV_VARS, DEFAULT_MODELS

    provider = detect_llm_provider()
    if not provider:
        logger.error(
            "No LLM API key found.\n"
            "The Merchandiser Agent requires one of:\n"
            f"  {', '.join(API_KEY_ENV_VARS.values())}\n"
            "Example: export ANTHROPIC_API_KEY='your-key-here'"
        )
        sys.exit(1)

    # Get the model that will be used
    model = os.environ.get("LLM_MODEL") or DEFAULT_MODELS[provider]

    # Update environment
    os.environ["GATEWAY_URL"] = args.gateway

    # Create and configure A2A server
    server = A2AServer(
        name=AGENT_NAME,
        description="AI assistant for inventory management, purchase orders, and supply chain operations (powered by LangGraph)",
        port=args.port,
        skills=SKILLS,
    )

    # Set the message handler
    server.set_message_handler(handle_merchandiser_message)

    # Run the server
    logger.info(f"Merchandiser Agent starting on port {args.port}")
    logger.info(f"Gateway URL: {args.gateway}")
    logger.info(f"LLM Provider: {provider}, Model: {model}")
    server.run(host=args.host)


if __name__ == "__main__":
    main()
