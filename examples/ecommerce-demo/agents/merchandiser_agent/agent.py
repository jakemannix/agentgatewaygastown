"""Merchandiser Agent implementation using LangGraph.

This agent handles inventory and supply chain tasks:
- Inventory monitoring and low stock alerts
- Purchase order management
- Sales analytics
- Order fulfillment tracking

Supports multiple LLM providers:
- Anthropic (ANTHROPIC_API_KEY): claude-sonnet-4-20250514
- OpenAI (OPENAI_API_KEY): gpt-4o
- Google (GOOGLE_API_KEY): gemini-2.0-flash
"""

import asyncio
import logging
import os
from typing import Annotated, Any, Literal, Optional, TypedDict

from langchain_core.messages import AIMessage, HumanMessage, SystemMessage, ToolMessage
from langchain_core.tools import tool
from langgraph.graph import END, StateGraph
from langgraph.graph.message import add_messages
from langgraph.prebuilt import ToolNode

from ..shared.gateway_client import GatewayMCPClient

logger = logging.getLogger(__name__)

# Configuration
AGENT_NAME = "merchandiser-agent"
GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:3000")

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


# Tool implementations
@tool
def get_inventory_report() -> dict:
    """Get a comprehensive inventory status report.

    Returns summary statistics and per-product stock levels including
    total products, units, value, and low/out-of-stock counts.
    """
    return _call_gateway_tool("get_inventory_report", {})


@tool
def get_low_stock_alerts(threshold: Optional[int] = None) -> dict:
    """Get products that are below their reorder threshold.

    Args:
        threshold: Optional custom threshold. If not provided, uses
                   each product's individual reorder threshold.

    Returns:
        List of products needing restock with deficit amounts.
    """
    args = {}
    if threshold is not None:
        args["threshold"] = threshold
    return _call_gateway_tool("get_low_stock_alerts", args)


@tool
def get_restock_report() -> dict:
    """Get a comprehensive restock report with supplier options.

    Aggregates low stock items, available suppliers, and pending POs.

    Returns:
        Aggregated data for restock decision making.
    """
    # Aggregate from multiple tools since we don't have a combined endpoint
    low_stock = _call_gateway_tool("get_low_stock_alerts", {})
    suppliers = _call_gateway_tool("list_suppliers", {})
    pos = _call_gateway_tool("list_purchase_orders", {"status": "pending"})

    return {
        "low_stock_items": low_stock.get("alerts", []) if isinstance(low_stock, dict) else [],
        "suppliers": suppliers.get("suppliers", []) if isinstance(suppliers, dict) else [],
        "pending_orders": pos.get("purchase_orders", []) if isinstance(pos, dict) else [],
    }


@tool
def adjust_inventory(product_id: str, quantity_change: int, reason: str) -> dict:
    """Manually adjust inventory levels with audit trail.

    Args:
        product_id: The product to adjust
        quantity_change: Amount to add (positive) or remove (negative)
        reason: Reason for adjustment (e.g., 'damaged', 'found', 'correction')

    Returns:
        Adjustment confirmation with new stock level.
    """
    return _call_gateway_tool("adjust_inventory", {
        "product_id": product_id,
        "quantity_change": quantity_change,
        "reason": reason,
    })


@tool
def list_suppliers() -> dict:
    """Get all available suppliers.

    Returns:
        List of suppliers with lead times and reliability scores.
    """
    return _call_gateway_tool("list_suppliers", {})


@tool
def create_purchase_order(
    product_id: str,
    supplier_id: str,
    quantity: int,
    notes: Optional[str] = None,
) -> dict:
    """Create a purchase order to restock inventory.

    Args:
        product_id: The product to order
        supplier_id: The supplier to order from
        quantity: Quantity to order
        notes: Optional notes for the order

    Returns:
        Purchase order confirmation with expected delivery.
    """
    args = {
        "product_id": product_id,
        "supplier_id": supplier_id,
        "quantity": quantity,
    }
    if notes:
        args["notes"] = notes
    return _call_gateway_tool("create_purchase_order", args)


@tool
def list_purchase_orders(status: Optional[str] = None) -> dict:
    """List purchase orders with optional status filter.

    Args:
        status: Filter by status (pending, confirmed, shipped, received, cancelled)

    Returns:
        List of purchase orders.
    """
    args = {}
    if status:
        args["status"] = status
    return _call_gateway_tool("list_purchase_orders", args)


@tool
def receive_shipment(po_id: str, quantity_received: Optional[int] = None) -> dict:
    """Mark a purchase order as received and add stock to inventory.

    Args:
        po_id: The purchase order ID
        quantity_received: Actual quantity received (defaults to ordered quantity)

    Returns:
        Confirmation with updated stock level.
    """
    args = {"po_id": po_id}
    if quantity_received is not None:
        args["quantity_received"] = quantity_received
    return _call_gateway_tool("receive_shipment", args)


@tool
def get_pending_deliveries() -> dict:
    """Get list of pending purchase orders awaiting delivery.

    Returns:
        List of purchase orders with shipped/pending status.
    """
    pos = _call_gateway_tool("list_purchase_orders", {})
    if isinstance(pos, dict):
        all_orders = pos.get("purchase_orders", [])
        pending = [po for po in all_orders if po.get("status") in ("pending", "confirmed", "shipped")]
        return {"pending_deliveries": pending}
    return {"pending_deliveries": []}


@tool
def get_sales_report(start_date: Optional[str] = None, end_date: Optional[str] = None) -> dict:
    """Get sales analytics report.

    Args:
        start_date: Start date for report (ISO format)
        end_date: End date for report (ISO format)

    Returns:
        Sales data including revenue, units sold, and per-product breakdown.
    """
    args = {}
    if start_date:
        args["start_date"] = start_date
    if end_date:
        args["end_date"] = end_date
    return _call_gateway_tool("get_sales_report", args)


@tool
def update_order_status(order_id: str, status: str) -> dict:
    """Update customer order status.

    Args:
        order_id: The order to update
        status: New status (confirmed, shipped, delivered, cancelled)

    Returns:
        Updated order details.
    """
    return _call_gateway_tool("update_order_status", {
        "order_id": order_id,
        "status": status,
    })


@tool
def get_dashboard_data() -> dict:
    """Get all dashboard metrics.

    Aggregates inventory, alerts, POs, and sales data.

    Returns:
        Aggregated dashboard data.
    """
    inventory = _call_gateway_tool("get_inventory_report", {})
    alerts = _call_gateway_tool("get_low_stock_alerts", {})
    pos = _call_gateway_tool("list_purchase_orders", {})
    sales = _call_gateway_tool("get_sales_report", {})

    return {
        "inventory": inventory if isinstance(inventory, dict) else {},
        "low_stock_alerts": alerts.get("alerts", []) if isinstance(alerts, dict) else [],
        "purchase_orders": pos.get("purchase_orders", []) if isinstance(pos, dict) else [],
        "sales": sales if isinstance(sales, dict) else {},
    }


# All tools available to the agent
MERCHANDISER_TOOLS = [
    get_inventory_report,
    get_low_stock_alerts,
    get_restock_report,
    adjust_inventory,
    list_suppliers,
    create_purchase_order,
    list_purchase_orders,
    receive_shipment,
    get_pending_deliveries,
    get_sales_report,
    update_order_status,
    get_dashboard_data,
]

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
1. First check the restock report to see what needs ordering
2. Review supplier options (lead time, reliability)
3. Recommend appropriate quantities
4. Confirm the order with the user before creating

For inventory adjustments, always require a reason for audit purposes.
"""


# LangGraph state
class AgentState(TypedDict):
    """State for the merchandiser agent graph."""

    messages: Annotated[list, add_messages]


def create_merchandiser_graph():
    """Create the LangGraph workflow for the merchandiser agent."""
    llm = get_llm()
    llm_with_tools = llm.bind_tools(MERCHANDISER_TOOLS)

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
    workflow.add_node("tools", ToolNode(MERCHANDISER_TOOLS))

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
        _graph = create_merchandiser_graph()
    return _graph


async def run_merchandiser_query(query: str) -> str:
    """Run a query through the merchandiser agent.

    Args:
        query: The merchandiser's question or task

    Returns:
        Agent's response
    """
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
