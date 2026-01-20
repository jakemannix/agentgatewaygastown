"""MCP Gateway client for connecting agents to backend services."""

import asyncio
import json
import logging
from typing import Any, Optional

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
        logger.debug(f"Calling tool: {name} with args: {arguments}")
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
