"""MCP client for connecting to agentgateway endpoints."""

from __future__ import annotations

import asyncio
from dataclasses import dataclass
from typing import Any

import httpx
from pydantic import BaseModel


class MCPRequest(BaseModel):
    """MCP JSON-RPC request model."""

    jsonrpc: str = "2.0"
    id: int | str
    method: str
    params: dict[str, Any] | None = None


class MCPResponse(BaseModel):
    """MCP JSON-RPC response model."""

    jsonrpc: str = "2.0"
    id: int | str
    result: dict[str, Any] | None = None
    error: dict[str, Any] | None = None


@dataclass
class ToolInfo:
    """Information about an MCP tool."""

    name: str
    description: str
    input_schema: dict[str, Any]


class MCPClient:
    """Client for interacting with agentgateway MCP endpoints.

    Supports both synchronous and asynchronous usage patterns.
    """

    def __init__(
        self,
        base_url: str,
        timeout: float = 30.0,
        headers: dict[str, str] | None = None,
    ):
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self.headers = headers or {}
        self._async_client: httpx.AsyncClient | None = None
        self._sync_client: httpx.Client | None = None
        self._request_id = 0
        self._tools_cache: list[ToolInfo] | None = None

    def _next_request_id(self) -> int:
        self._request_id += 1
        return self._request_id

    async def __aenter__(self) -> MCPClient:
        self._async_client = httpx.AsyncClient(
            base_url=self.base_url,
            timeout=self.timeout,
            headers=self.headers,
        )
        return self

    async def __aexit__(self, *args: Any) -> None:
        if self._async_client:
            await self._async_client.aclose()
            self._async_client = None

    def __enter__(self) -> MCPClient:
        self._sync_client = httpx.Client(
            base_url=self.base_url,
            timeout=self.timeout,
            headers=self.headers,
        )
        return self

    def __exit__(self, *args: Any) -> None:
        if self._sync_client:
            self._sync_client.close()
            self._sync_client = None

    def _make_request_sync(
        self,
        method: str,
        params: dict[str, Any] | None = None,
    ) -> MCPResponse:
        """Send a synchronous MCP request."""
        if self._sync_client is None:
            raise RuntimeError("Client not initialized. Use context manager.")

        request = MCPRequest(
            id=self._next_request_id(),
            method=method,
            params=params,
        )

        response = self._sync_client.post(
            "/mcp",
            json=request.model_dump(exclude_none=True),
            headers={"Content-Type": "application/json"},
        )
        response.raise_for_status()
        return MCPResponse.model_validate(response.json())

    async def _make_request_async(
        self,
        method: str,
        params: dict[str, Any] | None = None,
    ) -> MCPResponse:
        """Send an asynchronous MCP request."""
        if self._async_client is None:
            raise RuntimeError("Client not initialized. Use async context manager.")

        request = MCPRequest(
            id=self._next_request_id(),
            method=method,
            params=params,
        )

        response = await self._async_client.post(
            "/mcp",
            json=request.model_dump(exclude_none=True),
            headers={"Content-Type": "application/json"},
        )
        response.raise_for_status()
        return MCPResponse.model_validate(response.json())

    def list_tools(self) -> list[ToolInfo]:
        """List available MCP tools (synchronous)."""
        response = self._make_request_sync("tools/list")
        if response.error:
            raise RuntimeError(f"MCP error: {response.error}")

        tools = []
        for tool in response.result.get("tools", []):
            tools.append(
                ToolInfo(
                    name=tool["name"],
                    description=tool.get("description", ""),
                    input_schema=tool.get("inputSchema", {}),
                )
            )
        self._tools_cache = tools
        return tools

    async def list_tools_async(self) -> list[ToolInfo]:
        """List available MCP tools (asynchronous)."""
        response = await self._make_request_async("tools/list")
        if response.error:
            raise RuntimeError(f"MCP error: {response.error}")

        tools = []
        for tool in response.result.get("tools", []):
            tools.append(
                ToolInfo(
                    name=tool["name"],
                    description=tool.get("description", ""),
                    input_schema=tool.get("inputSchema", {}),
                )
            )
        self._tools_cache = tools
        return tools

    def call_tool(self, name: str, arguments: dict[str, Any] | None = None) -> Any:
        """Call an MCP tool (synchronous)."""
        response = self._make_request_sync(
            "tools/call",
            params={"name": name, "arguments": arguments or {}},
        )
        if response.error:
            raise RuntimeError(f"MCP error calling {name}: {response.error}")

        # Extract content from MCP tool response
        result = response.result or {}
        content = result.get("content", [])
        if content and isinstance(content, list) and len(content) > 0:
            first_content = content[0]
            if first_content.get("type") == "text":
                return first_content.get("text", "")
        return result

    async def call_tool_async(
        self, name: str, arguments: dict[str, Any] | None = None
    ) -> Any:
        """Call an MCP tool (asynchronous)."""
        response = await self._make_request_async(
            "tools/call",
            params={"name": name, "arguments": arguments or {}},
        )
        if response.error:
            raise RuntimeError(f"MCP error calling {name}: {response.error}")

        # Extract content from MCP tool response
        result = response.result or {}
        content = result.get("content", [])
        if content and isinstance(content, list) and len(content) > 0:
            first_content = content[0]
            if first_content.get("type") == "text":
                return first_content.get("text", "")
        return result

    def get_cached_tools(self) -> list[ToolInfo] | None:
        """Get cached tools list if available."""
        return self._tools_cache
