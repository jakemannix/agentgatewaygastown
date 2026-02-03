"""Pytest fixtures for virtual tools integration tests.

These tests verify gateway composition logic without requiring an LLM.
Services and gateway are started as subprocesses.
"""

import asyncio
import json
import os
import socket
import subprocess
import sys
import time
import uuid
from pathlib import Path
from typing import AsyncGenerator, Callable, Generator

import httpx
import pytest
import pytest_asyncio


# Path constants
DEMO_DIR = Path(__file__).parent.parent
REPO_ROOT = DEMO_DIR.parent.parent
GATEWAY_BIN = REPO_ROOT / "target" / "debug" / "agentgateway"
CONFIG_FILE = DEMO_DIR / "gateway-configs" / "config.yaml"

# Service configuration
SERVICES = [
    (8001, "mcp_tools.search_service"),
    (8002, "mcp_tools.fetch_service"),
    (8003, "mcp_tools.entity_service"),
    (8004, "mcp_tools.category_service"),
    (8005, "mcp_tools.tag_service"),
]

GATEWAY_PORT = 3000
GATEWAY_URL = f"http://localhost:{GATEWAY_PORT}/mcp"
MAX_STARTUP_WAIT_SECS = 30
POLL_INTERVAL_SECS = 0.5


def wait_for_port(port: int, timeout: float = MAX_STARTUP_WAIT_SECS) -> bool:
    """Wait for a port to become available."""
    start = time.time()
    while time.time() - start < timeout:
        try:
            with socket.create_connection(("localhost", port), timeout=1):
                return True
        except (ConnectionRefusedError, socket.timeout, OSError):
            time.sleep(POLL_INTERVAL_SECS)
    return False


class ServiceManager:
    """Manages backend MCP services lifecycle."""

    def __init__(self):
        self.procs: list[subprocess.Popen] = []

    def start(self) -> None:
        """Start all backend services and wait for them to be ready."""
        for port, module in SERVICES:
            proc = subprocess.Popen(
                [sys.executable, "-m", module, "--port", str(port)],
                cwd=str(DEMO_DIR),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            self.procs.append(proc)

        # Wait for all services to be ready
        for i, (port, module) in enumerate(SERVICES):
            if not wait_for_port(port):
                # Get stderr for debugging
                stderr = self.procs[i].stderr.read().decode() if self.procs[i].stderr else "no stderr"
                pytest.fail(f"Service {module} on port {port} failed to start. stderr: {stderr[:500]}")

    def stop(self) -> None:
        """Stop all backend services."""
        for proc in self.procs:
            try:
                proc.terminate()
                proc.wait(timeout=5)
            except Exception:
                proc.kill()
        self.procs.clear()

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, *args):
        self.stop()


class GatewayManager:
    """Manages agentgateway lifecycle."""

    def __init__(self):
        self.proc: subprocess.Popen | None = None

    def start(self) -> None:
        """Start the gateway and wait for it to be ready."""
        if not GATEWAY_BIN.exists():
            pytest.skip(f"Gateway binary not found at {GATEWAY_BIN}. Run: cargo build -p agentgateway-app")

        env = os.environ.copy()
        env["RUST_LOG"] = "warn"  # Reduce log noise during tests

        self.proc = subprocess.Popen(
            [str(GATEWAY_BIN), "-f", str(CONFIG_FILE)],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
        )

        if not wait_for_port(GATEWAY_PORT):
            # Get stderr for debugging
            stderr = self.proc.stderr.read().decode() if self.proc and self.proc.stderr else "no stderr"
            pytest.fail(f"Gateway on port {GATEWAY_PORT} failed to start. stderr: {stderr[:500]}")

    def stop(self) -> None:
        """Stop the gateway."""
        if self.proc:
            try:
                self.proc.terminate()
                self.proc.wait(timeout=5)
            except Exception:
                self.proc.kill()
            self.proc = None

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, *args):
        self.stop()


class McpHttpClient:
    """Simple MCP client using httpx for testing."""

    def __init__(self, base_url: str):
        self.base_url = base_url
        self.session_id: str | None = None
        self.client = httpx.Client(timeout=60.0)

    def initialize(self) -> dict:
        """Send MCP initialize request (no session header - server creates session)."""
        payload = {
            "jsonrpc": "2.0",
            "id": str(uuid.uuid4()),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "pytest", "version": "1.0.0"},
            },
        }

        # Initialize request has NO session header
        resp = self.client.post(
            self.base_url,
            json=payload,
            headers={
                "Content-Type": "application/json",
                "Accept": "application/json, text/event-stream",
            },
        )
        resp.raise_for_status()

        # Get session ID from response header
        self.session_id = resp.headers.get("mcp-session-id")
        if not self.session_id:
            raise Exception(f"No session ID in response. Headers: {dict(resp.headers)}")

        return self._parse_response(resp.text)

    def call_tool(self, name: str, arguments: dict) -> dict:
        """Call a tool and return the result."""
        if not self.session_id:
            self.initialize()

        payload = {
            "jsonrpc": "2.0",
            "id": str(uuid.uuid4()),
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments},
        }

        resp = self.client.post(
            self.base_url,
            json=payload,
            headers={
                "Content-Type": "application/json",
                "Accept": "application/json, text/event-stream",
                "mcp-session-id": self.session_id,
            },
        )
        resp.raise_for_status()
        return self._parse_response(resp.text)

    def _parse_response(self, text: str) -> dict:
        """Parse SSE response and extract result."""
        result = None
        for line in text.split("\n"):
            if line.startswith("data: "):
                data = json.loads(line[6:])
                if "result" in data:
                    result = data["result"]
                elif "error" in data:
                    raise Exception(f"MCP error: {data['error']}")

        if not result:
            return {}

        # For tool calls, extract structuredContent or parse content text
        if "structuredContent" in result and result["structuredContent"]:
            return result["structuredContent"]
        elif "content" in result and result["content"]:
            # Try to parse the first text content as JSON
            for item in result["content"]:
                if item.get("type") == "text":
                    try:
                        return json.loads(item["text"])
                    except json.JSONDecodeError:
                        pass
            return result
        return result

    def close(self):
        """Close the client."""
        self.client.close()


@pytest.fixture(scope="session")
def services() -> Generator[ServiceManager, None, None]:
    """Start backend MCP services for the test session."""
    print("\n=== Starting backend services ===")
    with ServiceManager() as mgr:
        print("=== Backend services started ===")
        yield mgr
    print("=== Backend services stopped ===")


@pytest.fixture(scope="session")
def gateway(services: ServiceManager) -> Generator[GatewayManager, None, None]:
    """Start gateway for the test session (requires services)."""
    print("\n=== Starting gateway ===")
    with GatewayManager() as mgr:
        print("=== Gateway started ===")
        yield mgr
    print("=== Gateway stopped ===")


@pytest.fixture(scope="session")
def mcp_client(gateway: GatewayManager) -> Generator[McpHttpClient, None, None]:
    """Create an MCP client for the test session."""
    client = McpHttpClient(GATEWAY_URL)
    client.initialize()
    print(f"\n=== MCP session established: {client.session_id} ===")
    yield client
    client.close()


@pytest.fixture
def call_tool(mcp_client: McpHttpClient) -> Callable:
    """Provide a helper function to call tools.

    Usage in tests:
        def test_foo(call_tool):
            result = call_tool("virtual_tool_name", {"arg": "value"})
    """
    return mcp_client.call_tool
