#!/usr/bin/env python3
"""Integration tests for ecommerce demo.

These tests verify that:
1. Gateway is running and tools are discoverable
2. Basic passthrough tools work (find_products, stock_status)
3. Composition tools work (personalized_search, product_with_availability)
4. Agent /chat endpoints respond correctly

Prerequisites:
- Gateway running at http://localhost:3000
- MCP services running (catalog, cart, inventory, supplier, order)
- Optionally: Agents running for full agent tests

Usage:
    # Run all tests (requires gateway + services)
    python test_integration.py

    # Run only gateway tests (no agents needed)
    python test_integration.py --gateway-only

    # Run against different gateway URL
    python test_integration.py --gateway-url http://localhost:3001

    # Verbose output
    python test_integration.py -v
"""

import argparse
import asyncio
import json
import sys
import os
from dataclasses import dataclass
from typing import Optional

import httpx

# Add the demo directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from agents.shared.gateway_client import GatewayMCPClient


@dataclass
class TestResult:
    """Result of a single test."""

    name: str
    passed: bool
    message: str
    duration_ms: float = 0.0


class IntegrationTester:
    """Integration test runner for ecommerce demo."""

    def __init__(
        self,
        gateway_url: str = "http://localhost:3000",
        customer_agent_url: str = "http://localhost:9001",
        merchandiser_agent_url: str = "http://localhost:9002",
        verbose: bool = False,
    ):
        self.gateway_url = gateway_url
        self.customer_agent_url = customer_agent_url
        self.merchandiser_agent_url = merchandiser_agent_url
        self.verbose = verbose
        self.results: list[TestResult] = []
        self.client: Optional[GatewayMCPClient] = None

    def log(self, msg: str):
        """Print message if verbose mode."""
        if self.verbose:
            print(f"  {msg}")

    async def setup(self):
        """Initialize the MCP client."""
        self.client = GatewayMCPClient(
            gateway_url=self.gateway_url, agent_name="integration-tester"
        )

    async def run_test(self, name: str, test_fn):
        """Run a single test and record result."""
        import time

        start = time.time()
        try:
            await test_fn()
            duration = (time.time() - start) * 1000
            result = TestResult(name=name, passed=True, message="OK", duration_ms=duration)
        except AssertionError as e:
            duration = (time.time() - start) * 1000
            result = TestResult(
                name=name, passed=False, message=f"ASSERTION: {e}", duration_ms=duration
            )
        except Exception as e:
            duration = (time.time() - start) * 1000
            result = TestResult(
                name=name, passed=False, message=f"ERROR: {e}", duration_ms=duration
            )

        self.results.append(result)

        status = "✓" if result.passed else "✗"
        print(f"  {status} {name} ({result.duration_ms:.0f}ms)")
        if not result.passed:
            print(f"      {result.message}")

    # ==================== Gateway Tests ====================

    async def test_gateway_health(self):
        """Test that gateway is reachable."""
        async with httpx.AsyncClient(timeout=5.0) as client:
            # Gateway should respond to MCP initialize
            response = await client.post(
                f"{self.gateway_url}/mcp",
                json={
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "clientInfo": {"name": "test", "version": "1.0"},
                    },
                },
                headers={"Content-Type": "application/json"},
            )
            assert response.status_code == 200, f"Gateway returned {response.status_code}"

    async def test_tool_discovery(self):
        """Test that tools are discoverable via tools/list."""
        tools = await self.client.list_tools()
        assert len(tools) > 0, "No tools found"

        # Check for expected tools
        tool_names = {t["name"] for t in tools}

        # Basic passthrough tools
        expected_basic = {"find_products", "my_cart", "add_item", "stock_status"}
        missing_basic = expected_basic - tool_names
        assert not missing_basic, f"Missing basic tools: {missing_basic}"

        # Composition tools
        expected_compositions = {"personalized_search", "product_with_availability"}
        missing_compositions = expected_compositions - tool_names
        assert not missing_compositions, f"Missing composition tools: {missing_compositions}"

        self.log(f"Found {len(tools)} tools including {len(expected_compositions)} compositions")

    async def test_basic_passthrough_tool(self):
        """Test a basic passthrough tool (find_products)."""
        result = await self.client.call_tool("find_products", {"query": "headphones"})

        # Result should have products array
        assert "products" in result or "error" not in result, f"Unexpected result: {result}"

        # If we get products, verify structure
        if "products" in result:
            self.log(f"Found {len(result['products'])} products")
            if result["products"]:
                product = result["products"][0]
                assert "id" in product, "Product missing 'id' field"
                assert "name" in product, "Product missing 'name' field"

    async def test_stock_status_tool(self):
        """Test the stock_status passthrough tool."""
        result = await self.client.call_tool("stock_status", {})

        # Should have summary fields from output transform
        assert (
            "total_products" in result or "summary" in result
        ), f"Unexpected result: {result}"
        self.log(f"Stock status: {json.dumps(result)[:100]}...")

    async def test_search_products_raw(self):
        """Test raw search_products tool (without projection)."""
        result = await self.client.call_tool(
            "search_products", {"query": "shoes", "limit": 3}
        )

        # Raw tool should return full result
        assert "products" in result or "results" in result, f"Unexpected result: {result}"
        self.log(f"Raw search returned: {json.dumps(result)[:100]}...")

    async def test_composition_pipeline(self):
        """Test a pipeline composition (personalized_search)."""
        result = await self.client.call_tool(
            "personalized_search", {"query": "laptop", "user_id": "test-user"}
        )

        # Pipeline should return results (may be empty if no products match)
        self.log(f"Pipeline result type: {type(result)}")
        self.log(f"Pipeline result: {json.dumps(result)[:200]}...")

        # The pipeline may return products or may propagate the final step's output
        # Just verify we get a dict response without errors
        assert isinstance(result, dict), f"Expected dict, got {type(result)}"

    async def test_composition_parallel_pipeline(self):
        """Test product_with_availability (parallel steps from different services)."""
        # First get a product ID
        products = await self.client.call_tool("browse_products", {"limit": 1})
        if "products" in products and products["products"]:
            product_id = products["products"][0]["id"]
        else:
            # Use a known test product ID
            product_id = "PROD-001"

        result = await self.client.call_tool(
            "product_with_availability", {"product_id": product_id}
        )

        self.log(f"Product+availability result: {json.dumps(result)[:200]}...")
        assert isinstance(result, dict), f"Expected dict, got {type(result)}"

    async def test_cart_operations(self):
        """Test cart operation sequence: view -> add -> view."""
        # View empty cart
        cart = await self.client.call_tool("my_cart", {})
        self.log(f"Initial cart: {json.dumps(cart)[:100]}...")

        # Get a product to add
        products = await self.client.call_tool("find_products", {"query": "test"})
        if "products" in products and products["products"]:
            product_id = products["products"][0]["id"]

            # Add to cart
            add_result = await self.client.call_tool(
                "add_item", {"product_id": product_id, "quantity": 1}
            )
            self.log(f"Add to cart result: {json.dumps(add_result)[:100]}...")

            # View cart again
            cart_after = await self.client.call_tool("my_cart", {})
            self.log(f"Cart after add: {json.dumps(cart_after)[:100]}...")

    # ==================== Agent Tests ====================

    async def test_customer_agent_health(self):
        """Test that customer agent is reachable."""
        async with httpx.AsyncClient(timeout=5.0) as client:
            response = await client.get(
                f"{self.customer_agent_url}/.well-known/agent.json"
            )
            assert response.status_code == 200, f"Agent returned {response.status_code}"

            card = response.json()
            assert "name" in card, "Agent card missing 'name'"
            self.log(f"Customer agent: {card.get('name')}")

    async def test_merchandiser_agent_health(self):
        """Test that merchandiser agent is reachable."""
        async with httpx.AsyncClient(timeout=5.0) as client:
            response = await client.get(
                f"{self.merchandiser_agent_url}/.well-known/agent.json"
            )
            assert response.status_code == 200, f"Agent returned {response.status_code}"

            card = response.json()
            assert "name" in card, "Agent card missing 'name'"
            self.log(f"Merchandiser agent: {card.get('name')}")

    async def test_customer_agent_chat(self):
        """Test customer agent /chat endpoint with a simple query."""
        async with httpx.AsyncClient(timeout=60.0) as client:
            response = await client.post(
                f"{self.customer_agent_url}/chat",
                json={
                    "message": "What products do you have?",
                    "user_id": "test-user",
                    "session_id": "test-session",
                },
            )
            assert response.status_code == 200, f"Chat returned {response.status_code}"

            result = response.json()
            assert "response" in result, f"Missing 'response' in: {result}"
            self.log(f"Agent response: {result['response'][:100]}...")

    async def test_merchandiser_agent_chat(self):
        """Test merchandiser agent /chat endpoint with a simple query."""
        async with httpx.AsyncClient(timeout=60.0) as client:
            response = await client.post(
                f"{self.merchandiser_agent_url}/chat",
                json={
                    "message": "Show me the current inventory status",
                    "user_id": "test-merch",
                    "session_id": "test-session",
                },
            )
            assert response.status_code == 200, f"Chat returned {response.status_code}"

            result = response.json()
            assert "response" in result, f"Missing 'response' in: {result}"
            self.log(f"Agent response: {result['response'][:100]}...")

    # ==================== Test Runner ====================

    async def run_gateway_tests(self):
        """Run all gateway-level tests."""
        print("\n=== Gateway Tests ===")
        await self.setup()

        await self.run_test("Gateway Health", self.test_gateway_health)
        await self.run_test("Tool Discovery", self.test_tool_discovery)
        await self.run_test("Basic Passthrough (find_products)", self.test_basic_passthrough_tool)
        await self.run_test("Stock Status Tool", self.test_stock_status_tool)
        await self.run_test("Raw Search Products", self.test_search_products_raw)
        await self.run_test("Pipeline Composition (personalized_search)", self.test_composition_pipeline)
        await self.run_test("Parallel Pipeline (product_with_availability)", self.test_composition_parallel_pipeline)
        await self.run_test("Cart Operations", self.test_cart_operations)

    async def run_agent_tests(self):
        """Run agent-level tests."""
        print("\n=== Agent Tests ===")

        await self.run_test("Customer Agent Health", self.test_customer_agent_health)
        await self.run_test("Merchandiser Agent Health", self.test_merchandiser_agent_health)
        await self.run_test("Customer Agent Chat", self.test_customer_agent_chat)
        await self.run_test("Merchandiser Agent Chat", self.test_merchandiser_agent_chat)

    def print_summary(self):
        """Print test summary."""
        print("\n" + "=" * 60)
        passed = sum(1 for r in self.results if r.passed)
        failed = sum(1 for r in self.results if not r.passed)
        total_time = sum(r.duration_ms for r in self.results)

        print(f"Results: {passed} passed, {failed} failed ({total_time:.0f}ms total)")

        if failed:
            print("\nFailed tests:")
            for r in self.results:
                if not r.passed:
                    print(f"  - {r.name}: {r.message}")

        print("=" * 60)
        return failed == 0


async def main():
    parser = argparse.ArgumentParser(description="Integration tests for ecommerce demo")
    parser.add_argument(
        "--gateway-url",
        default="http://localhost:3000",
        help="Gateway URL (default: http://localhost:3000)",
    )
    parser.add_argument(
        "--customer-agent-url",
        default="http://localhost:9001",
        help="Customer agent URL (default: http://localhost:9001)",
    )
    parser.add_argument(
        "--merchandiser-agent-url",
        default="http://localhost:9002",
        help="Merchandiser agent URL (default: http://localhost:9002)",
    )
    parser.add_argument(
        "--gateway-only",
        action="store_true",
        help="Only run gateway tests (skip agent tests)",
    )
    parser.add_argument(
        "-v", "--verbose", action="store_true", help="Verbose output"
    )
    args = parser.parse_args()

    tester = IntegrationTester(
        gateway_url=args.gateway_url,
        customer_agent_url=args.customer_agent_url,
        merchandiser_agent_url=args.merchandiser_agent_url,
        verbose=args.verbose,
    )

    await tester.run_gateway_tests()

    if not args.gateway_only:
        await tester.run_agent_tests()

    success = tester.print_summary()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    asyncio.run(main())
