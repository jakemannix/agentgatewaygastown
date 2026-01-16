"""A2A Test Client - Exercises the multi-agent delegation flow."""

import asyncio
import json
import sys
import uuid

import httpx

# Default gateway URL (Claude Delegator via AgentGateway)
GATEWAY_URL = "http://localhost:3000"


async def get_agent_card(url: str) -> dict:
    """Fetch the agent card from an A2A endpoint."""
    async with httpx.AsyncClient(timeout=10.0) as client:
        response = await client.get(f"{url}/.well-known/agent.json")
        response.raise_for_status()
        return response.json()


async def send_message(url: str, message: str) -> dict:
    """Send an A2A message to an agent."""
    request_id = str(uuid.uuid4().hex[:32])

    request_body = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "message/send",
        "params": {
            "message": {
                "role": "user",
                "content": [{"kind": "Text", "text": message}],
            },
        },
    }

    async with httpx.AsyncClient(timeout=60.0) as client:
        response = await client.post(url, json=request_body)
        response.raise_for_status()
        return response.json()


def print_separator(title: str):
    """Print a section separator."""
    print("\n" + "=" * 60)
    print(f"  {title}")
    print("=" * 60 + "\n")


async def run_demo():
    """Run the A2A multi-agent delegation demo."""
    print_separator("A2A Multi-Agent Delegation Demo")

    # Step 1: Fetch agent cards
    print("Step 1: Fetching Agent Cards\n")

    agents = [
        ("Claude Delegator (via Gateway)", GATEWAY_URL),
        ("LangGraph Processor", "http://localhost:3001"),
        ("Google ADK Specialist", "http://localhost:3002"),
    ]

    for name, url in agents:
        try:
            card = await get_agent_card(url)
            print(f"[OK] {name}")
            print(f"     Name: {card['name']}")
            print(f"     URL: {card['url']}")
            print(f"     Skills: {', '.join(s['name'] for s in card['skills'])}")
            print()
        except httpx.HTTPError as e:
            print(f"[FAIL] {name}: {e}")
            print(f"       Make sure the agent is running at {url}")
            print()

    # Step 2: Test direct handling (no delegation)
    print_separator("Test 1: Direct Handling (No Delegation)")

    try:
        message = "Hello, what can you do?"
        print(f"Message: {message}\n")

        response = await send_message(GATEWAY_URL, message)
        result = response.get("result", {})
        status = result.get("status", {})
        status_msg = status.get("message", {})
        content = status_msg.get("content", [])

        for part in content:
            if part.get("kind") == "Text":
                print(f"Response:\n{part['text']}")
    except httpx.HTTPError as e:
        print(f"Error: {e}")

    # Step 3: Test workflow delegation
    print_separator("Test 2: Workflow Delegation (Claude -> LangGraph)")

    try:
        message = "Process this workflow: transform the data and generate a report"
        print(f"Message: {message}\n")

        response = await send_message(GATEWAY_URL, message)
        result = response.get("result", {})
        status = result.get("status", {})
        status_msg = status.get("message", {})
        content = status_msg.get("content", [])

        for part in content:
            if part.get("kind") == "Text":
                print(f"Response:\n{part['text']}")
    except httpx.HTTPError as e:
        print(f"Error: {e}")

    # Step 4: Test full chain delegation
    print_separator("Test 3: Full Chain Delegation (Claude -> LangGraph -> ADK)")

    try:
        message = "Process this workflow: query BigQuery for sales data, run Vertex AI inference on the results, and store output in GCS"
        print(f"Message: {message}\n")

        response = await send_message(GATEWAY_URL, message)
        result = response.get("result", {})
        status = result.get("status", {})
        status_msg = status.get("message", {})
        content = status_msg.get("content", [])

        for part in content:
            if part.get("kind") == "Text":
                print(f"Response:\n{part['text']}")
    except httpx.HTTPError as e:
        print(f"Error: {e}")

    print_separator("Demo Complete")


def main():
    """Entry point."""
    try:
        asyncio.run(run_demo())
    except KeyboardInterrupt:
        print("\nDemo interrupted.")
        sys.exit(0)
    except httpx.ConnectError:
        print("\nError: Could not connect to agents.")
        print("Make sure all agents and the gateway are running:")
        print("  1. ADK Specialist:      cd agents/adk-specialist && uv run python -m adk_specialist")
        print("  2. LangGraph Processor: cd agents/langgraph-processor && uv run python -m langgraph_processor")
        print("  3. Claude Delegator:    cd agents/claude-delegator && uv run python -m claude_delegator")
        print("  4. AgentGateway:        cargo run -- -f examples/pattern-demos/a2a/config.yaml")
        sys.exit(1)


if __name__ == "__main__":
    main()
