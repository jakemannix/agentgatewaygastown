"""
Google ADK Agent - Multi-step Project Setup with Saga Pattern

This module demonstrates running the Google ADK agent either:
1. Standalone with ADK CLI (adk run/adk web)
2. With agentgateway tools integration

Usage:
    # Run standalone with ADK CLI
    adk run .

    # Run with ADK web interface
    adk web .

    # Run as Python module with gateway integration
    python -m google_adk_saga_agent --gateway-url http://localhost:3000
"""

import argparse
import asyncio
import logging
import os
import sys

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)


def run_with_gateway(gateway_url: str) -> None:
    """
    Run the agent with gateway tools integration.

    This demonstrates how to enhance the ADK agent with tools
    from an agentgateway instance.
    """
    from .agent import (
        create_coordinator_agent,
        create_project_structure,
        initialize_git,
        create_config,
        setup_dependencies,
        execute_saga,
        get_saga_status,
    )
    from .gateway_tools import (
        AgentGatewayMCPClient,
        discover_gateway_tools,
    )

    async def main() -> None:
        # Check gateway connectivity
        client = AgentGatewayMCPClient(gateway_url=gateway_url)
        try:
            tools = await client.list_tools()
            logger.info(f"Connected to gateway, found {len(tools)} tools")
            for tool in tools:
                logger.info(f"  - {tool.name}: {tool.description or 'No description'}")
        except Exception as e:
            logger.warning(f"Could not connect to gateway: {e}")
            logger.info("Running in standalone mode")

        # Run a demo saga
        logger.info("Starting project setup saga demo...")

        project_name = "demo-project"

        # Execute saga steps
        logger.info("Step 1: Creating project structure...")
        result = create_project_structure(project_name, "python")
        logger.info(f"  Result: {result['status']}")

        if result["status"] == "success":
            logger.info("Step 2: Initializing git...")
            result = initialize_git(project_name)
            logger.info(f"  Result: {result['status']}")

        if result["status"] == "success":
            logger.info("Step 3: Creating config...")
            result = create_config(project_name, "Demo Author", "A demo project")
            logger.info(f"  Result: {result['status']}")

        if result["status"] == "success":
            logger.info("Step 4: Setting up dependencies...")
            result = setup_dependencies(project_name, ["pytest", "httpx", "pydantic"])
            logger.info(f"  Result: {result['status']}")

        # Check saga status
        status = get_saga_status(project_name)
        logger.info(f"Saga status: {status}")

        # Execute saga completion
        final = execute_saga(project_name)
        logger.info(f"Final saga result: {final}")

    asyncio.run(main())


def run_interactive() -> None:
    """
    Run an interactive session with the coordinator agent.

    This is a simple REPL for testing the agent locally.
    """
    from .agent import create_coordinator_agent

    agent = create_coordinator_agent()

    print("\nGoogle ADK Saga Agent - Interactive Mode")
    print("=" * 50)
    print("This agent coordinates multi-step project setup with saga pattern.")
    print("Commands: 'quit' to exit, 'status <project>' to check saga status")
    print()

    # Note: Full interactive mode would require ADK's runner
    # This is a simplified demo
    print("For full interactive mode, use: adk run .")
    print("Or for web interface: adk web .")


def main() -> None:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Google ADK Agent with Saga Pattern for Project Setup"
    )
    parser.add_argument(
        "--gateway-url",
        default=os.environ.get("AGENTGATEWAY_URL", "http://localhost:3000"),
        help="URL of the agentgateway instance",
    )
    parser.add_argument(
        "--interactive",
        action="store_true",
        help="Run in interactive mode",
    )
    parser.add_argument(
        "--demo",
        action="store_true",
        help="Run a demo saga execution",
    )

    args = parser.parse_args()

    if args.demo:
        run_with_gateway(args.gateway_url)
    elif args.interactive:
        run_interactive()
    else:
        # Default: show help and run demo
        parser.print_help()
        print("\nRunning demo mode by default...\n")
        run_with_gateway(args.gateway_url)


if __name__ == "__main__":
    main()
