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
    python run_demo.py                    # Interactive menu
    python run_demo.py --demo aliasing    # Run specific demo
    python run_demo.py --list             # List available demos
    python run_demo.py --all              # Run all demos

Requirements:
    pip install httpx rich
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
    print("  pip install httpx rich")
    sys.exit(1)

console = Console()

# Gateway configuration
GATEWAY_URL = "http://localhost:3000"
MCP_ENDPOINT = f"{GATEWAY_URL}/mcp"
SSE_ENDPOINT = f"{GATEWAY_URL}/sse"
A2A_ENDPOINT = f"{GATEWAY_URL}/a2a"


class MCPClient:
    """Simple MCP client for demo purposes."""

    def __init__(self, base_url: str = MCP_ENDPOINT):
        self.base_url = base_url
        self.session_id = None
        self.client = httpx.AsyncClient(timeout=30.0)

    async def initialize(self) -> dict:
        """Initialize MCP session."""
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
            headers={"Content-Type": "application/json"}
        )
        result = response.json()
        self.session_id = response.headers.get("mcp-session-id")
        return result

    async def list_tools(self) -> list[dict]:
        """List available tools."""
        headers = {"Content-Type": "application/json"}
        if self.session_id:
            headers["mcp-session-id"] = self.session_id

        response = await self.client.post(
            self.base_url,
            json={
                "jsonrpc": "2.0",
                "method": "tools/list",
                "params": {},
                "id": 2
            },
            headers=headers
        )
        result = response.json()
        return result.get("result", {}).get("tools", [])

    async def call_tool(self, name: str, arguments: dict) -> Any:
        """Call a tool by name."""
        headers = {"Content-Type": "application/json"}
        if self.session_id:
            headers["mcp-session-id"] = self.session_id

        response = await self.client.post(
            self.base_url,
            json={
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {"name": name, "arguments": arguments},
                "id": 3
            },
            headers=headers
        )
        return response.json()

    async def close(self):
        """Close the client."""
        await self.client.aclose()


async def check_gateway() -> bool:
    """Check if gateway is running."""
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            response = await client.get(f"{GATEWAY_URL}/health")
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


async def demo_multiplexing():
    """Demo 1: MCP Multiplexing - multiple backend servers."""
    print_demo_header(
        "Pattern 1: MCP Multiplexing",
        "AgentGateway aggregates multiple MCP servers into a single endpoint.\n"
        "Clients see tools from fetch-server, memory-server, time-server, etc.\n"
        "as a unified tool catalog."
    )

    client = MCPClient()
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

    finally:
        await client.close()


async def demo_aliasing():
    """Demo 2: Tool Aliasing - semantic name mapping."""
    print_demo_header(
        "Pattern 2: Tool Aliasing",
        "Virtual tools map semantic names to underlying tools.\n"
        "Example: 'get_webpage', 'browse', 'download_page' all map to 'fetch'.\n"
        "This allows agents to use natural language tool names."
    )

    client = MCPClient()
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

            result = await client.call_tool(alias_name, {"url": "https://example.com"})

            if "result" in result:
                content = result["result"].get("content", [])
                if content:
                    text = content[0].get("text", "")[:200]
                    console.print(f"[dim]Response preview:[/dim] {text}...")
            else:
                console.print(f"[yellow]Response:[/yellow] {result}")
            console.print()

    finally:
        await client.close()


async def demo_projection():
    """Demo 3: Output Projection - extract specific fields."""
    print_demo_header(
        "Pattern 3: Output Projection",
        "Virtual tools can extract specific fields from complex responses.\n"
        "Example: 'list_entity_names' extracts just names from read_graph.\n"
        "This reduces token usage and simplifies agent processing."
    )

    client = MCPClient()
    try:
        await client.initialize()

        # First, create some entities
        console.print("[dim]Creating test entities in knowledge graph...[/dim]")
        await client.call_tool("create_entities", {
            "entities": [
                {"name": "Alice", "entityType": "person", "observations": ["Likes Python"]},
                {"name": "Bob", "entityType": "person", "observations": ["Likes Rust"]},
                {"name": "AgentGateway", "entityType": "project", "observations": ["MCP proxy"]}
            ]
        })

        # Compare full vs projected response
        console.print("\n[bold]Full response (read_graph):[/bold]")
        full_result = await client.call_tool("read_graph", {})
        print_json(full_result.get("result", {}), "read_graph response")

        console.print("\n[bold]Projected response (list_entity_names):[/bold]")
        projected_result = await client.call_tool("list_entity_names", {})
        print_json(projected_result.get("result", {}), "list_entity_names response")

        console.print("\n[green]Notice:[/green] Projected response contains only the entity names!")

    finally:
        await client.close()


async def demo_transformation():
    """Demo 4: Output Transformation - restructure responses."""
    print_demo_header(
        "Pattern 4: Output Transformation",
        "Virtual tools can reshape and flatten nested JSON responses.\n"
        "Example: 'get_time_structured' extracts nested fields into flat object.\n"
        "This normalizes outputs across different backend servers."
    )

    client = MCPClient()
    try:
        await client.initialize()

        console.print("[bold]Raw time response (get_current_time):[/bold]")
        raw_result = await client.call_tool("get_current_time", {"timezone": "America/Los_Angeles"})
        print_json(raw_result.get("result", {}), "get_current_time response")

        console.print("\n[bold]Transformed response (get_time_structured):[/bold]")
        transformed_result = await client.call_tool("get_time_structured", {"timezone": "America/Los_Angeles"})
        print_json(transformed_result.get("result", {}), "get_time_structured response")

        console.print("\n[bold]Minimal projection (what_day_is_it):[/bold]")
        minimal_result = await client.call_tool("what_day_is_it", {"timezone": "America/Los_Angeles"})
        print_json(minimal_result.get("result", {}), "what_day_is_it response")

    finally:
        await client.close()


async def demo_composition():
    """Demo 5: Tool Composition - pipeline multiple tools."""
    print_demo_header(
        "Pattern 5: Tool Composition (Pipeline)",
        "Virtual tools can chain multiple tools into a single operation.\n"
        "Example: 'fetch_and_remember' fetches URL then stores in knowledge graph.\n"
        "This enables complex workflows with a single tool call."
    )

    client = MCPClient()
    try:
        await client.initialize()

        console.print("[bold]Calling pipeline tool (fetch_and_remember):[/bold]")
        console.print("[dim]This will: 1) Fetch URL content, 2) Store in knowledge graph[/dim]\n")

        result = await client.call_tool("fetch_and_remember", {"url": "https://example.com"})
        print_json(result.get("result", {}), "fetch_and_remember response")

        # Verify it was stored
        console.print("\n[bold]Verifying entity was created:[/bold]")
        graph = await client.call_tool("read_graph", {})
        entities = graph.get("result", {}).get("content", [])
        if entities:
            text = entities[0].get("text", "")
            if "fetched_page" in text:
                console.print("[green]Success![/green] Pipeline created entity in knowledge graph.")

    finally:
        await client.close()


async def demo_a2a():
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
        async with httpx.AsyncClient(timeout=5.0) as client:
            response = await client.get(f"{A2A_ENDPOINT}/.well-known/agent.json")
            if response.status_code == 200:
                agent_card = response.json()
                print_json(agent_card, "A2A Agent Card")
                console.print("\n[green]A2A agent is reachable through the gateway![/green]")
                console.print(f"[dim]Notice: URL rewritten to point to gateway: {A2A_ENDPOINT}[/dim]")
            else:
                console.print(f"[red]A2A agent returned status {response.status_code}[/red]")
    except Exception as e:
        console.print(f"[red]Could not reach A2A agent: {e}[/red]")
        console.print("[dim]Make sure an A2A agent is running on port 9999[/dim]")


DEMOS = {
    "multiplexing": ("MCP Multiplexing", demo_multiplexing),
    "aliasing": ("Tool Aliasing", demo_aliasing),
    "projection": ("Output Projection", demo_projection),
    "transformation": ("Output Transformation", demo_transformation),
    "composition": ("Tool Composition", demo_composition),
    "a2a": ("A2A Proxy", demo_a2a),
}


async def run_all_demos():
    """Run all demos in sequence."""
    for name, (title, demo_fn) in DEMOS.items():
        await demo_fn()
        if name != list(DEMOS.keys())[-1]:
            console.print("\n" + "=" * 60 + "\n")
            if not Confirm.ask("Continue to next demo?", default=True):
                break


async def interactive_menu():
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
        console.print("\n[red]Gateway not reachable at {GATEWAY_URL}[/red]")
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
            await run_all_demos()
        elif choice.isdigit():
            idx = int(choice) - 1
            if 0 <= idx < len(DEMOS):
                key = list(DEMOS.keys())[idx]
                _, demo_fn = DEMOS[key]
                await demo_fn()
        elif choice in DEMOS:
            _, demo_fn = DEMOS[choice]
            await demo_fn()

        console.print()


def main():
    parser = argparse.ArgumentParser(description="AgentGateway Pattern Demos")
    parser.add_argument("--demo", choices=list(DEMOS.keys()), help="Run specific demo")
    parser.add_argument("--list", action="store_true", help="List available demos")
    parser.add_argument("--all", action="store_true", help="Run all demos")
    parser.add_argument("--url", default=GATEWAY_URL, help="Gateway URL")
    args = parser.parse_args()

    global GATEWAY_URL, MCP_ENDPOINT, SSE_ENDPOINT, A2A_ENDPOINT
    GATEWAY_URL = args.url
    MCP_ENDPOINT = f"{GATEWAY_URL}/mcp"
    SSE_ENDPOINT = f"{GATEWAY_URL}/sse"
    A2A_ENDPOINT = f"{GATEWAY_URL}/a2a"

    if args.list:
        console.print("[bold]Available demos:[/bold]")
        for key, (title, _) in DEMOS.items():
            console.print(f"  {key}: {title}")
        return

    if args.demo:
        _, demo_fn = DEMOS[args.demo]
        asyncio.run(demo_fn())
    elif args.all:
        asyncio.run(run_all_demos())
    else:
        asyncio.run(interactive_menu())


if __name__ == "__main__":
    main()
