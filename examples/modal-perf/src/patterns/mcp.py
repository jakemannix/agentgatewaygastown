"""MCP pattern test functions."""

from __future__ import annotations

import asyncio
from typing import Any

from ..client import (
    AgentGatewayClient,
    RequestMetrics,
    TestResult,
    aggregate_metrics,
)


async def mcp_list_tools_pattern(
    gateway_url: str,
    num_requests: int = 100,
    concurrency: int = 10,
    headers: dict[str, str] | None = None,
) -> TestResult:
    """Test pattern: MCP tools/list requests.

    This pattern tests the basic MCP functionality of listing available tools.
    It's useful for testing gateway latency and throughput for metadata requests.

    Args:
        gateway_url: The URL of the agentgateway endpoint
        num_requests: Total number of requests to send
        concurrency: Number of concurrent requests
        headers: Optional headers to include in requests

    Returns:
        Test results with metrics
    """
    metrics: list[RequestMetrics] = []
    semaphore = asyncio.Semaphore(concurrency)

    async def make_request(request_id: int) -> RequestMetrics:
        async with semaphore:
            async with AgentGatewayClient(gateway_url, headers=headers) as client:
                _, metric = await client.mcp_list_tools()
                return metric

    start_time = asyncio.get_event_loop().time()
    tasks = [make_request(i) for i in range(num_requests)]
    metrics = await asyncio.gather(*tasks)
    end_time = asyncio.get_event_loop().time()

    return aggregate_metrics(
        pattern="mcp:tools/list",
        metrics=list(metrics),
        duration_seconds=end_time - start_time,
    )


async def mcp_call_tool_pattern(
    gateway_url: str,
    tool_name: str,
    tool_arguments: dict[str, Any] | None = None,
    num_requests: int = 100,
    concurrency: int = 10,
    headers: dict[str, str] | None = None,
) -> TestResult:
    """Test pattern: MCP tools/call requests.

    This pattern tests MCP tool invocation. It's useful for testing gateway
    latency and throughput for actual tool calls.

    Args:
        gateway_url: The URL of the agentgateway endpoint
        tool_name: Name of the tool to call
        tool_arguments: Arguments to pass to the tool
        num_requests: Total number of requests to send
        concurrency: Number of concurrent requests
        headers: Optional headers to include in requests

    Returns:
        Test results with metrics
    """
    metrics: list[RequestMetrics] = []
    semaphore = asyncio.Semaphore(concurrency)

    async def make_request(request_id: int) -> RequestMetrics:
        async with semaphore:
            async with AgentGatewayClient(gateway_url, headers=headers) as client:
                _, metric = await client.mcp_call_tool(tool_name, tool_arguments)
                return metric

    start_time = asyncio.get_event_loop().time()
    tasks = [make_request(i) for i in range(num_requests)]
    metrics = await asyncio.gather(*tasks)
    end_time = asyncio.get_event_loop().time()

    return aggregate_metrics(
        pattern=f"mcp:tools/call:{tool_name}",
        metrics=list(metrics),
        duration_seconds=end_time - start_time,
    )


async def mcp_mixed_pattern(
    gateway_url: str,
    tool_name: str,
    tool_arguments: dict[str, Any] | None = None,
    num_requests: int = 100,
    concurrency: int = 10,
    list_ratio: float = 0.2,
    headers: dict[str, str] | None = None,
) -> TestResult:
    """Test pattern: Mixed MCP requests (list + call).

    This pattern simulates realistic MCP traffic with a mix of metadata
    requests and actual tool calls.

    Args:
        gateway_url: The URL of the agentgateway endpoint
        tool_name: Name of the tool to call
        tool_arguments: Arguments to pass to the tool
        num_requests: Total number of requests to send
        concurrency: Number of concurrent requests
        list_ratio: Ratio of list requests to total (0.0-1.0)
        headers: Optional headers to include in requests

    Returns:
        Test results with metrics
    """
    import random

    metrics: list[RequestMetrics] = []
    semaphore = asyncio.Semaphore(concurrency)

    async def make_request(request_id: int) -> RequestMetrics:
        async with semaphore:
            async with AgentGatewayClient(gateway_url, headers=headers) as client:
                if random.random() < list_ratio:
                    _, metric = await client.mcp_list_tools()
                else:
                    _, metric = await client.mcp_call_tool(tool_name, tool_arguments)
                return metric

    start_time = asyncio.get_event_loop().time()
    tasks = [make_request(i) for i in range(num_requests)]
    metrics = await asyncio.gather(*tasks)
    end_time = asyncio.get_event_loop().time()

    return aggregate_metrics(
        pattern="mcp:mixed",
        metrics=list(metrics),
        duration_seconds=end_time - start_time,
    )
