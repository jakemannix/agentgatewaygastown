"""A2A pattern test functions."""

from __future__ import annotations

import asyncio
from typing import Any

from ..client import (
    A2AMessage,
    AgentGatewayClient,
    RequestMetrics,
    TestResult,
    aggregate_metrics,
)


async def a2a_message_pattern(
    gateway_url: str,
    message: str = "Hello, agent!",
    num_requests: int = 100,
    concurrency: int = 10,
    stream: bool = False,
    headers: dict[str, str] | None = None,
) -> TestResult:
    """Test pattern: A2A message requests.

    This pattern tests the A2A protocol message sending. It's useful for
    testing gateway latency and throughput for agent-to-agent communication.

    Args:
        gateway_url: The URL of the agentgateway endpoint
        message: Message to send to the agent
        num_requests: Total number of requests to send
        concurrency: Number of concurrent requests
        stream: Whether to use streaming mode
        headers: Optional headers to include in requests

    Returns:
        Test results with metrics
    """
    metrics: list[RequestMetrics] = []
    semaphore = asyncio.Semaphore(concurrency)

    async def make_request() -> RequestMetrics:
        async with semaphore:
            async with AgentGatewayClient(gateway_url, headers=headers) as client:
                messages = [A2AMessage(role="user", content=message)]
                _, metric = await client.a2a_request(messages, stream=stream)
                return metric

    start_time = asyncio.get_event_loop().time()
    tasks = [make_request() for _ in range(num_requests)]
    metrics = await asyncio.gather(*tasks)
    end_time = asyncio.get_event_loop().time()

    pattern_name = "a2a:message:stream" if stream else "a2a:message"
    return aggregate_metrics(
        pattern=pattern_name,
        metrics=list(metrics),
        duration_seconds=end_time - start_time,
    )


async def a2a_agent_card_pattern(
    gateway_url: str,
    num_requests: int = 100,
    concurrency: int = 10,
    headers: dict[str, str] | None = None,
) -> TestResult:
    """Test pattern: A2A agent card requests.

    This pattern tests fetching the A2A agent card. It's useful for testing
    gateway latency for agent discovery requests.

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

    async def make_request() -> RequestMetrics:
        async with semaphore:
            async with AgentGatewayClient(gateway_url, headers=headers) as client:
                _, metric = await client.get_agent_card()
                return metric

    start_time = asyncio.get_event_loop().time()
    tasks = [make_request() for _ in range(num_requests)]
    metrics = await asyncio.gather(*tasks)
    end_time = asyncio.get_event_loop().time()

    return aggregate_metrics(
        pattern="a2a:agent_card",
        metrics=list(metrics),
        duration_seconds=end_time - start_time,
    )


async def a2a_conversation_pattern(
    gateway_url: str,
    messages: list[str] | None = None,
    num_conversations: int = 10,
    concurrency: int = 5,
    headers: dict[str, str] | None = None,
) -> TestResult:
    """Test pattern: A2A multi-turn conversations.

    This pattern simulates multi-turn conversations with an agent. It's useful
    for testing gateway behavior under realistic conversational workloads.

    Args:
        gateway_url: The URL of the agentgateway endpoint
        messages: List of messages to send in sequence (default: simple conversation)
        num_conversations: Number of conversations to run
        concurrency: Number of concurrent conversations
        headers: Optional headers to include in requests

    Returns:
        Test results with metrics
    """
    if messages is None:
        messages = [
            "Hello!",
            "What can you help me with?",
            "Tell me more about that.",
            "Thank you!",
        ]

    all_metrics: list[RequestMetrics] = []
    semaphore = asyncio.Semaphore(concurrency)

    async def run_conversation() -> list[RequestMetrics]:
        conversation_metrics: list[RequestMetrics] = []
        async with semaphore:
            async with AgentGatewayClient(gateway_url, headers=headers) as client:
                conversation_history: list[A2AMessage] = []
                for msg in messages:
                    conversation_history.append(A2AMessage(role="user", content=msg))
                    _, metric = await client.a2a_request(conversation_history)
                    conversation_metrics.append(metric)
                    # Simulate assistant response in history
                    conversation_history.append(A2AMessage(role="assistant", content="..."))
        return conversation_metrics

    start_time = asyncio.get_event_loop().time()
    tasks = [run_conversation() for _ in range(num_conversations)]
    results = await asyncio.gather(*tasks)
    for metrics_list in results:
        all_metrics.extend(metrics_list)
    end_time = asyncio.get_event_loop().time()

    return aggregate_metrics(
        pattern="a2a:conversation",
        metrics=all_metrics,
        duration_seconds=end_time - start_time,
    )
