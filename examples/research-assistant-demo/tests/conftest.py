"""Pytest fixtures for virtual tools integration tests.

These tests verify gateway composition logic without requiring an LLM.
Services and gateway are started as subprocesses.
"""

import asyncio
import os
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import AsyncGenerator, Generator

import pytest
import pytest_asyncio
from mcp import ClientSession
from mcp.client.streamable_http import streamablehttp_client


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

GATEWAY_URL = "http://localhost:3000/mcp"
STARTUP_WAIT_SECS = 3
GATEWAY_WAIT_SECS = 2


class ServiceManager:
    """Manages backend MCP services lifecycle."""

    def __init__(self):
        self.procs: list[subprocess.Popen] = []

    def start(self) -> None:
        """Start all backend services."""
        for port, module in SERVICES:
            proc = subprocess.Popen(
                [sys.executable, "-m", module, "--port", str(port)],
                cwd=str(DEMO_DIR),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            self.procs.append(proc)

        time.sleep(STARTUP_WAIT_SECS)

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
        """Start the gateway."""
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
        time.sleep(GATEWAY_WAIT_SECS)

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


@pytest.fixture(scope="session")
def event_loop() -> Generator[asyncio.AbstractEventLoop, None, None]:
    """Create event loop for async tests."""
    loop = asyncio.new_event_loop()
    yield loop
    loop.close()


@pytest.fixture(scope="session")
def services() -> Generator[ServiceManager, None, None]:
    """Start backend MCP services for the test session."""
    with ServiceManager() as mgr:
        yield mgr


@pytest.fixture(scope="session")
def gateway(services: ServiceManager) -> Generator[GatewayManager, None, None]:
    """Start gateway for the test session (requires services)."""
    with GatewayManager() as mgr:
        yield mgr


@pytest_asyncio.fixture
async def mcp_client(gateway: GatewayManager) -> AsyncGenerator[ClientSession, None]:
    """Create an MCP client session connected to the gateway."""
    async with streamablehttp_client(GATEWAY_URL) as (read_stream, write_stream, _):
        async with ClientSession(read_stream, write_stream) as session:
            await session.initialize()
            yield session


async def _call_tool_impl(session: ClientSession, name: str, arguments: dict) -> dict:
    """Call a tool and return the result as a dict.

    Args:
        session: MCP client session
        name: Tool name (with or without virtual_ prefix)
        arguments: Tool arguments

    Returns:
        Tool result as a dictionary
    """
    import json

    result = await session.call_tool(name, arguments)

    # Extract content from result
    if result.content and len(result.content) > 0:
        content = result.content[0]
        if hasattr(content, "text"):
            return json.loads(content.text)

    return {}


@pytest_asyncio.fixture
async def call_tool(mcp_client: ClientSession):
    """Provide a helper function to call tools.

    Usage in tests:
        async def test_foo(call_tool):
            result = await call_tool("virtual_tool_name", {"arg": "value"})
    """

    async def _call(name: str, arguments: dict) -> dict:
        return await _call_tool_impl(mcp_client, name, arguments)

    return _call
