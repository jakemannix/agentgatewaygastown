"""Modal.com function definitions for agentgateway pattern testing."""

from __future__ import annotations

import asyncio
import json
import os
from dataclasses import asdict
from typing import Any

import modal

from .client import (
    A2AMessage,
    AgentGatewayClient,
    RequestMetrics,
    TestResult,
    aggregate_metrics,
)

# Create Modal app
app = modal.App("agentgateway-perf")

# Define the image with dependencies
image = modal.Image.debian_slim(python_version="3.12").pip_install(
    "httpx>=0.27.0",
    "pydantic>=2.0.0",
)


@app.function(
    image=image,
    secrets=[modal.Secret.from_name("agentgateway-perf")],
    timeout=300,
)
async def run_mcp_load_test(
    gateway_url: str,
    num_requests: int = 100,
    concurrency: int = 10,
    method: str = "tools/list",
    params: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Run an MCP load test against an agentgateway endpoint.

    Args:
        gateway_url: The URL of the agentgateway endpoint
        num_requests: Total number of requests to send
        concurrency: Number of concurrent requests
        method: MCP method to call
        params: Optional parameters for the MCP method

    Returns:
        Test results as a dictionary
    """
    # Import inside function for Modal serialization
    from src.client import (
        AgentGatewayClient,
        aggregate_metrics,
    )

    headers = {}
    auth_token = os.environ.get("AGENTGATEWAY_AUTH_TOKEN")
    if auth_token:
        headers["Authorization"] = f"Bearer {auth_token}"

    metrics: list[RequestMetrics] = []
    semaphore = asyncio.Semaphore(concurrency)

    async def make_request(request_id: int) -> RequestMetrics:
        async with semaphore:
            async with AgentGatewayClient(gateway_url, headers=headers) as client:
                _, metric = await client.mcp_request(method, params, request_id)
                return metric

    start_time = asyncio.get_event_loop().time()
    tasks = [make_request(i) for i in range(num_requests)]
    metrics = await asyncio.gather(*tasks)
    end_time = asyncio.get_event_loop().time()

    result = aggregate_metrics(
        pattern=f"mcp:{method}",
        metrics=list(metrics),
        duration_seconds=end_time - start_time,
    )
    return asdict(result)


@app.function(
    image=image,
    secrets=[modal.Secret.from_name("agentgateway-perf")],
    timeout=300,
)
async def run_a2a_load_test(
    gateway_url: str,
    num_requests: int = 100,
    concurrency: int = 10,
    message: str = "Hello, agent!",
) -> dict[str, Any]:
    """Run an A2A load test against an agentgateway endpoint.

    Args:
        gateway_url: The URL of the agentgateway endpoint
        num_requests: Total number of requests to send
        concurrency: Number of concurrent requests
        message: Message to send to the agent

    Returns:
        Test results as a dictionary
    """
    from src.client import (
        A2AMessage,
        AgentGatewayClient,
        aggregate_metrics,
    )

    headers = {}
    auth_token = os.environ.get("AGENTGATEWAY_AUTH_TOKEN")
    if auth_token:
        headers["Authorization"] = f"Bearer {auth_token}"

    metrics: list[RequestMetrics] = []
    semaphore = asyncio.Semaphore(concurrency)

    async def make_request() -> RequestMetrics:
        async with semaphore:
            async with AgentGatewayClient(gateway_url, headers=headers) as client:
                messages = [A2AMessage(role="user", content=message)]
                _, metric = await client.a2a_request(messages)
                return metric

    start_time = asyncio.get_event_loop().time()
    tasks = [make_request() for _ in range(num_requests)]
    metrics = await asyncio.gather(*tasks)
    end_time = asyncio.get_event_loop().time()

    result = aggregate_metrics(
        pattern="a2a:message",
        metrics=list(metrics),
        duration_seconds=end_time - start_time,
    )
    return asdict(result)


@app.function(
    image=image,
    secrets=[modal.Secret.from_name("agentgateway-perf")],
    timeout=60,
)
async def health_check(gateway_url: str) -> dict[str, Any]:
    """Check health of an agentgateway endpoint.

    Args:
        gateway_url: The URL of the agentgateway endpoint

    Returns:
        Health check results
    """
    from src.client import AgentGatewayClient

    headers = {}
    auth_token = os.environ.get("AGENTGATEWAY_AUTH_TOKEN")
    if auth_token:
        headers["Authorization"] = f"Bearer {auth_token}"

    async with AgentGatewayClient(gateway_url, headers=headers) as client:
        healthy, metrics = await client.health_check()
        return {
            "healthy": healthy,
            "latency_ms": metrics.latency_ms,
            "status_code": metrics.status_code,
            "error": metrics.error,
        }


@app.function(
    image=image,
    secrets=[modal.Secret.from_name("agentgateway-perf")],
    timeout=60,
)
async def get_agent_card(gateway_url: str) -> dict[str, Any]:
    """Fetch the A2A agent card from an agentgateway endpoint.

    Args:
        gateway_url: The URL of the agentgateway endpoint

    Returns:
        Agent card data with metrics
    """
    from src.client import AgentGatewayClient

    headers = {}
    auth_token = os.environ.get("AGENTGATEWAY_AUTH_TOKEN")
    if auth_token:
        headers["Authorization"] = f"Bearer {auth_token}"

    async with AgentGatewayClient(gateway_url, headers=headers) as client:
        card, metrics = await client.get_agent_card()
        return {
            "agent_card": card,
            "latency_ms": metrics.latency_ms,
            "status_code": metrics.status_code,
            "success": metrics.success,
        }


@app.function(
    image=image,
    secrets=[modal.Secret.from_name("agentgateway-perf")],
    timeout=600,
)
async def run_pattern_suite(
    gateway_url: str,
    patterns: list[str] | None = None,
    num_requests: int = 100,
    concurrency: int = 10,
) -> dict[str, Any]:
    """Run a suite of pattern tests against an agentgateway endpoint.

    Args:
        gateway_url: The URL of the agentgateway endpoint
        patterns: List of patterns to test (default: all)
        num_requests: Number of requests per pattern
        concurrency: Number of concurrent requests

    Returns:
        Combined results from all pattern tests
    """
    available_patterns = ["mcp:tools/list", "mcp:tools/call", "a2a:message", "a2a:agent_card"]
    if patterns is None:
        patterns = available_patterns

    results: dict[str, Any] = {
        "gateway_url": gateway_url,
        "patterns": {},
        "summary": {},
    }

    for pattern in patterns:
        if pattern == "mcp:tools/list":
            result = await run_mcp_load_test.remote.aio(
                gateway_url=gateway_url,
                num_requests=num_requests,
                concurrency=concurrency,
                method="tools/list",
            )
        elif pattern == "mcp:tools/call":
            result = await run_mcp_load_test.remote.aio(
                gateway_url=gateway_url,
                num_requests=num_requests,
                concurrency=concurrency,
                method="tools/call",
                params={"name": "echo", "arguments": {"message": "test"}},
            )
        elif pattern == "a2a:message":
            result = await run_a2a_load_test.remote.aio(
                gateway_url=gateway_url,
                num_requests=num_requests,
                concurrency=concurrency,
            )
        elif pattern == "a2a:agent_card":
            card_result = await get_agent_card.remote.aio(gateway_url=gateway_url)
            result = {
                "pattern": "a2a:agent_card",
                "total_requests": 1,
                "successful_requests": 1 if card_result["success"] else 0,
                "failed_requests": 0 if card_result["success"] else 1,
                "avg_latency_ms": card_result["latency_ms"],
                "p50_latency_ms": card_result["latency_ms"],
                "p95_latency_ms": card_result["latency_ms"],
                "p99_latency_ms": card_result["latency_ms"],
                "requests_per_second": 0,
                "errors": [],
            }
        else:
            continue

        results["patterns"][pattern] = result

    # Calculate summary
    total_requests = sum(r.get("total_requests", 0) for r in results["patterns"].values())
    successful = sum(r.get("successful_requests", 0) for r in results["patterns"].values())
    avg_latencies = [r.get("avg_latency_ms", 0) for r in results["patterns"].values() if r.get("avg_latency_ms")]

    results["summary"] = {
        "total_patterns": len(results["patterns"]),
        "total_requests": total_requests,
        "successful_requests": successful,
        "success_rate": successful / total_requests if total_requests > 0 else 0,
        "overall_avg_latency_ms": sum(avg_latencies) / len(avg_latencies) if avg_latencies else 0,
    }

    return results


@app.local_entrypoint()
def main(
    gateway_url: str = "http://localhost:3000",
    pattern: str = "all",
    num_requests: int = 100,
    concurrency: int = 10,
):
    """Run agentgateway performance tests.

    Args:
        gateway_url: The URL of the agentgateway endpoint
        pattern: Pattern to test (mcp, a2a, or all)
        num_requests: Number of requests to send
        concurrency: Number of concurrent requests
    """
    import json

    if pattern == "all":
        result = run_pattern_suite.remote(
            gateway_url=gateway_url,
            num_requests=num_requests,
            concurrency=concurrency,
        )
    elif pattern == "mcp":
        result = run_mcp_load_test.remote(
            gateway_url=gateway_url,
            num_requests=num_requests,
            concurrency=concurrency,
        )
    elif pattern == "a2a":
        result = run_a2a_load_test.remote(
            gateway_url=gateway_url,
            num_requests=num_requests,
            concurrency=concurrency,
        )
    else:
        print(f"Unknown pattern: {pattern}")
        return

    print(json.dumps(result, indent=2))
