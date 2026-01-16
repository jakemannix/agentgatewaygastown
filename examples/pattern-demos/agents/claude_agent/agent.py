#!/usr/bin/env python3
"""Claude Agent SDK ReAct agent that connects to agentgateway for virtual tools.

This agent demonstrates how to use the Claude Agent SDK to build an autonomous
agent that leverages tools provided by agentgateway via MCP.

The agent implements a ReAct-style (Reasoning + Acting) pattern where it:
1. Reasons about the task at hand
2. Selects appropriate tools from agentgateway
3. Executes actions and observes results
4. Iterates until the task is complete
"""

from __future__ import annotations

import asyncio
import os
import sys
from typing import Any

from claude_agent_sdk import (
    AssistantMessage,
    ClaudeAgentOptions,
    ResultMessage,
    SystemMessage,
    query,
)


# Default configuration
DEFAULT_GATEWAY_URL = "http://localhost:3000"
DEFAULT_GATEWAY_NAME = "agentgateway"


def get_gateway_config(
    gateway_url: str | None = None,
    gateway_name: str = DEFAULT_GATEWAY_NAME,
    auth_token: str | None = None,
) -> dict[str, Any]:
    """Build MCP server configuration for agentgateway.

    Args:
        gateway_url: URL of the agentgateway instance. Defaults to localhost:3000.
        gateway_name: Name to identify the MCP server. Defaults to 'agentgateway'.
        auth_token: Optional bearer token for authentication.

    Returns:
        MCP server configuration dictionary.
    """
    url = gateway_url or os.environ.get("AGENTGATEWAY_URL", DEFAULT_GATEWAY_URL)
    token = auth_token or os.environ.get("AGENTGATEWAY_AUTH_TOKEN")

    config: dict[str, Any] = {
        "type": "sse",
        "url": f"{url.rstrip('/')}/sse",
    }

    if token:
        config["headers"] = {"Authorization": f"Bearer {token}"}

    return {gateway_name: config}


def build_system_prompt(scenario: str = "document_tasks") -> str:
    """Build the system prompt for the agent based on the scenario.

    Args:
        scenario: The demo scenario to run. Options: 'document_tasks', 'custom'.

    Returns:
        System prompt string.
    """
    base_prompt = """You are an autonomous ReAct agent that uses tools to accomplish tasks.

For each step, you should:
1. THINK: Analyze what you need to do next
2. ACT: Choose and execute the appropriate tool
3. OBSERVE: Review the tool results
4. REPEAT: Continue until the task is complete

Always explain your reasoning before taking actions."""

    scenario_prompts = {
        "document_tasks": """
Your current task is to help find relevant documents and create actionable tasks from them.

When working with documents:
- Search for documents matching user criteria
- Extract key information and action items
- Create structured tasks with clear descriptions
- Prioritize tasks based on urgency or importance

Use the available tools from agentgateway to accomplish this.""",
    }

    scenario_addition = scenario_prompts.get(scenario, "")
    return base_prompt + scenario_addition


async def run_agent(
    prompt: str,
    gateway_url: str | None = None,
    auth_token: str | None = None,
    verbose: bool = True,
) -> str | None:
    """Run the ReAct agent with the given prompt.

    Args:
        prompt: The task or question for the agent.
        gateway_url: URL of the agentgateway instance.
        auth_token: Optional bearer token for authentication.
        verbose: Whether to print progress messages.

    Returns:
        The final result from the agent, or None if it failed.
    """
    mcp_servers = get_gateway_config(gateway_url, auth_token=auth_token)

    options = ClaudeAgentOptions(
        mcp_servers=mcp_servers,
        allowed_tools=["mcp__agentgateway__*"],  # Allow all tools from agentgateway
        system_prompt=build_system_prompt("document_tasks"),
        permission_mode="acceptEdits",
    )

    result = None

    async for message in query(prompt=prompt, options=options):
        # Check MCP server connection status
        if isinstance(message, SystemMessage) and message.subtype == "init":
            servers = message.data.get("mcp_servers", [])
            if verbose:
                for server in servers:
                    status = server.get("status", "unknown")
                    name = server.get("name", "unknown")
                    if status == "connected":
                        print(f"[Connected] MCP server: {name}")
                        tools = server.get("tools", [])
                        if tools:
                            print(f"  Available tools: {', '.join(t.get('name', '?') for t in tools[:5])}")
                            if len(tools) > 5:
                                print(f"  ... and {len(tools) - 5} more")
                    else:
                        print(f"[Failed] MCP server: {name} - {status}")

        # Print agent's reasoning and actions
        if isinstance(message, AssistantMessage):
            for block in message.content:
                if hasattr(block, "text") and block.text:
                    if verbose:
                        print(f"\n[Thinking] {block.text}")
                elif hasattr(block, "name"):
                    tool_name = block.name
                    if verbose:
                        print(f"\n[Action] Calling tool: {tool_name}")
                        if hasattr(block, "input"):
                            # Truncate long inputs for readability
                            input_str = str(block.input)
                            if len(input_str) > 200:
                                input_str = input_str[:200] + "..."
                            print(f"  Input: {input_str}")

        # Capture the final result
        if isinstance(message, ResultMessage):
            if message.subtype == "success":
                result = message.result
                if verbose:
                    print(f"\n[Complete] Task finished successfully")
            elif message.subtype == "error_during_execution":
                if verbose:
                    print(f"\n[Error] Task failed during execution")
            elif message.subtype == "end_turn":
                if verbose:
                    print(f"\n[End] Agent ended turn")

    return result


async def main():
    """Main entry point for the demo."""
    import argparse

    parser = argparse.ArgumentParser(
        description="Claude Agent SDK ReAct agent demo",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Run with default demo prompt (document tasks)
  python agent.py

  # Run with custom prompt
  python agent.py --prompt "List available tools and describe what they do"

  # Connect to a different gateway
  python agent.py --gateway-url http://gateway.example.com:3000

  # With authentication
  python agent.py --auth-token "your-bearer-token"

Environment variables:
  AGENTGATEWAY_URL         Gateway URL (default: http://localhost:3000)
  AGENTGATEWAY_AUTH_TOKEN  Bearer token for authentication
""",
    )

    parser.add_argument(
        "--prompt",
        "-p",
        default="Help me find relevant documents about API design and create tasks for implementing the suggestions.",
        help="The task prompt for the agent",
    )
    parser.add_argument(
        "--gateway-url",
        "-g",
        default=None,
        help="URL of the agentgateway instance (default: http://localhost:3000)",
    )
    parser.add_argument(
        "--auth-token",
        "-a",
        default=None,
        help="Bearer token for authentication",
    )
    parser.add_argument(
        "--quiet",
        "-q",
        action="store_true",
        help="Only print the final result",
    )

    args = parser.parse_args()

    print("=" * 60)
    print("Claude Agent SDK ReAct Demo")
    print("=" * 60)
    print(f"\nPrompt: {args.prompt}")
    print(f"Gateway: {args.gateway_url or os.environ.get('AGENTGATEWAY_URL', DEFAULT_GATEWAY_URL)}")
    print("-" * 60)

    result = await run_agent(
        prompt=args.prompt,
        gateway_url=args.gateway_url,
        auth_token=args.auth_token,
        verbose=not args.quiet,
    )

    if result:
        print("\n" + "=" * 60)
        print("Final Result:")
        print("=" * 60)
        print(result)

    return 0 if result else 1


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
