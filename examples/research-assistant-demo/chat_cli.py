#!/usr/bin/env python3
"""Simple CLI chat client for the Research Agent.

Usage:
    python chat_cli.py [--agent-url URL] [--user USER]

Examples:
    python chat_cli.py
    python chat_cli.py --agent-url http://localhost:9001
"""

import argparse
import httpx
import json
import os
import sys

# ANSI colors
GREEN = "\033[92m"
BLUE = "\033[94m"
YELLOW = "\033[93m"
RED = "\033[91m"
RESET = "\033[0m"
BOLD = "\033[1m"
DIM = "\033[2m"

# Log file locations to check for errors
LOG_PATHS = [
    "logs/agent.log",  # When started via start_services.sh
    "/tmp/agent.log",  # When started manually
]


def get_recent_error_from_logs() -> str | None:
    """Try to extract the most recent error from agent logs."""
    script_dir = os.path.dirname(os.path.abspath(__file__))

    for log_path in LOG_PATHS:
        full_path = os.path.join(script_dir, log_path) if not log_path.startswith("/") else log_path
        if os.path.exists(full_path):
            try:
                with open(full_path, "r") as f:
                    lines = f.readlines()
                    # Look for error patterns in last 50 lines
                    recent = lines[-50:] if len(lines) > 50 else lines
                    for line in reversed(recent):
                        # Look for common error patterns
                        if "AnthropicException" in line or "credit balance" in line:
                            # Extract the JSON error message
                            if '"message":' in line:
                                start = line.find('"message":"') + 11
                                end = line.find('"', start)
                                if start > 10 and end > start:
                                    return line[start:end]
                        if "RateLimitError" in line:
                            return "Rate limit exceeded - wait a minute and try again"
                        if "BadRequestError" in line and "credit" in line.lower():
                            return "Anthropic API credits exhausted - add credits or switch models"
            except Exception:
                pass
    return None


def create_session(client: httpx.Client, agent_url: str, user_id: str) -> str:
    """Create a new ADK session and return the session ID."""
    url = f"{agent_url}/apps/research_agent/users/{user_id}/sessions"
    resp = client.post(url, json={})
    resp.raise_for_status()
    return resp.json()["id"]


def send_message(
    client: httpx.Client, agent_url: str, user_id: str, session_id: str, message: str
) -> str:
    """Send a message and return the agent's response."""
    url = f"{agent_url}/run"
    payload = {
        "appName": "research_agent",
        "userId": user_id,
        "sessionId": session_id,
        "newMessage": {"role": "user", "parts": [{"text": message}]},
    }

    resp = client.post(url, json=payload, timeout=120.0)
    resp.raise_for_status()

    events = resp.json()

    # Extract response from events
    response_parts = []
    tool_calls = []

    for event in events:
        content = event.get("content", {})
        parts = content.get("parts", [])

        for part in parts:
            if "functionCall" in part:
                fc = part["functionCall"]
                tool_calls.append(f"{fc.get('name', 'unknown')}")

            if "functionResponse" in part:
                fr = part["functionResponse"]
                resp_data = fr.get("response", {})
                sc = resp_data.get("structuredContent", {})
                if "results" in sc:
                    tool_calls.append(f"  -> {len(sc['results'])} results")

            if "text" in part:
                text = part["text"]
                if text and len(text.strip()) > 0:
                    response_parts.append(text)

    # Build output
    output = []
    if tool_calls:
        output.append(f"{YELLOW}Tools: {', '.join(tool_calls)}{RESET}")
    if response_parts:
        output.append(response_parts[-1])  # Last text is usually the final response

    return "\n".join(output) if output else "(no response)"


def main():
    parser = argparse.ArgumentParser(description="Chat with the Research Agent")
    parser.add_argument(
        "--agent-url",
        default="http://localhost:9001",
        help="Agent URL (default: http://localhost:9001)",
    )
    parser.add_argument(
        "--user", default="cli_user", help="User ID (default: cli_user)"
    )
    args = parser.parse_args()

    print(f"{BOLD}Research Agent CLI{RESET}")
    print(f"Agent: {args.agent_url}")
    print(f"Type 'quit' or 'exit' to quit, 'new' for new session")
    print("-" * 50)

    client = httpx.Client()

    try:
        # Create initial session
        print(f"{BLUE}Creating session...{RESET}", end=" ", flush=True)
        session_id = create_session(client, args.agent_url, args.user)
        print(f"{GREEN}OK{RESET} (session: {session_id[:8]}...)")
        print()

        while True:
            try:
                user_input = input(f"{GREEN}You: {RESET}").strip()
            except EOFError:
                break

            if not user_input:
                continue

            if user_input.lower() in ("quit", "exit", "q"):
                print("Goodbye!")
                break

            if user_input.lower() == "new":
                print(f"{BLUE}Creating new session...{RESET}", end=" ", flush=True)
                session_id = create_session(client, args.agent_url, args.user)
                print(f"{GREEN}OK{RESET} (session: {session_id[:8]}...)")
                continue

            try:
                print(f"{BLUE}Agent: {RESET}", end="", flush=True)
                response = send_message(
                    client, args.agent_url, args.user, session_id, user_input
                )
                # Clear the "Agent: " prefix and print response
                print(f"\r{BLUE}Agent:{RESET} {response}")
            except httpx.HTTPStatusError as e:
                print(f"\r{RED}Error: HTTP {e.response.status_code}{RESET}")
                # Try to get actual error from logs since ADK returns generic 500
                if e.response.status_code == 500:
                    log_error = get_recent_error_from_logs()
                    if log_error:
                        print(f"  {RED}{log_error}{RESET}")
                    else:
                        print(f"  {e.response.text[:200]}")
                        print(f"  {DIM}Check logs/agent.log or /tmp/agent.log for details{RESET}")
                else:
                    print(f"  {e.response.text[:200]}")
            except httpx.RequestError as e:
                print(f"\r{RED}Error: {e}{RESET}")

            print()

    except httpx.RequestError as e:
        print(f"{RED}Failed to connect to agent: {e}{RESET}")
        sys.exit(1)
    except KeyboardInterrupt:
        print("\nGoodbye!")
    finally:
        client.close()


if __name__ == "__main__":
    main()
