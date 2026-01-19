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


def build_system_prompt(scenario: str = "full_workflow") -> str:
    """Build the system prompt for the agent based on the scenario.

    Args:
        scenario: The demo scenario to run. Options:
            - 'full_workflow': Use all custom services together
            - 'document_search': Focus on document service
            - 'task_management': Focus on task service
            - 'user_collaboration': Focus on user + notification services

    Returns:
        System prompt string.
    """
    base_prompt = """You are an autonomous ReAct agent that uses tools to accomplish tasks.

For each step, you should:
1. THINK: Analyze what you need to do next
2. ACT: Choose and execute the appropriate tool
3. OBSERVE: Review the tool results
4. REPEAT: Continue until the task is complete

Always explain your reasoning before taking actions.

You have access to tools from AgentGateway, including:
- Document management with semantic search (create_document, search_documents)
- Task/todo management with history tracking (create_task, list_tasks, complete_task)
- User profiles with bio-based search (list_users, search_users_by_bio)
- Notifications across channels (send_notification, get_notifications)
- Web fetching and knowledge graph tools
- Virtual tools that compose multiple operations"""

    scenario_prompts = {
        "full_workflow": """
Your task is to demonstrate a complete workflow across multiple services:
1. Create or search for relevant documents
2. Create tasks based on document content
3. Find appropriate users to assign tasks
4. Send notifications about new assignments

Be thorough and show how the services work together.""",

        "document_search": """
Your task is to work with the document service:
- Create documents with meaningful content
- Use semantic search to find relevant documents
- Demonstrate how chunking and embeddings enable intelligent search

Focus on showing the power of semantic search vs keyword matching.""",

        "task_management": """
Your task is to demonstrate task management capabilities:
- Create tasks with different priorities
- List and filter tasks by status
- Complete tasks and track state transitions
- Show the audit history of task changes

Focus on workflow patterns and state management.""",

        "user_collaboration": """
Your task is to demonstrate user and notification features:
- List available users and their profiles
- Search for users by interests (bio-based semantic search)
- Send notifications to relevant users
- Check notification delivery status

Focus on collaboration and communication patterns.""",
    }

    scenario_addition = scenario_prompts.get(scenario, scenario_prompts["full_workflow"])
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
        system_prompt=build_system_prompt("full_workflow"),
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


DEMO_PROMPTS = {
    "full_workflow": """Demonstrate a complete workflow:
1. First, create a document titled "Project Requirements" with content about building a REST API
2. Create a high-priority task to "Review API requirements" and assign it to any available user
3. Send a notification to that user about their new task assignment
4. Finally, search for documents related to "API" to verify it was created""",

    "document_search": """Work with the document service:
1. Create three short documents about different programming topics (Python, JavaScript, Rust)
2. Use semantic search to find documents about "systems programming" (should find Rust)
3. Search for "web development" (should find JavaScript)
4. List all documents to show what was created""",

    "task_management": """Demonstrate task management:
1. Create a task "Set up CI/CD pipeline" with high priority
2. Create a task "Write unit tests" with medium priority  
3. Create a task "Update documentation" with low priority
4. List all pending tasks sorted by priority
5. Complete the documentation task
6. List tasks again to show the status change""",

    "user_collaboration": """Demonstrate user and notification features:
1. List all available users and their bios
2. Search for users interested in "machine learning" or "AI"
3. Send an in-app notification to one of them about a new project
4. Send an email notification to another user
5. Check the notification queue to see pending deliveries""",
}


async def main():
    """Main entry point for the demo."""
    import argparse

    parser = argparse.ArgumentParser(
        description="Claude Agent SDK ReAct agent demo with AgentGateway",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Run the full workflow demo (default)
  python agent.py --scenario full_workflow

  # Run a specific scenario  
  python agent.py --scenario document_search
  python agent.py --scenario task_management
  python agent.py --scenario user_collaboration

  # Run with a custom prompt
  python agent.py --prompt "Create a document about Python best practices"

  # Connect to a different gateway
  python agent.py --gateway-url http://gateway.example.com:3000

Scenarios:
  full_workflow      - Complete demo using all services together
  document_search    - Focus on document service with semantic search
  task_management    - Focus on task service with state tracking
  user_collaboration - Focus on users and notifications

Environment variables:
  AGENTGATEWAY_URL         Gateway URL (default: http://localhost:3000)
  AGENTGATEWAY_AUTH_TOKEN  Bearer token for authentication
""",
    )

    parser.add_argument(
        "--scenario",
        "-s",
        choices=list(DEMO_PROMPTS.keys()),
        default=None,
        help="Run a predefined demo scenario",
    )
    parser.add_argument(
        "--prompt",
        "-p",
        default=None,
        help="Custom task prompt (overrides --scenario)",
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
    parser.add_argument(
        "--list-scenarios",
        action="store_true",
        help="List available scenarios and exit",
    )

    args = parser.parse_args()

    if args.list_scenarios:
        print("Available scenarios:\n")
        for name, prompt in DEMO_PROMPTS.items():
            print(f"  {name}:")
            print(f"    {prompt[:80]}...")
            print()
        return 0

    # Determine prompt: custom > scenario > default
    if args.prompt:
        prompt = args.prompt
    elif args.scenario:
        prompt = DEMO_PROMPTS[args.scenario]
    else:
        prompt = DEMO_PROMPTS["full_workflow"]

    print("=" * 60)
    print("Claude Agent SDK ReAct Demo")
    print("=" * 60)
    print(f"\nPrompt: {prompt[:100]}{'...' if len(prompt) > 100 else ''}")
    print(f"Gateway: {args.gateway_url or os.environ.get('AGENTGATEWAY_URL', DEFAULT_GATEWAY_URL)}")
    print("-" * 60)

    result = await run_agent(
        prompt=prompt,
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
