"""LangGraph ReAct Agent demo with agentgateway MCP integration.

Demo scenario: Research and summarize, then notify team.

This demonstrates:
- Connecting to agentgateway MCP endpoint
- Dynamic tool discovery and loading
- ReAct reasoning pattern with state management
- Tool orchestration across research, summarization, and notification
- Execution visualization
"""

from __future__ import annotations

import argparse
import asyncio
import logging
import os
import sys

from rich.console import Console
from rich.panel import Panel

from .agent import create_react_agent, run_agent, arun_agent
from .mcp_client import MCPClient
from .tools import MCPToolProvider
from .visualization import (
    visualize_trace,
    visualize_state,
    visualize_graph_structure,
    print_step_progress,
)


console = Console()
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)


# Demo scenario prompt
DEMO_PROMPT = """Research the latest developments in AI agents and tool use,
summarize the key findings, and prepare a brief notification for the team.

Focus on:
1. Recent advances in agent architectures
2. Tool orchestration patterns
3. State management approaches

After researching and summarizing, format a team notification with the highlights."""


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="LangGraph ReAct Agent with agentgateway MCP integration"
    )
    parser.add_argument(
        "--gateway-url",
        default=os.getenv("AGENTGATEWAY_URL", "http://localhost:3000"),
        help="URL of the agentgateway endpoint (default: http://localhost:3000)",
    )
    parser.add_argument(
        "--model",
        default=os.getenv("ANTHROPIC_MODEL", "claude-sonnet-4-20250514"),
        help="Anthropic model to use (default: claude-sonnet-4-20250514)",
    )
    parser.add_argument(
        "--prompt",
        default=None,
        help="Custom prompt (default: demo scenario)",
    )
    parser.add_argument(
        "--max-iterations",
        type=int,
        default=10,
        help="Maximum reasoning iterations (default: 10)",
    )
    parser.add_argument(
        "--show-graph",
        action="store_true",
        help="Show graph structure visualization",
    )
    parser.add_argument(
        "--save-graph",
        type=str,
        default=None,
        help="Save graph visualization to file (PNG)",
    )
    parser.add_argument(
        "--async",
        dest="use_async",
        action="store_true",
        help="Use async execution",
    )
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Enable verbose output",
    )
    return parser.parse_args()


def main() -> int:
    """Main entry point for the demo."""
    args = parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    console.print(
        Panel(
            "[bold blue]LangGraph ReAct Agent[/bold blue]\n"
            "[dim]Connecting to agentgateway MCP endpoint[/dim]",
            border_style="blue",
        )
    )

    # Connect to agentgateway
    console.print(f"\n[cyan]Connecting to gateway:[/cyan] {args.gateway_url}")

    try:
        with MCPClient(args.gateway_url) as mcp_client:
            # Load tools from MCP endpoint
            console.print("[cyan]Loading tools from MCP endpoint...[/cyan]")
            tool_provider = MCPToolProvider(mcp_client)
            tools = tool_provider.load_tools()

            if not tools:
                console.print("[yellow]Warning: No tools loaded from MCP endpoint[/yellow]")
                console.print("[dim]The agent will run without tools[/dim]")
            else:
                console.print(f"[green]Loaded {len(tools)} tools:[/green]")
                for tool in tools:
                    console.print(f"  • {tool.name}: {tool.description[:60]}...")

            # Create the agent
            console.print(f"\n[cyan]Creating ReAct agent with model:[/cyan] {args.model}")
            graph, trace = create_react_agent(
                tools=tools,
                model=args.model,
                max_iterations=args.max_iterations,
            )

            # Show graph structure if requested
            if args.show_graph or args.save_graph:
                visualize_graph_structure(graph, args.save_graph)

            # Run the agent
            prompt = args.prompt or DEMO_PROMPT
            console.print("\n[bold]Running agent with prompt:[/bold]")
            console.print(Panel(prompt.strip(), border_style="cyan"))

            console.print("\n[bold cyan]Agent Execution:[/bold cyan]\n")

            if args.use_async:
                response, final_state = asyncio.run(arun_agent(graph, prompt, trace))
            else:
                response, final_state = run_agent(graph, prompt, trace)

            # Visualize results
            console.print("\n")
            visualize_trace(trace)
            console.print("\n")
            visualize_state(final_state)

            # Show final response
            console.print("\n")
            print_step_progress("final", response)

            return 0

    except ConnectionError as e:
        console.print(f"[red]Connection error:[/red] {e}")
        console.print(
            "\n[yellow]Make sure agentgateway is running:[/yellow]\n"
            "  cargo run -- -f examples/basic/config.yaml"
        )
        return 1
    except Exception as e:
        console.print(f"[red]Error:[/red] {e}")
        if args.verbose:
            import traceback
            traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
