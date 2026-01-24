"""Customer Agent implementation using Google ADK.

This agent handles shopping tasks:
- Product search and browsing
- Cart management
- Checkout and order tracking

Tools are dynamically discovered from the MCP gateway via McpToolset.

Supports multiple LLM providers via LiteLLM:
- Anthropic (ANTHROPIC_API_KEY): anthropic/claude-sonnet-4-20250514
- OpenAI (OPENAI_API_KEY): openai/gpt-4o
- Google (GOOGLE_API_KEY): gemini-2.0-flash
"""

import logging
import os
from typing import Optional

from google.adk import Agent
from google.adk.runners import Runner
from google.adk.sessions import InMemorySessionService
from google.adk.tools.mcp_tool import McpToolset
from google.adk.tools.mcp_tool.mcp_toolset import StreamableHTTPConnectionParams
from google.genai import types

logger = logging.getLogger(__name__)

# Configuration
AGENT_NAME = "customer-agent"
AGENT_VERSION = "1.0.0"
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


def _create_mcp_toolset() -> McpToolset:
    """Create McpToolset connected to the gateway.

    The gateway filters tools based on agent identity (via clientInfo.name
    in the MCP initialize request). Tool filtering is handled server-side
    based on the registry's agent dependencies configuration.
    """
    return McpToolset(
        connection_params=StreamableHTTPConnectionParams(
            url=f"{GATEWAY_URL}/mcp",
            headers={
                "X-Agent-Name": AGENT_NAME,
                "X-Agent-Version": AGENT_VERSION,
            },
        ),
    )


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
- Use virtual_personalized_search when you want to provide personalized results for a user
- Use virtual_product_with_availability to check real-time stock for a product
- Suggest related products when appropriate
- Confirm actions like adding to cart or completing checkout
- If a product is out of stock, let the customer know and suggest alternatives
- For checkout, always ask for a shipping address if not provided

The user_id will be provided in the context. Use it for cart and order operations.
"""


# Global session service (shared across requests)
_session_service: Optional[InMemorySessionService] = None
_runner: Optional[Runner] = None
_mcp_toolset: Optional[McpToolset] = None


def _get_session_service() -> InMemorySessionService:
    """Get or create the session service."""
    global _session_service
    if _session_service is None:
        _session_service = InMemorySessionService()
    return _session_service


def _get_mcp_toolset() -> McpToolset:
    """Get or create the MCP toolset."""
    global _mcp_toolset
    if _mcp_toolset is None:
        logger.info(f"Creating MCP toolset connected to gateway: {GATEWAY_URL}/mcp")
        logger.info(f"Agent identity: {AGENT_NAME} v{AGENT_VERSION}")
        _mcp_toolset = _create_mcp_toolset()
    return _mcp_toolset


def create_customer_agent(user_id: str = "default-user") -> Agent:
    """Create a customer shopping agent.

    Args:
        user_id: The user ID for cart/order operations

    Returns:
        Configured ADK Agent
    """
    model = get_configured_model()
    mcp_toolset = _get_mcp_toolset()

    logger.info(f"Creating customer agent with model: {model}")

    # Inject user_id into system prompt
    system_prompt = CUSTOMER_SYSTEM_PROMPT + f"\n\nCurrent user_id: {user_id}"

    return Agent(
        name="customer_shopping_agent",
        model=model,
        description="Shopping assistant that helps customers find products, manage cart, and place orders",
        instruction=system_prompt,
        tools=[mcp_toolset],
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
