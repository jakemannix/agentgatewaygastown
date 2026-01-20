"""Customer Agent implementation using Google ADK.

This agent handles shopping tasks:
- Product search and browsing
- Cart management
- Checkout and order tracking

Supports multiple LLM providers via LiteLLM:
- Anthropic (ANTHROPIC_API_KEY): anthropic/claude-sonnet-4-20250514
- OpenAI (OPENAI_API_KEY): openai/gpt-4o
- Google (GOOGLE_API_KEY): gemini-2.0-flash
"""

import asyncio
import logging
import os
from typing import Any, Optional

from google.adk import Agent
from google.adk.runners import Runner
from google.adk.sessions import InMemorySessionService
from google.adk.tools import FunctionTool
from google.genai import types

from ..shared.gateway_client import GatewayMCPClient

logger = logging.getLogger(__name__)

# Configuration
AGENT_NAME = "customer-agent"
GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:3000")

# LLM Provider Configuration (using LiteLLM format for non-Google models)
DEFAULT_MODELS = {
    "anthropic": "anthropic/claude-sonnet-4-20250514",
    "openai": "openai/gpt-4o",
    "google": "gemini-2.0-flash",
}

API_KEY_ENV_VARS = {
    "anthropic": "ANTHROPIC_API_KEY",
    "openai": "OPENAI_API_KEY",
    "google": "GOOGLE_API_KEY",
}

# Provider detection priority
PROVIDER_PRIORITY = ["anthropic", "openai", "google"]


def detect_llm_provider() -> str | None:
    """Detect which LLM provider is available based on API keys."""
    for provider in PROVIDER_PRIORITY:
        env_var = API_KEY_ENV_VARS[provider]
        if os.environ.get(env_var):
            return provider
    return None


def get_configured_model() -> str:
    """Get the configured LLM model with multi-provider support.

    Checks in order:
    1. LLM_MODEL env var (explicit model override)
    2. LLM_PROVIDER env var (explicit provider selection)
    3. Auto-detect from available API keys (Anthropic > OpenAI > Google)
    """
    # Check for explicit model override
    if model := os.environ.get("LLM_MODEL"):
        logger.info(f"Using explicit model override: {model}")
        return model

    # Check for explicit provider selection
    if provider := os.environ.get("LLM_PROVIDER", "").lower():
        if provider in DEFAULT_MODELS:
            model = DEFAULT_MODELS[provider]
            logger.info(f"Using {provider} provider: {model}")
            return model

    # Auto-detect from API keys
    provider = detect_llm_provider()
    if provider:
        model = DEFAULT_MODELS[provider]
        logger.info(f"Auto-detected {provider} provider: {model}")
        return model

    # No API key found - this will fail at runtime
    raise RuntimeError(
        "No LLM API key found. Set one of: "
        f"{', '.join(API_KEY_ENV_VARS.values())}"
    )


# Initialize gateway client
gateway_client: Optional[GatewayMCPClient] = None


def get_gateway_client() -> GatewayMCPClient:
    """Get or create the gateway client."""
    global gateway_client
    if gateway_client is None:
        gateway_client = GatewayMCPClient(
            gateway_url=GATEWAY_URL,
            agent_name=AGENT_NAME,
        )
    return gateway_client


def _call_gateway_tool(name: str, args: dict) -> Any:
    """Call a gateway tool, handling both sync and async contexts."""
    import concurrent.futures

    client = get_gateway_client()

    try:
        # Check if there's already a running event loop
        loop = asyncio.get_running_loop()
    except RuntimeError:
        # No running loop - safe to use asyncio.run()
        return asyncio.run(client.call_tool(name, args))

    # There's a running loop - run in a separate thread to avoid conflicts
    with concurrent.futures.ThreadPoolExecutor(max_workers=1) as executor:
        future = executor.submit(asyncio.run, client.call_tool(name, args))
        return future.result()


# Tool implementations that call through the gateway
def search_products(query: str, category: Optional[str] = None, max_results: int = 10) -> dict:
    """Search for products using natural language.

    Args:
        query: Natural language search query (e.g., "wireless headphones", "camping gear")
        category: Optional category filter (e.g., "Electronics", "Sports & Outdoors")
        max_results: Maximum number of results to return (default 10)

    Returns:
        Search results with matching products
    """
    args = {"query": query, "max_results": max_results}
    if category:
        args["category"] = category
    return _call_gateway_tool("search_products", args)


def browse_products(category: Optional[str] = None, page: int = 1, limit: int = 10) -> dict:
    """Browse the product catalog.

    Args:
        category: Optional category to filter by
        page: Page number for pagination (default 1)
        limit: Number of products per page (default 10)

    Returns:
        List of products in the catalog
    """
    args = {"page": page, "limit": limit}
    if category:
        args["category"] = category
    return _call_gateway_tool("browse_products", args)


def get_product_details(product_id: str) -> dict:
    """Get detailed information about a specific product.

    Args:
        product_id: The product ID to look up

    Returns:
        Product details including name, description, price, and availability
    """
    return _call_gateway_tool("get_product_details", {"product_id": product_id})


def get_categories() -> dict:
    """Get all available product categories.

    Returns:
        List of category names
    """
    return _call_gateway_tool("get_categories", {})


def view_cart(user_id: str) -> dict:
    """View the contents of the shopping cart.

    Args:
        user_id: The user's ID

    Returns:
        Cart contents including items, quantities, and total
    """
    return _call_gateway_tool("view_cart", {"user_id": user_id})


def add_to_cart(user_id: str, product_id: str, quantity: int = 1) -> dict:
    """Add a product to the shopping cart.

    Args:
        user_id: The user's ID
        product_id: The product to add
        quantity: Number of items to add (default 1)

    Returns:
        Updated cart information
    """
    return _call_gateway_tool("add_to_cart", {
        "user_id": user_id,
        "product_id": product_id,
        "quantity": quantity,
    })


def update_cart_item(cart_item_id: str, quantity: int) -> dict:
    """Update the quantity of an item in the cart.

    Args:
        cart_item_id: The cart item ID to update
        quantity: New quantity (0 to remove)

    Returns:
        Updated cart item information
    """
    return _call_gateway_tool("update_cart_item", {
        "cart_item_id": cart_item_id,
        "quantity": quantity,
    })


def checkout(user_id: str, shipping_address: str) -> dict:
    """Complete checkout and place an order.

    Args:
        user_id: The user's ID
        shipping_address: Delivery address for the order

    Returns:
        Order confirmation with order ID and details
    """
    return _call_gateway_tool("checkout", {
        "user_id": user_id,
        "shipping_address": shipping_address,
    })


def get_order(order_id: str) -> dict:
    """Get details of a specific order.

    Args:
        order_id: The order ID to look up

    Returns:
        Order details including items, status, and shipping info
    """
    return _call_gateway_tool("get_order", {"order_id": order_id})


def list_my_orders(user_id: str, status: Optional[str] = None) -> dict:
    """List orders for the current user.

    Args:
        user_id: The user's ID
        status: Optional status filter (pending, confirmed, shipped, delivered)

    Returns:
        List of orders
    """
    args = {"user_id": user_id}
    if status:
        args["status"] = status
    return _call_gateway_tool("list_orders", args)


# Create ADK tools
CUSTOMER_TOOLS = [
    FunctionTool(search_products),
    FunctionTool(browse_products),
    FunctionTool(get_product_details),
    FunctionTool(get_categories),
    FunctionTool(view_cart),
    FunctionTool(add_to_cart),
    FunctionTool(update_cart_item),
    FunctionTool(checkout),
    FunctionTool(get_order),
    FunctionTool(list_my_orders),
]

# System prompt for the customer agent
CUSTOMER_SYSTEM_PROMPT = """You are a helpful shopping assistant for an ecommerce store.

Your role is to help customers:
- Find products they're looking for using search or browsing
- Get detailed product information
- Manage their shopping cart (add items, update quantities)
- Complete checkout with their shipping address
- Track their orders

Guidelines:
- Always be helpful and friendly
- When searching, use natural language queries that capture what the customer wants
- Suggest related products when appropriate
- Confirm actions like adding to cart or completing checkout
- If a product is out of stock, let the customer know and suggest alternatives
- For checkout, always ask for a shipping address if not provided

The user_id will be provided in the context. Use it for cart and order operations.
"""


# Global session service (shared across requests)
_session_service: Optional[InMemorySessionService] = None
_runner: Optional[Runner] = None


def _get_session_service() -> InMemorySessionService:
    """Get or create the session service."""
    global _session_service
    if _session_service is None:
        _session_service = InMemorySessionService()
    return _session_service


def create_customer_agent(user_id: str = "default-user") -> Agent:
    """Create a customer shopping agent.

    Args:
        user_id: The user ID for cart/order operations

    Returns:
        Configured ADK Agent
    """
    model = get_configured_model()
    logger.info(f"Creating customer agent with model: {model}")

    # Inject user_id into system prompt
    system_prompt = CUSTOMER_SYSTEM_PROMPT + f"\n\nCurrent user_id: {user_id}"

    return Agent(
        name="customer_shopping_agent",
        model=model,
        description="Shopping assistant that helps customers find products, manage cart, and place orders",
        instruction=system_prompt,
        tools=CUSTOMER_TOOLS,
    )


def get_runner(user_id: str = "default-user") -> Runner:
    """Get or create the Runner with session service.

    Args:
        user_id: The user ID for the agent context

    Returns:
        Configured Runner instance
    """
    global _runner
    if _runner is None:
        agent = create_customer_agent(user_id)
        session_service = _get_session_service()
        _runner = Runner(
            app_name="customer_shopping_app",
            agent=agent,
            session_service=session_service,
        )
    return _runner


APP_NAME = "customer_shopping_app"


async def _ensure_session(session_service: InMemorySessionService, user_id: str, session_id: str):
    """Ensure a session exists, creating it if needed."""
    session = await session_service.get_session(
        app_name=APP_NAME,
        user_id=user_id,
        session_id=session_id,
    )
    if session is None:
        # Session doesn't exist, create it
        session = await session_service.create_session(
            app_name=APP_NAME,
            user_id=user_id,
            session_id=session_id,
        )
    return session


async def run_customer_query(
    query: str,
    user_id: str = "default-user",
    session_id: str = "default-session",
) -> str:
    """Run a single query through the customer agent.

    Args:
        query: The customer's question or request
        user_id: The user ID for personalization
        session_id: Session ID for conversation continuity

    Returns:
        Agent's response

    Raises:
        RuntimeError: If no LLM API key is configured or agent fails
    """
    # Validate that at least one provider is configured
    provider = detect_llm_provider()
    if not provider:
        raise RuntimeError(
            "No LLM API key found. Set one of: "
            f"{', '.join(API_KEY_ENV_VARS.values())}"
        )

    runner = get_runner(user_id)
    session_service = _get_session_service()

    # Ensure session exists
    await _ensure_session(session_service, user_id, session_id)

    # Create the message
    message = types.Content(
        role="user",
        parts=[types.Part(text=query)],
    )

    # Run the agent
    logger.info(f"Running agent for user={user_id}, session={session_id}")
    response_parts = []

    async for event in runner.run_async(
        user_id=user_id,
        session_id=session_id,
        new_message=message,
    ):
        # Collect response parts
        if hasattr(event, "content") and event.content:
            for part in event.content.parts:
                if hasattr(part, "text") and part.text:
                    response_parts.append(part.text)

    if not response_parts:
        raise RuntimeError("Agent returned empty response")

    return "".join(response_parts)
