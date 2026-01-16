"""Main entry point for modal-perf CLI."""

from __future__ import annotations

import argparse
import asyncio
import json
import sys
from dataclasses import asdict


def main():
    """Run agentgateway performance tests locally or via Modal."""
    parser = argparse.ArgumentParser(
        description="Agentgateway performance testing with Modal.com"
    )
    parser.add_argument(
        "--gateway-url",
        "-u",
        default="http://localhost:3000",
        help="URL of the agentgateway endpoint",
    )
    parser.add_argument(
        "--pattern",
        "-p",
        choices=["mcp", "a2a", "all", "health"],
        default="all",
        help="Pattern to test",
    )
    parser.add_argument(
        "--num-requests",
        "-n",
        type=int,
        default=100,
        help="Number of requests to send",
    )
    parser.add_argument(
        "--concurrency",
        "-c",
        type=int,
        default=10,
        help="Number of concurrent requests",
    )
    parser.add_argument(
        "--local",
        "-l",
        action="store_true",
        help="Run locally instead of on Modal",
    )
    parser.add_argument(
        "--auth-token",
        "-t",
        help="Authentication token for agentgateway",
    )
    parser.add_argument(
        "--output",
        "-o",
        choices=["json", "text"],
        default="text",
        help="Output format",
    )

    args = parser.parse_args()

    headers = {}
    if args.auth_token:
        headers["Authorization"] = f"Bearer {args.auth_token}"

    if args.local:
        result = asyncio.run(run_local_tests(
            gateway_url=args.gateway_url,
            pattern=args.pattern,
            num_requests=args.num_requests,
            concurrency=args.concurrency,
            headers=headers,
        ))
    else:
        result = run_modal_tests(
            gateway_url=args.gateway_url,
            pattern=args.pattern,
            num_requests=args.num_requests,
            concurrency=args.concurrency,
        )

    if args.output == "json":
        print(json.dumps(result, indent=2))
    else:
        print_result(result)


async def run_local_tests(
    gateway_url: str,
    pattern: str,
    num_requests: int,
    concurrency: int,
    headers: dict[str, str] | None = None,
) -> dict:
    """Run tests locally without Modal."""
    from .client import AgentGatewayClient
    from .patterns.mcp import mcp_list_tools_pattern, mcp_call_tool_pattern
    from .patterns.a2a import a2a_message_pattern, a2a_agent_card_pattern

    results = {"gateway_url": gateway_url, "patterns": {}}

    if pattern == "health":
        async with AgentGatewayClient(gateway_url, headers=headers) as client:
            healthy, metrics = await client.health_check()
            return {
                "healthy": healthy,
                "latency_ms": metrics.latency_ms,
                "status_code": metrics.status_code,
                "error": metrics.error,
            }

    if pattern in ("mcp", "all"):
        result = await mcp_list_tools_pattern(
            gateway_url=gateway_url,
            num_requests=num_requests,
            concurrency=concurrency,
            headers=headers,
        )
        results["patterns"]["mcp:tools/list"] = asdict(result)

    if pattern in ("a2a", "all"):
        result = await a2a_message_pattern(
            gateway_url=gateway_url,
            num_requests=num_requests,
            concurrency=concurrency,
            headers=headers,
        )
        results["patterns"]["a2a:message"] = asdict(result)

        result = await a2a_agent_card_pattern(
            gateway_url=gateway_url,
            num_requests=num_requests,
            concurrency=concurrency,
            headers=headers,
        )
        results["patterns"]["a2a:agent_card"] = asdict(result)

    # Calculate summary
    if results["patterns"]:
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


def run_modal_tests(
    gateway_url: str,
    pattern: str,
    num_requests: int,
    concurrency: int,
) -> dict:
    """Run tests on Modal.com."""
    try:
        from .modal_app import (
            run_mcp_load_test,
            run_a2a_load_test,
            run_pattern_suite,
            health_check,
        )
    except ImportError:
        print("Error: Modal is not installed. Install with: pip install modal")
        print("Or run locally with: --local flag")
        sys.exit(1)

    if pattern == "health":
        return health_check.remote(gateway_url=gateway_url)
    elif pattern == "all":
        return run_pattern_suite.remote(
            gateway_url=gateway_url,
            num_requests=num_requests,
            concurrency=concurrency,
        )
    elif pattern == "mcp":
        return run_mcp_load_test.remote(
            gateway_url=gateway_url,
            num_requests=num_requests,
            concurrency=concurrency,
        )
    elif pattern == "a2a":
        return run_a2a_load_test.remote(
            gateway_url=gateway_url,
            num_requests=num_requests,
            concurrency=concurrency,
        )
    else:
        return {"error": f"Unknown pattern: {pattern}"}


def print_result(result: dict):
    """Print results in human-readable format."""
    if "healthy" in result:
        # Health check result
        status = "HEALTHY" if result["healthy"] else "UNHEALTHY"
        print(f"Status: {status}")
        print(f"Latency: {result['latency_ms']:.2f}ms")
        if result.get("error"):
            print(f"Error: {result['error']}")
        return

    print(f"Gateway: {result.get('gateway_url', 'N/A')}")
    print()

    if "patterns" in result:
        for pattern_name, pattern_result in result["patterns"].items():
            print(f"Pattern: {pattern_name}")
            print(f"  Total requests: {pattern_result.get('total_requests', 0)}")
            print(f"  Successful: {pattern_result.get('successful_requests', 0)}")
            print(f"  Failed: {pattern_result.get('failed_requests', 0)}")
            print(f"  Avg latency: {pattern_result.get('avg_latency_ms', 0):.2f}ms")
            print(f"  P50 latency: {pattern_result.get('p50_latency_ms', 0):.2f}ms")
            print(f"  P95 latency: {pattern_result.get('p95_latency_ms', 0):.2f}ms")
            print(f"  P99 latency: {pattern_result.get('p99_latency_ms', 0):.2f}ms")
            print(f"  RPS: {pattern_result.get('requests_per_second', 0):.2f}")
            if pattern_result.get("errors"):
                print(f"  Errors: {pattern_result['errors'][:3]}")
            print()

    if "summary" in result:
        summary = result["summary"]
        print("Summary:")
        print(f"  Patterns tested: {summary.get('total_patterns', 0)}")
        print(f"  Total requests: {summary.get('total_requests', 0)}")
        print(f"  Success rate: {summary.get('success_rate', 0) * 100:.1f}%")
        print(f"  Avg latency: {summary.get('overall_avg_latency_ms', 0):.2f}ms")


if __name__ == "__main__":
    main()
