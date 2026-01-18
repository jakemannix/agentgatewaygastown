#!/usr/bin/env python3
"""
run_demo.py - Interactive demo for AgentGateway patterns

This script demonstrates all major agentgateway patterns through
interactive examples. Each demo shows a specific pattern:

1. MCP Multiplexing - Multiple backend servers through one gateway
2. Tool Aliasing - Semantic tool name mapping
3. Output Projection - Extract specific fields from responses
4. Output Transformation - Restructure/flatten nested responses
5. Tool Composition - Pipeline multiple tools together
6. A2A Proxy - Agent-to-agent communication

Usage:
    uv run python run_demo.py                    # Interactive menu
    uv run python run_demo.py --demo aliasing    # Run specific demo
    uv run python run_demo.py --list             # List available demos
    uv run python run_demo.py --all              # Run all demos

Setup:
    make setup   # or: uv sync
"""

import argparse
import asyncio
import json
import sys
from typing import Any

try:
    import httpx
    from rich.console import Console
    from rich.panel import Panel
    from rich.syntax import Syntax
    from rich.table import Table
    from rich.prompt import Prompt, Confirm
    from rich.progress import Progress, SpinnerColumn, TextColumn
except ImportError:
    print("Required packages missing. Install with:")
    print("  make setup   # or: uv sync")
    sys.exit(1)

console = Console()

# Gateway configuration
GATEWAY_URL = "http://localhost:3000"
MCP_ENDPOINT = f"{GATEWAY_URL}/mcp"
SSE_ENDPOINT = f"{GATEWAY_URL}/sse"
A2A_ENDPOINT = f"{GATEWAY_URL}/a2a"
DEBUG = False  # Set via --debug flag


class MCPClient:
    """Simple MCP client for demo purposes with detailed error reporting."""

    DEFAULT_HEADERS = {
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream"
    }

    def __init__(self, base_url: str = MCP_ENDPOINT, debug: bool = False):
        self.base_url = base_url
        self.session_id = None
        self.debug = debug
        self.client = httpx.AsyncClient(timeout=60.0)

    def _log(self, msg: str):
        """Print debug message if debug mode is enabled."""
        if self.debug:
            console.print(f"[dim][DEBUG] {msg}[/dim]")

    def _headers(self) -> dict:
        """Get headers with session ID if available."""
        headers = self.DEFAULT_HEADERS.copy()
        if self.session_id:
            headers["mcp-session-id"] = self.session_id
        return headers

    def _parse_response(self, response, method: str = "unknown") -> dict:
        """Parse response, handling both JSON and SSE formats."""
        content_type = response.headers.get("content-type", "")
        text = response.text.strip()
        
        self._log(f"{method} response: status={response.status_code}, content-type={content_type}")
        
        if response.status_code != 200:
            raise MCPError(f"{method} failed with status {response.status_code}: {text[:500]}")
        
        if not text:
            raise MCPError(f"{method} returned empty response")
        
        # Handle SSE format: extract JSON from "data: {...}" lines
        if "text/event-stream" in content_type or text.startswith("data:"):
            for line in text.split("\n"):
                line = line.strip()
                if line.startswith("data:"):
                    json_str = line[5:].strip()
                    if json_str:
                        try:
                            result = json.loads(json_str)
                            self._log(f"{method} parsed SSE response OK")
                            return result
                        except json.JSONDecodeError as e:
                            raise MCPError(f"{method} SSE response has invalid JSON: {e}\nContent: {json_str[:200]}")
            raise MCPError(f"{method} SSE response has no data lines: {text[:200]}")
        
        # Regular JSON response
        try:
            result = response.json()
            self._log(f"{method} parsed JSON response OK")
            return result
        except json.JSONDecodeError as e:
            raise MCPError(f"{method} response is not valid JSON: {e}\nContent: {text[:200]}")

    async def initialize(self) -> dict:
        """Initialize MCP session."""
        self._log(f"Initializing session with {self.base_url}")
        try:
            response = await self.client.post(
                self.base_url,
                json={
                    "jsonrpc": "2.0",
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "clientInfo": {"name": "pattern-demo", "version": "1.0.0"}
                    },
                    "id": 1
                },
                headers=self._headers()
            )
        except httpx.ConnectError as e:
            raise MCPError(f"Cannot connect to gateway at {self.base_url}: {e}")
        except httpx.TimeoutException as e:
            raise MCPError(f"Timeout connecting to gateway at {self.base_url}: {e}")
        
        result = self._parse_response(response, "initialize")
        self.session_id = response.headers.get("mcp-session-id")
        self._log(f"Session initialized, session_id={self.session_id}")
        
        # Check for JSON-RPC error
        if "error" in result:
            raise MCPError(f"initialize returned error: {result['error']}")
        
        return result

    async def list_tools(self) -> list[dict]:
        """List available tools."""
        self._log("Listing tools...")
        try:
            response = await self.client.post(
                self.base_url,
                json={
                    "jsonrpc": "2.0",
                    "method": "tools/list",
                    "params": {},
                    "id": 2
                },
                headers=self._headers()
            )
        except httpx.TimeoutException as e:
            raise MCPError(f"Timeout listing tools: {e}")
        
        result = self._parse_response(response, "tools/list")
        
        if "error" in result:
            raise MCPError(f"tools/list returned error: {result['error']}")
        
        tools = result.get("result", {}).get("tools", [])
        self._log(f"Found {len(tools)} tools")
        return tools

    async def call_tool(self, name: str, arguments: dict) -> dict:
        """Call a tool by name."""
        self._log(f"Calling tool '{name}' with args: {json.dumps(arguments)[:100]}")
        try:
            response = await self.client.post(
                self.base_url,
                json={
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {"name": name, "arguments": arguments},
                    "id": 3
                },
                headers=self._headers()
            )
        except httpx.TimeoutException as e:
            raise MCPError(f"Timeout calling tool '{name}': {e}\nThis may mean the backend server is not responding.")
        
        result = self._parse_response(response, f"tools/call({name})")
        
        if "error" in result:
            err = result["error"]
            err_msg = err.get("message", str(err)) if isinstance(err, dict) else str(err)
            raise MCPError(f"Tool '{name}' returned error: {err_msg}")
        
        return result

    async def close(self):
        """Close the client."""
        await self.client.aclose()


class MCPError(Exception):
    """Error from MCP operations with helpful context."""
    pass


async def check_gateway() -> bool:
    """Check if gateway is running."""
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            # Try MCP initialize to check if gateway is up
            response = await client.post(
                MCP_ENDPOINT,
                json={
                    "jsonrpc": "2.0",
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "clientInfo": {"name": "health-check", "version": "1.0.0"}
                    },
                    "id": 0
                },
                headers={
                    "Content-Type": "application/json",
                    "Accept": "application/json, text/event-stream"
                }
            )
            return response.status_code == 200
    except Exception:
        return False


def print_json(data: Any, title: str = "Response"):
    """Pretty print JSON data."""
    json_str = json.dumps(data, indent=2)
    syntax = Syntax(json_str, "json", theme="monokai", line_numbers=False)
    console.print(Panel(syntax, title=title, border_style="green"))


def print_demo_header(title: str, description: str):
    """Print demo section header."""
    console.print()
    console.print(Panel(
        f"[bold]{title}[/bold]\n\n{description}",
        border_style="blue",
        padding=(1, 2)
    ))
    console.print()


async def demo_multiplexing(debug: bool = False):
    """Demo 1: MCP Multiplexing - multiple backend servers."""
    print_demo_header(
        "Pattern 1: MCP Multiplexing",
        "AgentGateway aggregates multiple MCP servers into a single endpoint.\n"
        "Clients see tools from fetch-server, memory-server, time-server, etc.\n"
        "as a unified tool catalog."
    )

    client = MCPClient(debug=debug)
    try:
        # Initialize
        console.print("[dim]Initializing MCP session...[/dim]")
        await client.initialize()
        # List tools
        tools = await client.list_tools()

        # Group by source server
        table = Table(title="Aggregated Tools by Server")
        table.add_column("Server", style="cyan")
        table.add_column("Tool", style="green")
        table.add_column("Description", style="dim")

        # Detect server from tool name prefix
        for tool in tools[:15]:  # Show first 15
            name = tool.get("name", "")
            desc = tool.get("description", "")[:50] + "..."

            # Infer server from naming convention
            if "fetch" in name.lower() or "browse" in name.lower() or "webpage" in name.lower():
                server = "fetch-server"
            elif "graph" in name.lower() or "entity" in name.lower() or "memory" in name.lower():
                server = "memory-server"
            elif "time" in name.lower() or "day" in name.lower() or "timezone" in name.lower():
                server = "time-server"
            else:
                server = "registry"

            table.add_row(server, name, desc)

        console.print(table)
        console.print(f"\n[green]Total tools available:[/green] {len(tools)}")

    except MCPError as e:
        console.print(f"[red]Error:[/red] {e}")
    except Exception as e:
        console.print(f"[red]Unexpected error:[/red] {type(e).__name__}: {e}")
    finally:
        await client.close()


async def demo_aliasing(debug: bool = False):
    """Demo 2: Tool Aliasing - semantic name mapping."""
    print_demo_header(
        "Pattern 2: Tool Aliasing",
        "Virtual tools map semantic names to underlying tools.\n"
        "Example: 'get_webpage', 'browse', 'download_page' all map to 'fetch'.\n"
        "This allows agents to use natural language tool names."
    )

    client = MCPClient(debug=debug)
    try:
        await client.initialize()

        # Show aliased tools
        aliases = [
            ("get_webpage", "Semantic alias for fetch"),
            ("browse", "Alternative name for web browsing"),
        ]

        console.print("[bold]Calling aliased tools:[/bold]\n")

        for alias_name, desc in aliases:
            console.print(f"[cyan]Tool:[/cyan] {alias_name} - {desc}")

            try:
                result = await client.call_tool(alias_name, {"url": "https://example.com"})

                if "result" in result:
                    content = result["result"].get("content", [])
                    if content:
                        text = content[0].get("text", "")[:200]
                        console.print(f"[green]✓ Success![/green] Response preview: {text}...")
                    else:
                        console.print(f"[green]✓ Success![/green] (empty content)")
                else:
                    console.print(f"[yellow]Unexpected response:[/yellow] {result}")
            except MCPError as e:
                console.print(f"[red]✗ Tool error:[/red] {e}")
            except Exception as e:
                console.print(f"[red]✗ Request failed:[/red] {type(e).__name__}: {e}")
            console.print()

    except MCPError as e:
        console.print(f"[red]Error:[/red] {e}")
    except Exception as e:
        console.print(f"[red]Unexpected error:[/red] {type(e).__name__}: {e}")
    finally:
        await client.close()


async def demo_projection(debug: bool = False):
    """Demo 3: Output Projection - extract specific fields."""
    print_demo_header(
        "Pattern 3: Output Projection",
        "Virtual tools can extract specific fields from complex responses.\n"
        "Example: 'list_entity_names' extracts just names from read_graph.\n"
        "This reduces token usage and simplifies agent processing."
    )

    client = MCPClient(debug=debug)
    try:
        await client.initialize()

        # First, create some entities
        console.print("[dim]Creating test entities in knowledge graph...[/dim]")
        try:
            await client.call_tool("create_entities", {
                "entities": [
                    {"name": "Alice", "entityType": "person", "observations": ["Likes Python"]},
                    {"name": "Bob", "entityType": "person", "observations": ["Likes Rust"]},
                    {"name": "AgentGateway", "entityType": "project", "observations": ["MCP proxy"]}
                ]
            })
            console.print("[green]✓[/green] Entities created")
        except MCPError as e:
            console.print(f"[yellow]Note: Could not create entities (may already exist): {e}[/yellow]")

        # Compare full vs projected response
        console.print("\n[bold]Full response (read_graph):[/bold]")
        try:
            full_result = await client.call_tool("read_graph", {})
            print_json(full_result.get("result", {}), "read_graph response")
        except MCPError as e:
            console.print(f"[red]✗ read_graph failed:[/red] {e}")
            return

        console.print("\n[bold]Projected response (list_entity_names):[/bold]")
        try:
            projected_result = await client.call_tool("list_entity_names", {})
            print_json(projected_result.get("result", {}), "list_entity_names response")
            console.print("\n[green]Notice:[/green] Projected response contains only the entity names!")
        except MCPError as e:
            console.print(f"[yellow]✗ list_entity_names not available:[/yellow] {e}")
            console.print("[dim]This virtual tool may not be defined in the registry.[/dim]")

    except MCPError as e:
        console.print(f"[red]Error:[/red] {e}")
    finally:
        await client.close()


async def demo_transformation(debug: bool = False):
    """Demo 4: Output Transformation - restructure responses."""
    print_demo_header(
        "Pattern 4: Output Transformation",
        "Virtual tools can reshape and flatten nested JSON responses.\n"
        "Example: 'get_time_structured' extracts nested fields into flat object.\n"
        "This normalizes outputs across different backend servers."
    )

    client = MCPClient(debug=debug)
    try:
        await client.initialize()

        console.print("[bold]Raw time response (get_current_time):[/bold]")
        try:
            raw_result = await client.call_tool("get_current_time", {"timezone": "America/Los_Angeles"})
            print_json(raw_result.get("result", {}), "get_current_time response")
        except MCPError as e:
            console.print(f"[red]✗ get_current_time failed:[/red] {e}")
            console.print("[dim]The time-server backend may not be available.[/dim]")
            return

        console.print("\n[bold]Transformed response (get_time_structured):[/bold]")
        try:
            transformed_result = await client.call_tool("get_time_structured", {"timezone": "America/Los_Angeles"})
            print_json(transformed_result.get("result", {}), "get_time_structured response")
        except MCPError as e:
            console.print(f"[yellow]✗ get_time_structured not available:[/yellow] {e}")
            console.print("[dim]This virtual tool may not be defined in the registry.[/dim]")

        console.print("\n[bold]Minimal projection (what_day_is_it):[/bold]")
        try:
            minimal_result = await client.call_tool("what_day_is_it", {"timezone": "America/Los_Angeles"})
            print_json(minimal_result.get("result", {}), "what_day_is_it response")
        except MCPError as e:
            console.print(f"[yellow]✗ what_day_is_it not available:[/yellow] {e}")
            console.print("[dim]This virtual tool may not be defined in the registry.[/dim]")

    except MCPError as e:
        console.print(f"[red]Error:[/red] {e}")
    finally:
        await client.close()


async def demo_composition(debug: bool = False):
    """Demo 5: Tool Composition - pipeline multiple tools."""
    print_demo_header(
        "Pattern 5: Tool Composition (Pipeline)",
        "Virtual tools can chain multiple tools into a single operation.\n"
        "Example: 'fetch_and_remember' fetches URL then stores in knowledge graph.\n"
        "This enables complex workflows with a single tool call."
    )

    client = MCPClient(debug=debug)
    try:
        await client.initialize()

        console.print("[bold]Calling pipeline tool (fetch_and_remember):[/bold]")
        console.print("[dim]This will: 1) Fetch URL content, 2) Store in knowledge graph[/dim]\n")

        try:
            result = await client.call_tool("fetch_and_remember", {"url": "https://example.com"})
            print_json(result.get("result", {}), "fetch_and_remember response")
        except MCPError as e:
            console.print(f"[yellow]✗ fetch_and_remember not available:[/yellow] {e}")
            console.print("[dim]This virtual tool may not be defined in the registry, or a backend is unavailable.[/dim]")
            return

        # Verify it was stored
        console.print("\n[bold]Verifying entity was created:[/bold]")
        try:
            graph = await client.call_tool("read_graph", {})
            entities = graph.get("result", {}).get("content", [])
            if entities:
                text = entities[0].get("text", "")
                if "fetched_page" in text:
                    console.print("[green]✓ Success![/green] Pipeline created entity in knowledge graph.")
                else:
                    console.print("[yellow]Entity exists but may not match expected format[/yellow]")
            else:
                console.print("[yellow]No entities found in graph[/yellow]")
        except MCPError as e:
            console.print(f"[yellow]Could not verify: {e}[/yellow]")

    except MCPError as e:
        console.print(f"[red]Error:[/red] {e}")
    finally:
        await client.close()


async def demo_a2a(debug: bool = False):
    """Demo 6: A2A Proxy - agent-to-agent communication."""
    print_demo_header(
        "Pattern 6: A2A (Agent-to-Agent) Proxy",
        "AgentGateway can proxy A2A protocol traffic to backend agents.\n"
        "This enables secure, observable agent-to-agent communication.\n"
        "Note: Requires A2A agent running on localhost:9999."
    )

    console.print("[yellow]This demo requires an A2A agent running.[/yellow]")
    console.print("Start one with: [cyan]cd examples/a2a/strands-agents && uv run .[/cyan]")
    console.print()

    # Try to fetch agent card
    try:
        if debug:
            console.print(f"[dim][DEBUG] Fetching {A2A_ENDPOINT}/.well-known/agent.json[/dim]")
        async with httpx.AsyncClient(timeout=5.0) as client:
            response = await client.get(f"{A2A_ENDPOINT}/.well-known/agent.json")
            if debug:
                console.print(f"[dim][DEBUG] Response status: {response.status_code}[/dim]")
            if response.status_code == 200:
                agent_card = response.json()
                print_json(agent_card, "A2A Agent Card")
                console.print("\n[green]✓ A2A agent is reachable through the gateway![/green]")
                console.print(f"[dim]Notice: URL rewritten to point to gateway: {A2A_ENDPOINT}[/dim]")
            else:
                console.print(f"[red]✗ A2A agent returned status {response.status_code}[/red]")
                console.print(f"[dim]Response: {response.text[:200]}[/dim]")
    except httpx.ConnectError as e:
        console.print(f"[red]✗ Cannot connect to A2A endpoint:[/red] {e}")
        console.print("[dim]Make sure the gateway is running and an A2A agent is on port 9999[/dim]")
    except httpx.TimeoutException as e:
        console.print(f"[red]✗ Timeout reaching A2A endpoint:[/red] {e}")
    except Exception as e:
        console.print(f"[red]✗ Could not reach A2A agent:[/red] {type(e).__name__}: {e}")


DEMOS = {
    "multiplexing": ("MCP Multiplexing", demo_multiplexing),
    "aliasing": ("Tool Aliasing", demo_aliasing),
    "projection": ("Output Projection", demo_projection),
    "transformation": ("Output Transformation", demo_transformation),
    "composition": ("Tool Composition", demo_composition),
    "a2a": ("A2A Proxy", demo_a2a),
}


async def run_all_demos(debug: bool = False):
    """Run all demos in sequence."""
    for name, (title, demo_fn) in DEMOS.items():
        try:
            await demo_fn(debug=debug)
        except Exception as e:
            console.print(f"[red]Demo '{name}' failed: {type(e).__name__}: {e}[/red]")
        if name != list(DEMOS.keys())[-1]:
            console.print("\n" + "=" * 60 + "\n")
            if not Confirm.ask("Continue to next demo?", default=True):
                break


async def interactive_menu(debug: bool = False):
    """Show interactive demo menu."""
    console.print(Panel(
        "[bold blue]AgentGateway Pattern Demos[/bold blue]\n\n"
        "These demos showcase the key patterns that agentgateway enables:\n"
        "- MCP server aggregation/multiplexing\n"
        "- Virtual tool definitions (aliasing, projection, transformation)\n"
        "- Tool composition (pipelines)\n"
        "- A2A agent proxying",
        border_style="blue"
    ))

    # Check gateway
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        transient=True,
    ) as progress:
        progress.add_task("Checking gateway connection...", total=None)
        gateway_up = await check_gateway()

    if not gateway_up:
        console.print(f"\n[red]Gateway not reachable at {GATEWAY_URL}[/red]")
        console.print("Start it with: [cyan]./start_gateway.sh[/cyan]")
        return

    console.print(f"\n[green]Gateway connected at {GATEWAY_URL}[/green]\n")

    while True:
        # Build menu
        table = Table(show_header=False, box=None)
        table.add_column("Key", style="cyan", width=3)
        table.add_column("Demo", style="bold")
        table.add_column("Description", style="dim")

        for i, (key, (title, _)) in enumerate(DEMOS.items(), 1):
            table.add_row(str(i), title, key)

        table.add_row("a", "Run All", "all demos in sequence")
        table.add_row("q", "Quit", "exit")

        console.print(table)
        console.print()

        choice = Prompt.ask("Select demo", choices=list(DEMOS.keys()) + ["a", "q", "1", "2", "3", "4", "5", "6"])

        if choice == "q":
            break
        elif choice == "a":
            await run_all_demos(debug=debug)
        elif choice.isdigit():
            idx = int(choice) - 1
            if 0 <= idx < len(DEMOS):
                key = list(DEMOS.keys())[idx]
                _, demo_fn = DEMOS[key]
                try:
                    await demo_fn(debug=debug)
                except Exception as e:
                    console.print(f"[red]Demo failed: {type(e).__name__}: {e}[/red]")
        elif choice in DEMOS:
            _, demo_fn = DEMOS[choice]
            try:
                await demo_fn(debug=debug)
            except Exception as e:
                console.print(f"[red]Demo failed: {type(e).__name__}: {e}[/red]")

        console.print()


def main():
    global GATEWAY_URL, MCP_ENDPOINT, SSE_ENDPOINT, A2A_ENDPOINT, DEBUG

    parser = argparse.ArgumentParser(description="AgentGateway Pattern Demos")
    parser.add_argument("--demo", choices=list(DEMOS.keys()), help="Run specific demo")
    parser.add_argument("--list", action="store_true", help="List available demos")
    parser.add_argument("--all", action="store_true", help="Run all demos")
    parser.add_argument("--url", default=GATEWAY_URL, help="Gateway URL")
    parser.add_argument("--debug", action="store_true", help="Enable debug output")
    args = parser.parse_args()

    GATEWAY_URL = args.url
    MCP_ENDPOINT = f"{GATEWAY_URL}/mcp"
    SSE_ENDPOINT = f"{GATEWAY_URL}/sse"
    A2A_ENDPOINT = f"{GATEWAY_URL}/a2a"
    DEBUG = args.debug

    if args.debug:
        console.print("[yellow]Debug mode enabled[/yellow]\n")

    if args.list:
        console.print("[bold]Available demos:[/bold]")
        for key, (title, _) in DEMOS.items():
            console.print(f"  {key}: {title}")
        return

    if args.demo:
        _, demo_fn = DEMOS[args.demo]
        asyncio.run(demo_fn(debug=DEBUG))
    elif args.all:
        asyncio.run(run_all_demos(debug=DEBUG))
    else:
        asyncio.run(interactive_menu(debug=DEBUG))


if __name__ == "__main__":
    main()
