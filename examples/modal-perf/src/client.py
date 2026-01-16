"""Agentgateway client wrapper for performance testing."""

from __future__ import annotations

import asyncio
from dataclasses import dataclass, field
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


class A2AMessage(BaseModel):
    """A2A message model."""

    role: str
    content: str


class A2ARequest(BaseModel):
    """A2A request model."""

    messages: list[A2AMessage]
    stream: bool = False


@dataclass
class RequestMetrics:
    """Metrics for a single request."""

    latency_ms: float
    status_code: int
    success: bool
    error: str | None = None


@dataclass
class TestResult:
    """Result of a test run."""

    pattern: str
    total_requests: int
    successful_requests: int
    failed_requests: int
    avg_latency_ms: float
    p50_latency_ms: float
    p95_latency_ms: float
    p99_latency_ms: float
    requests_per_second: float
    errors: list[str] = field(default_factory=list)


class AgentGatewayClient:
    """Client for interacting with agentgateway endpoints."""

    def __init__(
        self,
        base_url: str,
        timeout: float = 30.0,
        headers: dict[str, str] | None = None,
    ):
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self.headers = headers or {}
        self._client: httpx.AsyncClient | None = None

    async def __aenter__(self) -> AgentGatewayClient:
        self._client = httpx.AsyncClient(
            base_url=self.base_url,
            timeout=self.timeout,
            headers=self.headers,
        )
        return self

    async def __aexit__(self, *args: Any) -> None:
        if self._client:
            await self._client.aclose()
            self._client = None

    @property
    def client(self) -> httpx.AsyncClient:
        if self._client is None:
            raise RuntimeError("Client not initialized. Use async context manager.")
        return self._client

    async def mcp_request(
        self,
        method: str,
        params: dict[str, Any] | None = None,
        request_id: int | str = 1,
    ) -> tuple[MCPResponse, RequestMetrics]:
        """Send an MCP JSON-RPC request and return response with metrics."""
        request = MCPRequest(
            id=request_id,
            method=method,
            params=params,
        )

        start_time = asyncio.get_event_loop().time()
        try:
            response = await self.client.post(
                "/",
                json=request.model_dump(exclude_none=True),
                headers={"Content-Type": "application/json"},
            )
            end_time = asyncio.get_event_loop().time()
            latency_ms = (end_time - start_time) * 1000

            mcp_response = MCPResponse.model_validate(response.json())
            metrics = RequestMetrics(
                latency_ms=latency_ms,
                status_code=response.status_code,
                success=response.status_code == 200 and mcp_response.error is None,
                error=str(mcp_response.error) if mcp_response.error else None,
            )
            return mcp_response, metrics

        except Exception as e:
            end_time = asyncio.get_event_loop().time()
            latency_ms = (end_time - start_time) * 1000
            metrics = RequestMetrics(
                latency_ms=latency_ms,
                status_code=0,
                success=False,
                error=str(e),
            )
            return MCPResponse(id=request_id, error={"message": str(e)}), metrics

    async def mcp_list_tools(self) -> tuple[MCPResponse, RequestMetrics]:
        """List available MCP tools."""
        return await self.mcp_request("tools/list")

    async def mcp_call_tool(
        self,
        name: str,
        arguments: dict[str, Any] | None = None,
    ) -> tuple[MCPResponse, RequestMetrics]:
        """Call an MCP tool."""
        return await self.mcp_request(
            "tools/call",
            params={"name": name, "arguments": arguments or {}},
        )

    async def a2a_request(
        self,
        messages: list[A2AMessage],
        stream: bool = False,
    ) -> tuple[dict[str, Any], RequestMetrics]:
        """Send an A2A request and return response with metrics."""
        request = A2ARequest(messages=messages, stream=stream)

        start_time = asyncio.get_event_loop().time()
        try:
            response = await self.client.post(
                "/",
                json=request.model_dump(),
                headers={"Content-Type": "application/json"},
            )
            end_time = asyncio.get_event_loop().time()
            latency_ms = (end_time - start_time) * 1000

            metrics = RequestMetrics(
                latency_ms=latency_ms,
                status_code=response.status_code,
                success=response.status_code == 200,
                error=None if response.status_code == 200 else response.text,
            )
            return response.json(), metrics

        except Exception as e:
            end_time = asyncio.get_event_loop().time()
            latency_ms = (end_time - start_time) * 1000
            metrics = RequestMetrics(
                latency_ms=latency_ms,
                status_code=0,
                success=False,
                error=str(e),
            )
            return {"error": str(e)}, metrics

    async def get_agent_card(self) -> tuple[dict[str, Any], RequestMetrics]:
        """Get the A2A agent card."""
        start_time = asyncio.get_event_loop().time()
        try:
            response = await self.client.get("/.well-known/agent.json")
            end_time = asyncio.get_event_loop().time()
            latency_ms = (end_time - start_time) * 1000

            metrics = RequestMetrics(
                latency_ms=latency_ms,
                status_code=response.status_code,
                success=response.status_code == 200,
                error=None if response.status_code == 200 else response.text,
            )
            return response.json(), metrics

        except Exception as e:
            end_time = asyncio.get_event_loop().time()
            latency_ms = (end_time - start_time) * 1000
            metrics = RequestMetrics(
                latency_ms=latency_ms,
                status_code=0,
                success=False,
                error=str(e),
            )
            return {"error": str(e)}, metrics

    async def health_check(self) -> tuple[bool, RequestMetrics]:
        """Check if the gateway is healthy."""
        start_time = asyncio.get_event_loop().time()
        try:
            response = await self.client.get("/health")
            end_time = asyncio.get_event_loop().time()
            latency_ms = (end_time - start_time) * 1000

            metrics = RequestMetrics(
                latency_ms=latency_ms,
                status_code=response.status_code,
                success=response.status_code == 200,
            )
            return response.status_code == 200, metrics

        except Exception as e:
            end_time = asyncio.get_event_loop().time()
            latency_ms = (end_time - start_time) * 1000
            metrics = RequestMetrics(
                latency_ms=latency_ms,
                status_code=0,
                success=False,
                error=str(e),
            )
            return False, metrics


def calculate_percentile(latencies: list[float], percentile: float) -> float:
    """Calculate a percentile from a list of latencies."""
    if not latencies:
        return 0.0
    sorted_latencies = sorted(latencies)
    index = int(len(sorted_latencies) * percentile / 100)
    index = min(index, len(sorted_latencies) - 1)
    return sorted_latencies[index]


def aggregate_metrics(
    pattern: str,
    metrics: list[RequestMetrics],
    duration_seconds: float,
) -> TestResult:
    """Aggregate metrics from multiple requests into a test result."""
    latencies = [m.latency_ms for m in metrics]
    successful = [m for m in metrics if m.success]
    failed = [m for m in metrics if not m.success]
    errors = [m.error for m in failed if m.error]

    return TestResult(
        pattern=pattern,
        total_requests=len(metrics),
        successful_requests=len(successful),
        failed_requests=len(failed),
        avg_latency_ms=sum(latencies) / len(latencies) if latencies else 0,
        p50_latency_ms=calculate_percentile(latencies, 50),
        p95_latency_ms=calculate_percentile(latencies, 95),
        p99_latency_ms=calculate_percentile(latencies, 99),
        requests_per_second=len(metrics) / duration_seconds if duration_seconds > 0 else 0,
        errors=errors[:10],  # Keep first 10 errors
    )
