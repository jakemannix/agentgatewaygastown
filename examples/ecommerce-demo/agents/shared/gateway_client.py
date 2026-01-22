"""MCP Gateway client for connecting agents to backend services.

This module provides:
- GatewayMCPClient: Async client for MCP communication with AgentGateway
- Dynamic tool discovery and creation for ADK agents
"""

import asyncio
import json
import logging
from typing import Any, Callable, Optional

import httpx

logger = logging.getLogger(__name__)


def _parse_sse_response(text: str) -> dict[str, Any]:
    """Parse a Server-Sent Events response to extract JSON data.

    SSE format: data: {"jsonrpc": "2.0", ...}
    """
    for line in text.strip().split("\n"):
        if line.startswith("data: "):
            json_str = line[6:]  # Skip "data: " prefix
            return json.loads(json_str)
    # If no SSE data line found, try parsing as plain JSON
    return json.loads(text)


class GatewayMCPClient:
    """Async client for MCP communication with AgentGateway."""

    def __init__(
        self,
        gateway_url: str = "http://localhost:3000",
        agent_name: str = "ecommerce-agent",
        agent_version: str = "1.0.0",
        timeout: float = 30.0,
    ):
        self.gateway_url = gateway_url.rstrip("/")
        self.agent_name = agent_name
        self.agent_version = agent_version
        self.timeout = timeout
        self._request_id = 0
        self._session_id: Optional[str] = None
        self._initialized = False

    def _next_request_id(self) -> int:
        self._request_id += 1
        return self._request_id

    def _get_headers(self) -> dict[str, str]:
        """Get headers including agent identity and session."""
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
            "X-Agent-Name": self.agent_name,
            "X-Agent-Version": self.agent_version,
        }
        if self._session_id:
            headers["Mcp-Session-Id"] = self._session_id
        return headers

    async def _ensure_initialized(self) -> None:
        """Ensure the MCP session is initialized."""
        if self._initialized:
            return

        request = {
            "jsonrpc": "2.0",
            "id": self._next_request_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": self.agent_name,
                    "version": self.agent_version,
                },
            },
        }

        async with httpx.AsyncClient(timeout=self.timeout) as client:
            response = await client.post(
                f"{self.gateway_url}/mcp",
                json=request,
                headers=self._get_headers(),
            )
            response.raise_for_status()

            # Extract session ID from response header
            session_id = response.headers.get("mcp-session-id")
            if session_id:
                self._session_id = session_id
                logger.info(f"MCP session initialized: {session_id[:8]}...")

            # Parse SSE response to verify initialization succeeded
            result = _parse_sse_response(response.text)
            if "error" in result:
                raise Exception(f"MCP init error: {result['error']}")

            self._initialized = True

    async def _send_jsonrpc(
        self, method: str, params: Optional[dict] = None
    ) -> dict[str, Any]:
        """Send a JSON-RPC request to the gateway."""
        # Ensure session is initialized first
        await self._ensure_initialized()

        request = {
            "jsonrpc": "2.0",
            "id": self._next_request_id(),
            "method": method,
        }
        if params:
            request["params"] = params

        async with httpx.AsyncClient(timeout=self.timeout) as client:
            try:
                response = await client.post(
                    f"{self.gateway_url}/mcp",
                    json=request,
                    headers=self._get_headers(),
                )
                response.raise_for_status()

                # Parse SSE response format
                result = _parse_sse_response(response.text)

                if "error" in result:
                    error = result["error"]
                    raise Exception(f"MCP error {error.get('code')}: {error.get('message')}")

                return result.get("result", {})

            except httpx.HTTPError as e:
                logger.error(f"HTTP error communicating with gateway: {e}")
                raise
            except Exception as e:
                logger.error(f"Error in MCP request: {e}")
                raise

    async def list_tools(self) -> list[dict]:
        """List available tools from the gateway."""
        result = await self._send_jsonrpc("tools/list")
        return result.get("tools", [])

    async def call_tool(self, name: str, arguments: Optional[dict] = None) -> Any:
        """Call a tool through the gateway."""
        logger.info(f"Calling tool: {name} with args: {arguments}")
        params = {"name": name}
        if arguments:
            params["arguments"] = arguments

        result = await self._send_jsonrpc("tools/call", params)
        logger.debug(f"Tool {name} raw result: {str(result)[:200]}...")

        # Extract content from MCP response
        content = result.get("content", [])
        if content and len(content) > 0:
            first_content = content[0]
            if first_content.get("type") == "text":
                try:
                    parsed = json.loads(first_content.get("text", "{}"))
                    logger.debug(f"Tool {name} parsed result: {str(parsed)[:200]}...")
                    return parsed
                except json.JSONDecodeError:
                    return first_content.get("text")
        return result


# Convenience functions for simpler usage
async def discover_tools(
    gateway_url: str = "http://localhost:3000",
    agent_name: str = "ecommerce-agent",
) -> list[dict]:
    """Discover available tools from the gateway."""
    client = GatewayMCPClient(gateway_url=gateway_url, agent_name=agent_name)
    return await client.list_tools()


async def call_tool(
    name: str,
    arguments: Optional[dict] = None,
    gateway_url: str = "http://localhost:3000",
    agent_name: str = "ecommerce-agent",
) -> Any:
    """Call a tool through the gateway."""
    client = GatewayMCPClient(gateway_url=gateway_url, agent_name=agent_name)
    return await client.call_tool(name, arguments)


def create_tool_caller(client: GatewayMCPClient, tool_name: str) -> Callable[..., Any]:
    """Create a callable function that invokes an MCP tool through the gateway.

    Args:
        client: The gateway client to use for tool calls
        tool_name: The MCP tool name to call

    Returns:
        A function that accepts **kwargs and calls the tool
    """
    import concurrent.futures

    def tool_fn(**kwargs) -> Any:
        """Dynamically generated tool function."""
        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            # No running loop - safe to use asyncio.run()
            return asyncio.run(client.call_tool(tool_name, kwargs if kwargs else None))

        # There's a running loop - run in a separate thread
        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as executor:
            future = executor.submit(
                asyncio.run, client.call_tool(tool_name, kwargs if kwargs else None)
            )
            return future.result()

    return tool_fn


def create_adk_tools(
    client: GatewayMCPClient,
    mcp_tools: list[dict],
    allowed_tools: Optional[set[str]] = None,
) -> list:
    """Create ADK FunctionTools from MCP tool definitions.

    Args:
        client: The gateway client to use for tool calls
        mcp_tools: List of MCP tool definitions from tools/list
        allowed_tools: Optional set of tool names to include. If None, includes all tools.

    Returns:
        List of ADK FunctionTool objects
    """
    from google.adk.tools import FunctionTool

    adk_tools = []

    for tool_def in mcp_tools:
        tool_name = tool_def.get("name", "")

        # Skip if not in allowed list
        if allowed_tools is not None and tool_name not in allowed_tools:
            continue

        description = tool_def.get("description", f"Call the {tool_name} tool")
        input_schema = tool_def.get("inputSchema", {})

        # Create the callable
        tool_fn = create_tool_caller(client, tool_name)

        # Set function metadata for ADK
        tool_fn.__name__ = tool_name.replace("-", "_").replace("/", "_")
        tool_fn.__doc__ = description

        # Build parameter documentation from schema
        properties = input_schema.get("properties", {})
        required = set(input_schema.get("required", []))

        if properties:
            param_docs = []
            for param_name, param_def in properties.items():
                param_type = param_def.get("type", "any")
                param_desc = param_def.get("description", "")
                req_marker = " (required)" if param_name in required else ""
                param_docs.append(f"    {param_name}: {param_type}{req_marker} - {param_desc}")

            if param_docs:
                tool_fn.__doc__ = f"{description}\n\nArgs:\n" + "\n".join(param_docs)

        # Create ADK FunctionTool
        try:
            adk_tool = FunctionTool(tool_fn)
            adk_tools.append(adk_tool)
            logger.debug(f"Created ADK tool: {tool_name}")
        except Exception as e:
            logger.warning(f"Failed to create ADK tool for {tool_name}: {e}")

    return adk_tools


async def discover_and_create_tools(
    gateway_url: str = "http://localhost:3000",
    agent_name: str = "ecommerce-agent",
    allowed_tools: Optional[set[str]] = None,
) -> tuple[GatewayMCPClient, list]:
    """Discover tools from gateway and create ADK FunctionTools.

    Args:
        gateway_url: URL of the MCP gateway
        agent_name: Name to identify this agent
        allowed_tools: Optional set of tool names to include

    Returns:
        Tuple of (client, list of ADK FunctionTools)
    """
    client = GatewayMCPClient(gateway_url=gateway_url, agent_name=agent_name)
    mcp_tools = await client.list_tools()

    logger.info(f"Discovered {len(mcp_tools)} tools from gateway")
    for tool in mcp_tools:
        logger.info(f"  - {tool.get('name')}: {tool.get('description', '')[:50]}...")

    adk_tools = create_adk_tools(client, mcp_tools, allowed_tools)

    if allowed_tools:
        logger.info(f"Created {len(adk_tools)} ADK tools (filtered from {len(mcp_tools)})")
    else:
        logger.info(f"Created {len(adk_tools)} ADK tools")

    return client, adk_tools


def create_langchain_tools(
    client: GatewayMCPClient,
    mcp_tools: list[dict],
    allowed_tools: Optional[set[str]] = None,
) -> list:
    """Create LangChain StructuredTools from MCP tool definitions.

    Args:
        client: The gateway client to use for tool calls
        mcp_tools: List of MCP tool definitions from tools/list
        allowed_tools: Optional set of tool names to include. If None, includes all tools.

    Returns:
        List of LangChain StructuredTool objects
    """
    from langchain_core.tools import StructuredTool

    langchain_tools = []

    for tool_def in mcp_tools:
        tool_name = tool_def.get("name", "")

        # Skip if not in allowed list
        if allowed_tools is not None and tool_name not in allowed_tools:
            continue

        description = tool_def.get("description", f"Call the {tool_name} tool")
        input_schema = tool_def.get("inputSchema", {})

        # Create the callable
        tool_fn = create_tool_caller(client, tool_name)

        # Create args schema from MCP inputSchema for LangChain
        # LangChain uses Pydantic models, but we can pass the JSON schema
        properties = input_schema.get("properties", {})
        required = input_schema.get("required", [])

        # Build parameter documentation
        param_docs = []
        for param_name, param_def in properties.items():
            param_type = param_def.get("type", "any")
            param_desc = param_def.get("description", "")
            req_marker = " (required)" if param_name in required else ""
            param_docs.append(f"    {param_name}: {param_type}{req_marker} - {param_desc}")

        full_description = description
        if param_docs:
            full_description = f"{description}\n\nArgs:\n" + "\n".join(param_docs)

        # Create LangChain StructuredTool
        try:
            lc_tool = StructuredTool.from_function(
                func=tool_fn,
                name=tool_name.replace("-", "_").replace("/", "_"),
                description=full_description,
            )
            langchain_tools.append(lc_tool)
            logger.debug(f"Created LangChain tool: {tool_name}")
        except Exception as e:
            logger.warning(f"Failed to create LangChain tool for {tool_name}: {e}")

    return langchain_tools


async def discover_and_create_langchain_tools(
    gateway_url: str = "http://localhost:3000",
    agent_name: str = "ecommerce-agent",
    allowed_tools: Optional[set[str]] = None,
) -> tuple[GatewayMCPClient, list]:
    """Discover tools from gateway and create LangChain StructuredTools.

    Args:
        gateway_url: URL of the MCP gateway
        agent_name: Name to identify this agent
        allowed_tools: Optional set of tool names to include

    Returns:
        Tuple of (client, list of LangChain StructuredTools)
    """
    client = GatewayMCPClient(gateway_url=gateway_url, agent_name=agent_name)
    mcp_tools = await client.list_tools()

    logger.info(f"Discovered {len(mcp_tools)} tools from gateway")
    for tool in mcp_tools:
        logger.info(f"  - {tool.get('name')}: {tool.get('description', '')[:50]}...")

    langchain_tools = create_langchain_tools(client, mcp_tools, allowed_tools)

    if allowed_tools:
        logger.info(f"Created {len(langchain_tools)} LangChain tools (filtered from {len(mcp_tools)})")
    else:
        logger.info(f"Created {len(langchain_tools)} LangChain tools")

    return client, langchain_tools
