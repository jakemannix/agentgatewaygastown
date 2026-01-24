"""Merchandiser Agent implementation using LangGraph.

This agent handles inventory and supply chain tasks:
- Inventory monitoring and low stock alerts
- Purchase order management
- Sales analytics
- Order fulfillment tracking

Tools are dynamically discovered from the MCP gateway at startup.

Supports multiple LLM providers:
- Anthropic (ANTHROPIC_API_KEY): claude-sonnet-4-20250514
- OpenAI (OPENAI_API_KEY): gpt-4o
- Google (GOOGLE_API_KEY): gemini-2.0-flash
"""

import asyncio
import logging
import os
from typing import Annotated, Literal, Optional, TypedDict

from langchain_core.messages import HumanMessage, SystemMessage
from langgraph.graph import END, StateGraph
from langgraph.graph.message import add_messages
from langgraph.prebuilt import ToolNode

from ..shared.gateway_client import GatewayMCPClient, discover_and_create_langchain_tools

logger = logging.getLogger(__name__)

# Configuration
AGENT_NAME = "merchandiser-agent"
GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:3000")

# Tools this agent is allowed to use (merchandiser/admin tools)
# All registry virtual tools use "virtual_" prefix
ALLOWED_TOOLS = {
    # Inventory management
    "virtual_stock_status",
    "virtual_get_inventory_report",
    "virtual_get_low_stock_alerts",
    "virtual_adjust_inventory",
    "virtual_check_stock",
    # Supplier management
    "virtual_list_suppliers",
    "virtual_get_supplier",
    "virtual_create_purchase_order",
    "virtual_list_purchase_orders",
    "virtual_receive_shipment",
    "virtual_get_all_supplier_quotes",
    # Sales and orders
    "virtual_get_sales_report",
    "virtual_update_order_status",
    "virtual_list_orders",
    "virtual_get_order",
    # Product management (read-only for merchandisers)
    "virtual_browse_products",
    "virtual_get_product_details",
    "virtual_get_categories",
}

# LLM Provider Configuration
DEFAULT_MODELS = {
    "anthropic": "claude-sonnet-4-20250514",
    "openai": "gpt-4o",
    "google": "gemini-2.0-flash",
}

API_KEY_ENV_VARS = {
    "anthropic": "ANTHROPIC_API_KEY",
    "openai": "OPENAI_API_KEY",
    "google": "GOOGLE_API_KEY",
}

# Provider detection priority (same as Customer Agent)
PROVIDER_PRIORITY = ["anthropic", "openai", "google"]


def detect_llm_provider() -> str | None:
    """Detect which LLM provider is available based on API keys."""
    for provider in PROVIDER_PRIORITY:
        env_var = API_KEY_ENV_VARS[provider]
        if os.environ.get(env_var):
            return provider
    return None


def get_llm():
    """Get the configured LLM based on environment.

    Checks in order:
    1. LLM_MODEL env var (explicit model override)
    2. LLM_PROVIDER env var (explicit provider selection)
    3. Auto-detect from available API keys (Anthropic > OpenAI > Google)
    """
    # Check for explicit provider selection
    provider = os.environ.get("LLM_PROVIDER", "").lower()
    if not provider or provider not in DEFAULT_MODELS:
        provider = detect_llm_provider()

    if not provider:
        raise RuntimeError(
            "No LLM API key found. Set one of: "
            f"{', '.join(API_KEY_ENV_VARS.values())}"
        )

    # Get model (allow override)
    model = os.environ.get("LLM_MODEL") or DEFAULT_MODELS[provider]
    logger.info(f"Using {provider} provider with model: {model}")

    if provider == "anthropic":
        from langchain_anthropic import ChatAnthropic
        return ChatAnthropic(model=model, temperature=0)
    elif provider == "openai":
        from langchain_openai import ChatOpenAI
        return ChatOpenAI(model=model, temperature=0)
    elif provider == "google":
        from langchain_google_genai import ChatGoogleGenerativeAI
        return ChatGoogleGenerativeAI(model=model, temperature=0)
    else:
        raise RuntimeError(f"Unknown provider: {provider}")


# Global state for gateway client and tools
_gateway_client: Optional[GatewayMCPClient] = None
_langchain_tools: Optional[list] = None
_tools_initialized = False


async def _initialize_tools():
    """Initialize tools from gateway (called once at startup)."""
    global _gateway_client, _langchain_tools, _tools_initialized

    if _tools_initialized:
        return

    logger.info(f"Discovering tools from gateway: {GATEWAY_URL}")
    logger.info(f"Allowed tools for merchandiser agent: {sorted(ALLOWED_TOOLS)}")

    _gateway_client, _langchain_tools = await discover_and_create_langchain_tools(
        gateway_url=GATEWAY_URL,
        agent_name=AGENT_NAME,
        allowed_tools=ALLOWED_TOOLS,
    )

    _tools_initialized = True
    logger.info(f"Merchandiser agent initialized with {len(_langchain_tools)} tools")


def _get_tools() -> list:
    """Get the initialized tools (must call _initialize_tools first)."""
    if not _tools_initialized or _langchain_tools is None:
        raise RuntimeError("Tools not initialized. Call _initialize_tools() first.")
    return _langchain_tools


# System prompt
MERCHANDISER_SYSTEM_PROMPT = """You are an inventory and supply chain management assistant for an ecommerce business.

Your role is to help merchandisers:
- Monitor inventory levels and identify low stock items
- Create and manage purchase orders with suppliers
- Track deliveries and receive shipments
- Analyze sales data and trends
- Manage customer order fulfillment

Guidelines:
- Proactively alert about low stock situations
- Consider supplier lead times and reliability when recommending orders
- Suggest optimal reorder quantities based on stock levels and thresholds
- Provide clear summaries of inventory status
- Help prioritize which items to restock first
- Track pending purchase orders and expected deliveries

When creating purchase orders:
1. First check inventory status to see what needs ordering
2. Review supplier options (lead time, reliability)
3. Recommend appropriate quantities
4. Confirm the order with the user before creating

For inventory adjustments, always require a reason for audit purposes.
"""


# LangGraph state
class AgentState(TypedDict):
    """State for the merchandiser agent graph."""

    messages: Annotated[list, add_messages]


def create_merchandiser_graph(tools: list):
    """Create the LangGraph workflow for the merchandiser agent.

    Args:
        tools: List of LangChain tools to use
    """
    llm = get_llm()
    llm_with_tools = llm.bind_tools(tools)

    def should_continue(state: AgentState) -> Literal["tools", END]:
        """Determine if we should continue to tools or end."""
        messages = state["messages"]
        last_message = messages[-1]

        if hasattr(last_message, "tool_calls") and last_message.tool_calls:
            return "tools"
        return END

    def call_model(state: AgentState) -> dict:
        """Call the LLM with the current state."""
        messages = state["messages"]

        # Add system message if not present
        if not messages or not isinstance(messages[0], SystemMessage):
            messages = [SystemMessage(content=MERCHANDISER_SYSTEM_PROMPT)] + messages

        response = llm_with_tools.invoke(messages)
        return {"messages": [response]}

    # Create the graph
    workflow = StateGraph(AgentState)

    # Add nodes
    workflow.add_node("agent", call_model)
    workflow.add_node("tools", ToolNode(tools))

    # Set entry point
    workflow.set_entry_point("agent")

    # Add edges
    workflow.add_conditional_edges("agent", should_continue)
    workflow.add_edge("tools", "agent")

    return workflow.compile()


# Compiled graph (lazy loaded)
_graph = None


def get_graph():
    """Get or create the compiled graph."""
    global _graph
    if _graph is None:
        tools = _get_tools()
        _graph = create_merchandiser_graph(tools)
    return _graph


async def run_merchandiser_query(query: str) -> str:
    """Run a query through the merchandiser agent.

    Args:
        query: The merchandiser's question or task

    Returns:
        Agent's response
    """
    # Ensure tools are initialized
    await _initialize_tools()

    graph = get_graph()

    # Run the graph
    result = await asyncio.to_thread(
        graph.invoke,
        {"messages": [HumanMessage(content=query)]},
    )

    # Extract the final response
    messages = result.get("messages", [])
    if messages:
        last_message = messages[-1]
        if hasattr(last_message, "content"):
            return last_message.content
        return str(last_message)

    return "No response generated."
