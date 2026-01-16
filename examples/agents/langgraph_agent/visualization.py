"""Visualization utilities for LangGraph execution."""

from __future__ import annotations

from typing import Any

from rich.console import Console
from rich.panel import Panel
from rich.table import Table
from rich.text import Text
from rich.tree import Tree

from .agent import ExecutionTrace


console = Console()


def visualize_trace(trace: ExecutionTrace) -> None:
    """Visualize an execution trace using rich.

    Displays a tree view of the agent's execution steps.
    """
    tree = Tree("[bold blue]Agent Execution Trace[/bold blue]")

    for i, step in enumerate(trace.steps, 1):
        step_type = step["type"]
        content = step["content"]

        if step_type == "input":
            node = tree.add(f"[bold green]{i}. User Input[/bold green]")
            node.add(Text(content[:200] + "..." if len(content) > 200 else content))

        elif step_type == "reasoning":
            node = tree.add(f"[bold cyan]{i}. Reasoning[/bold cyan]")
            if content:
                node.add(Text(content[:300] + "..." if len(content) > 300 else content))

        elif step_type == "tool_call":
            tool_name = step.get("tool_name", "unknown")
            tool_args = step.get("tool_args", {})
            node = tree.add(f"[bold yellow]{i}. Tool Call: {tool_name}[/bold yellow]")
            if content:
                node.add(f"[dim]Reasoning: {content[:150]}...[/dim]" if len(content) > 150 else f"[dim]Reasoning: {content}[/dim]")
            if tool_args:
                args_str = ", ".join(f"{k}={v!r}" for k, v in list(tool_args.items())[:3])
                if len(tool_args) > 3:
                    args_str += ", ..."
                node.add(f"[magenta]Args: {args_str}[/magenta]")

        elif step_type == "tool_result":
            tool_name = step.get("tool_name", "unknown")
            result = step.get("tool_result", "")
            node = tree.add(f"[bold green]{i}. Tool Result: {tool_name}[/bold green]")
            result_preview = result[:200] + "..." if len(str(result)) > 200 else result
            node.add(Text(str(result_preview)))

    console.print(tree)


def visualize_state(state: dict[str, Any]) -> None:
    """Visualize the current agent state.

    Displays a summary of the agent's state including step count,
    current phase, and recent tool calls.
    """
    table = Table(title="Agent State", show_header=True, header_style="bold magenta")
    table.add_column("Property", style="cyan")
    table.add_column("Value", style="green")

    table.add_row("Step Count", str(state.get("step_count", 0)))
    table.add_row("Current Phase", state.get("current_phase", "unknown"))

    tool_calls = state.get("tool_calls_made", [])
    table.add_row("Tools Called", str(len(tool_calls)))

    if tool_calls:
        recent_tools = ", ".join(tc["tool"] for tc in tool_calls[-3:])
        if len(tool_calls) > 3:
            recent_tools = "..." + recent_tools
        table.add_row("Recent Tools", recent_tools)

    console.print(table)


def visualize_graph_structure(graph: Any, output_path: str | None = None) -> str | None:
    """Visualize the LangGraph structure.

    If pygraphviz is available, generates a PNG visualization.
    Otherwise, returns an ASCII representation.

    Args:
        graph: The compiled LangGraph
        output_path: Optional path to save PNG visualization

    Returns:
        ASCII representation of the graph if PNG generation fails
    """
    try:
        # Try to generate PNG with mermaid
        png_bytes = graph.get_graph().draw_mermaid_png()
        if output_path:
            with open(output_path, "wb") as f:
                f.write(png_bytes)
            console.print(f"[green]Graph visualization saved to {output_path}[/green]")
            return None
    except Exception:
        pass

    # Fallback to ASCII representation
    ascii_graph = """
    ┌─────────────────────────────────────────────────────────┐
    │                   LangGraph ReAct Agent                  │
    └─────────────────────────────────────────────────────────┘
                              │
                              ▼
    ┌─────────────────────────────────────────────────────────┐
    │                      [START]                            │
    └─────────────────────────────────────────────────────────┘
                              │
                              ▼
    ┌─────────────────────────────────────────────────────────┐
    │                       agent                             │
    │  • Receives messages                                    │
    │  • Reasons about next action                            │
    │  • Decides: use tools or respond                        │
    └─────────────────────────────────────────────────────────┘
                              │
               ┌──────────────┴──────────────┐
               │ has_tool_calls?             │
               │                             │
               ▼                             ▼
    ┌─────────────────────┐    ┌─────────────────────────────┐
    │       tools         │    │           [END]             │
    │  • Execute MCP tool │    │  • Return final response    │
    │  • Return result    │    └─────────────────────────────┘
    └─────────────────────┘
               │
               └──────────────┐
                              ▼
                    (back to agent)
    """

    console.print(Panel(ascii_graph, title="Graph Structure", border_style="blue"))
    return ascii_graph


def print_step_progress(
    step_type: str,
    message: str,
    tool_name: str | None = None,
) -> None:
    """Print real-time progress during agent execution.

    Args:
        step_type: Type of step (reasoning, tool_call, tool_result)
        message: The message or content
        tool_name: Optional tool name for tool-related steps
    """
    if step_type == "reasoning":
        console.print(f"[cyan]💭 Reasoning:[/cyan] {message[:150]}...")
    elif step_type == "tool_call":
        console.print(f"[yellow]🔧 Calling tool:[/yellow] {tool_name}")
    elif step_type == "tool_result":
        console.print(f"[green]✅ Tool result:[/green] {message[:100]}...")
    elif step_type == "final":
        console.print(Panel(message, title="[bold green]Final Response[/bold green]", border_style="green"))
