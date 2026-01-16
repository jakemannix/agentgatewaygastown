"""Tests for the agentgateway client wrapper."""

import pytest
from src.client import (
    MCPRequest,
    MCPResponse,
    A2AMessage,
    A2ARequest,
    RequestMetrics,
    TestResult,
    calculate_percentile,
    aggregate_metrics,
)


class TestModels:
    """Test Pydantic models."""

    def test_mcp_request_defaults(self):
        request = MCPRequest(id=1, method="tools/list")
        assert request.jsonrpc == "2.0"
        assert request.id == 1
        assert request.method == "tools/list"
        assert request.params is None

    def test_mcp_request_with_params(self):
        request = MCPRequest(
            id="test-id",
            method="tools/call",
            params={"name": "echo", "arguments": {"msg": "hello"}},
        )
        assert request.id == "test-id"
        assert request.params == {"name": "echo", "arguments": {"msg": "hello"}}

    def test_mcp_response_success(self):
        response = MCPResponse(id=1, result={"tools": []})
        assert response.jsonrpc == "2.0"
        assert response.result == {"tools": []}
        assert response.error is None

    def test_mcp_response_error(self):
        response = MCPResponse(id=1, error={"code": -32600, "message": "Invalid request"})
        assert response.result is None
        assert response.error["code"] == -32600

    def test_a2a_message(self):
        message = A2AMessage(role="user", content="Hello!")
        assert message.role == "user"
        assert message.content == "Hello!"

    def test_a2a_request(self):
        messages = [A2AMessage(role="user", content="Hello!")]
        request = A2ARequest(messages=messages)
        assert len(request.messages) == 1
        assert request.stream is False


class TestRequestMetrics:
    """Test RequestMetrics dataclass."""

    def test_request_metrics_success(self):
        metrics = RequestMetrics(
            latency_ms=10.5,
            status_code=200,
            success=True,
        )
        assert metrics.latency_ms == 10.5
        assert metrics.status_code == 200
        assert metrics.success is True
        assert metrics.error is None

    def test_request_metrics_failure(self):
        metrics = RequestMetrics(
            latency_ms=5.0,
            status_code=500,
            success=False,
            error="Internal server error",
        )
        assert metrics.success is False
        assert metrics.error == "Internal server error"


class TestPercentileCalculation:
    """Test percentile calculations."""

    def test_percentile_empty_list(self):
        assert calculate_percentile([], 50) == 0.0

    def test_percentile_single_value(self):
        assert calculate_percentile([10.0], 50) == 10.0
        assert calculate_percentile([10.0], 99) == 10.0

    def test_percentile_multiple_values(self):
        latencies = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
        p50 = calculate_percentile(latencies, 50)
        p95 = calculate_percentile(latencies, 95)
        p99 = calculate_percentile(latencies, 99)

        # Index-based calculation: index = int(len * percentile / 100)
        # For 10 items: p50 -> index 5 -> 6.0, p95 -> index 9 -> 10.0
        assert p50 == 6.0
        assert p95 == 10.0
        assert p99 == 10.0  # Index-based, capped at len-1

    def test_percentile_unsorted_input(self):
        latencies = [10.0, 1.0, 5.0, 3.0, 8.0]
        p50 = calculate_percentile(latencies, 50)
        assert p50 == 5.0


class TestAggregateMetrics:
    """Test metrics aggregation."""

    def test_aggregate_all_success(self):
        metrics = [
            RequestMetrics(latency_ms=10.0, status_code=200, success=True),
            RequestMetrics(latency_ms=20.0, status_code=200, success=True),
            RequestMetrics(latency_ms=30.0, status_code=200, success=True),
        ]
        result = aggregate_metrics("test", metrics, duration_seconds=1.0)

        assert result.pattern == "test"
        assert result.total_requests == 3
        assert result.successful_requests == 3
        assert result.failed_requests == 0
        assert result.avg_latency_ms == 20.0
        assert result.requests_per_second == 3.0
        assert result.errors == []

    def test_aggregate_with_failures(self):
        metrics = [
            RequestMetrics(latency_ms=10.0, status_code=200, success=True),
            RequestMetrics(latency_ms=5.0, status_code=500, success=False, error="Error 1"),
            RequestMetrics(latency_ms=15.0, status_code=200, success=True),
        ]
        result = aggregate_metrics("test", metrics, duration_seconds=1.0)

        assert result.successful_requests == 2
        assert result.failed_requests == 1
        assert len(result.errors) == 1
        assert result.errors[0] == "Error 1"

    def test_aggregate_zero_duration(self):
        metrics = [
            RequestMetrics(latency_ms=10.0, status_code=200, success=True),
        ]
        result = aggregate_metrics("test", metrics, duration_seconds=0.0)

        assert result.requests_per_second == 0.0

    def test_aggregate_error_truncation(self):
        # Test that errors are truncated to 10
        metrics = [
            RequestMetrics(latency_ms=5.0, status_code=500, success=False, error=f"Error {i}")
            for i in range(15)
        ]
        result = aggregate_metrics("test", metrics, duration_seconds=1.0)

        assert len(result.errors) == 10
