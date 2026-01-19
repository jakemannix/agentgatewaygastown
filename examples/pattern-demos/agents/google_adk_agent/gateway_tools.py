"""
AgentGateway MCP Tools Integration for Google ADK

This module provides integration between Google ADK agents and agentgateway's
MCP (Model Context Protocol) tools. It demonstrates how to:

1. Connect to an agentgateway instance
2. Discover available MCP tools
3. Create ADK-compatible tool wrappers
4. Handle tool execution through the gateway
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any

import httpx
from google.adk.tools import FunctionTool
from pydantic import BaseModel

logger = logging.getLogger(__name__)

# Agent identity - used by gateway to scope tool visibility
AGENT_NAME = "adk-demo-agent"
AGENT_VERSION = "1.0.0"


def get_agent_identity_headers() -> dict[str, str]:
    """Get headers for agent identity, used by gateway for tool scoping."""
    return {
        "X-Agent-Name": AGENT_NAME,
        "X-Agent-Version": AGENT_VERSION,
    }


class MCPToolSchema(BaseModel):
    """Schema for an MCP tool definition."""

    name: str
    description: str | None = None
    input_schema: dict[str, Any] = {}


class AgentGatewayMCPClient:
    """
    Client for interacting with agentgateway's MCP endpoint.

    This client connects to an agentgateway instance and provides methods
    to discover and invoke MCP tools.
    """

    def __init__(
        self,
        gateway_url: str = "http://localhost:3000",
        timeout: float = 30.0,
        headers: dict[str, str] | None = None,
    ):
        """
        Initialize the gateway client.

        Args:
            gateway_url: URL of the agentgateway MCP endpoint
            timeout: Request timeout in seconds
            headers: Optional headers for authentication
        """
        self.gateway_url = gateway_url.rstrip("/")
        self.timeout = timeout
        self.headers = headers or {}
        self._request_id = 0

    def _next_request_id(self) -> int:
        """Generate the next request ID."""
        self._request_id += 1
        return self._request_id

    async def _send_jsonrpc(
        self,
        method: str,
        params: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        """
        Send a JSON-RPC request to the gateway.

        Args:
            method: The JSON-RPC method name
            params: Optional parameters

        Returns:
            The response result
        """
        request = {
            "jsonrpc": "2.0",
            "id": self._next_request_id(),
            "method": method,
        }
        if params:
            request["params"] = params

        async with httpx.AsyncClient(timeout=self.timeout) as client:
            response = await client.post(
                f"{self.gateway_url}/mcp",
                json=request,
                headers={
                    "Content-Type": "application/json",
                    **self.headers,
                },
            )
            response.raise_for_status()
            result = response.json()

            if "error" in result:
                raise RuntimeError(f"MCP error: {result['error']}")

            return result.get("result", {})

    async def list_tools(self) -> list[MCPToolSchema]:
        """
        List available tools from the gateway.

        Returns:
            List of tool definitions
        """
        result = await self._send_jsonrpc("tools/list")
        tools = result.get("tools", [])
        return [MCPToolSchema(**tool) for tool in tools]

    async def call_tool(
        self,
        name: str,
        arguments: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        """
        Call a tool through the gateway.

        Args:
            name: Tool name
            arguments: Tool arguments

        Returns:
            Tool execution result
        """
        result = await self._send_jsonrpc(
            "tools/call",
            params={
                "name": name,
                "arguments": arguments or {},
            },
        )
        return result


def create_gateway_tool(
    client: AgentGatewayMCPClient,
    tool_schema: MCPToolSchema,
) -> FunctionTool:
    """
    Create an ADK FunctionTool that wraps an MCP tool from the gateway.

    Args:
        client: The gateway client
        tool_schema: The tool's schema from the gateway

    Returns:
        An ADK-compatible FunctionTool
    """

    async def tool_wrapper(**kwargs: Any) -> dict[str, Any]:
        """Wrapper function that calls the gateway tool."""
        try:
            result = await client.call_tool(tool_schema.name, kwargs)
            return {
                "status": "success",
                "tool": tool_schema.name,
                "result": result,
            }
        except Exception as e:
            logger.error(f"Tool {tool_schema.name} failed: {e}")
            return {
                "status": "error",
                "tool": tool_schema.name,
                "error": str(e),
            }

    # Create sync wrapper for ADK
    def sync_wrapper(**kwargs: Any) -> dict[str, Any]:
        """Synchronous wrapper for the async tool."""
        return asyncio.get_event_loop().run_until_complete(tool_wrapper(**kwargs))

    # Set function metadata
    sync_wrapper.__name__ = tool_schema.name.replace(":", "_")
    sync_wrapper.__doc__ = tool_schema.description or f"Call {tool_schema.name} via gateway"

    return FunctionTool(sync_wrapper)


async def discover_gateway_tools(
    gateway_url: str = "http://localhost:3000",
    headers: dict[str, str] | None = None,
) -> list[FunctionTool]:
    """
    Discover and create ADK tools from an agentgateway instance.

    This function connects to the gateway, lists available MCP tools,
    and creates ADK-compatible FunctionTool wrappers for each.

    Args:
        gateway_url: URL of the agentgateway
        headers: Optional authentication headers (agent identity added automatically)

    Returns:
        List of ADK FunctionTools
    """
    # Merge agent identity headers with any provided headers
    all_headers = get_agent_identity_headers()
    if headers:
        all_headers.update(headers)

    client = AgentGatewayMCPClient(
        gateway_url=gateway_url,
        headers=all_headers,
    )

    try:
        tools = await client.list_tools()
        logger.info(f"Discovered {len(tools)} tools from gateway")

        adk_tools = []
        for tool in tools:
            adk_tool = create_gateway_tool(client, tool)
            adk_tools.append(adk_tool)
            logger.debug(f"Created ADK tool wrapper for {tool.name}")

        return adk_tools

    except Exception as e:
        logger.error(f"Failed to discover gateway tools: {e}")
        return []


def create_gateway_echo_tool(gateway_url: str = "http://localhost:3000") -> FunctionTool:
    """
    Create a simple echo tool that uses the gateway's everything:echo.

    This is a convenience function for testing gateway connectivity.

    Args:
        gateway_url: URL of the agentgateway

    Returns:
        An ADK FunctionTool for the echo operation
    """
    client = AgentGatewayMCPClient(gateway_url=gateway_url)

    def echo_via_gateway(message: str) -> dict[str, Any]:
        """
        Echo a message through the agentgateway.

        Args:
            message: Message to echo

        Returns:
            Dictionary with the echoed message
        """
        async def _echo() -> dict[str, Any]:
            return await client.call_tool("everything:echo", {"message": message})

        try:
            result = asyncio.get_event_loop().run_until_complete(_echo())
            return {"status": "success", "result": result}
        except Exception as e:
            return {"status": "error", "error": str(e)}

    return FunctionTool(echo_via_gateway)


# Example of creating an agent with gateway tools
def create_gateway_enhanced_agent(
    gateway_url: str = "http://localhost:3000",
    model: str | None = None,
):
    """
    Create an ADK agent enhanced with tools from agentgateway.

    This demonstrates the integration pattern where an ADK agent
    gets its tools from an agentgateway instance.

    Args:
        gateway_url: URL of the agentgateway
        model: Optional model string. If None, auto-detects from environment.

    Returns:
        An ADK Agent with gateway tools
    """
    from google.adk.agents import Agent

    from .agent import get_configured_model

    # Discover tools from gateway (sync wrapper for simplicity)
    async def _discover():
        return await discover_gateway_tools(gateway_url)

    try:
        gateway_tools = asyncio.get_event_loop().run_until_complete(_discover())
    except Exception as e:
        logger.warning(f"Could not discover gateway tools: {e}")
        gateway_tools = []

    # Add a fallback echo tool
    if not gateway_tools:
        gateway_tools = [create_gateway_echo_tool(gateway_url)]

    return Agent(
        name="gateway_enhanced_agent",
        model=model or get_configured_model(),
        description="An ADK agent with tools from agentgateway",
        instruction="""You are an assistant with access to tools provided by agentgateway.

Available capabilities depend on the MCP servers connected to the gateway.
Use the available tools to help users accomplish their tasks.

When a tool call fails, report the error and suggest alternatives.""",
        tools=gateway_tools,
    )
