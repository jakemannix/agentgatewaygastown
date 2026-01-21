"""Customer Agent implementation using Google ADK.

This agent handles shopping tasks:
- Product search and browsing
- Cart management
- Checkout and order tracking

Tools are dynamically discovered from the MCP gateway at startup.

Supports multiple LLM providers via LiteLLM:
- Anthropic (ANTHROPIC_API_KEY): anthropic/claude-sonnet-4-20250514
- OpenAI (OPENAI_API_KEY): openai/gpt-4o
- Google (GOOGLE_API_KEY): gemini-2.0-flash
"""

import asyncio
import logging
import os
from typing import Optional

from google.adk import Agent
from google.adk.runners import Runner
from google.adk.sessions import InMemorySessionService
from google.genai import types

from ..shared.gateway_client import GatewayMCPClient, discover_and_create_tools

logger = logging.getLogger(__name__)

# Configuration
AGENT_NAME = "customer-agent"
GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:3000")

# Tools this agent is allowed to use (customer-facing tools)
# These names must match what's returned by the gateway in multiplexing mode
# Backend tools are prefixed with their service name (e.g., "catalog-service_")
# Composition tools use their simple names (e.g., "personalized_search")
ALLOWED_TOOLS = {
    # Product discovery (catalog-service)
    "catalog-service_find_products",
    "catalog-service_browse_products",
    "catalog-service_search_products",
    "catalog-service_get_product_details",
    "catalog-service_get_categories",
    # Composition tools for enhanced search (no prefix)
    "personalized_search",
    "product_with_availability",
    # Cart management (cart-service)
    "cart-service_my_cart",
    "cart-service_view_cart",
    "cart-service_add_item",
    "cart-service_add_to_cart",
    "cart-service_update_cart_item",
    "cart-service_remove_from_cart",
    "cart-service_clear_cart",
    # Orders (order-service)
    "order-service_checkout",
    "order-service_get_order",
    "order-service_list_orders",
}

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


# Global state for gateway client and tools
_gateway_client: Optional[GatewayMCPClient] = None
_adk_tools: Optional[list] = None
_tools_initialized = False


async def _initialize_tools():
    """Initialize tools from gateway (called once at startup)."""
    global _gateway_client, _adk_tools, _tools_initialized

    if _tools_initialized:
        return

    logger.info(f"Discovering tools from gateway: {GATEWAY_URL}")
    logger.info(f"Allowed tools for customer agent: {sorted(ALLOWED_TOOLS)}")

    _gateway_client, _adk_tools = await discover_and_create_tools(
        gateway_url=GATEWAY_URL,
        agent_name=AGENT_NAME,
        allowed_tools=ALLOWED_TOOLS,
    )

    _tools_initialized = True
    logger.info(f"Customer agent initialized with {len(_adk_tools)} tools")


def _get_tools() -> list:
    """Get the initialized tools (must call _initialize_tools first)."""
    if not _tools_initialized or _adk_tools is None:
        raise RuntimeError("Tools not initialized. Call _initialize_tools() first.")
    return _adk_tools


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
- Use personalized_search when you want to provide personalized results for a user
- Use product_with_availability to check real-time stock for a product
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
    tools = _get_tools()

    logger.info(f"Creating customer agent with model: {model}, tools: {len(tools)}")

    # Inject user_id into system prompt
    system_prompt = CUSTOMER_SYSTEM_PROMPT + f"\n\nCurrent user_id: {user_id}"

    return Agent(
        name="customer_shopping_agent",
        model=model,
        description="Shopping assistant that helps customers find products, manage cart, and place orders",
        instruction=system_prompt,
        tools=tools,
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

    # Ensure tools are initialized
    await _initialize_tools()

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
