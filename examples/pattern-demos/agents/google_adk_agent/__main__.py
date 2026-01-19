"""
Google ADK Agent - Multi-step Project Setup with Saga Pattern

This module demonstrates running the Google ADK agent with agentgateway.

Usage:
    # Interactive CLI chat (recommended)
    adk run .

    # Web interface with chat UI
    adk web .

    # Run a one-shot demo with gateway integration
    python -m google_adk_agent --demo

    # Connect to a different gateway
    python -m google_adk_agent --demo --gateway-url http://gateway:3000

    # Use a specific LLM provider
    python -m google_adk_agent --demo --llm-provider anthropic

The ADK CLI provides the best interactive experience with built-in
multi-turn conversation support, tool execution visualization, and
session management.
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


def display_provider_info() -> None:
    """Display information about the configured LLM provider."""
    from .agent import get_provider_info

    info = get_provider_info()
    print(f"\nLLM Provider: {info['provider']}")
    print(f"Model: {info['model']}")
    print(f"Detection: {info['method']}")
    print()


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

    # Display configured provider
    display_provider_info()

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


def run_interactive(web: bool = False) -> None:
    """
    Launch interactive mode using ADK's built-in CLI or web interface.

    Args:
        web: If True, launch web interface. Otherwise launch CLI.
    """
    import subprocess

    # Get the directory containing this module
    module_dir = os.path.dirname(os.path.abspath(__file__))

    cmd = ["adk", "web" if web else "run", module_dir]

    print(f"\nLaunching: {' '.join(cmd)}")
    print("=" * 50)

    if web:
        print("Opening web interface...")
        print("Press Ctrl+C to stop the server.")
    else:
        print("Starting interactive chat...")
        print("Type your messages and press Enter. Type 'quit' to exit.")

    print()

    try:
        subprocess.run(cmd, check=True)
    except FileNotFoundError:
        print("Error: 'adk' command not found.")
        print("Install Google ADK with: pip install google-adk")
    except KeyboardInterrupt:
        print("\n[Stopped]")


def main() -> None:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Google ADK Agent with Saga Pattern for Project Setup",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Interactive CLI chat (recommended)
  python -m google_adk_agent --chat
  # Or directly: adk run .

  # Web interface
  python -m google_adk_agent --web
  # Or directly: adk web .

  # One-shot demo
  python -m google_adk_agent --demo

  # Use specific LLM provider
  python -m google_adk_agent --demo --llm-provider anthropic
  python -m google_adk_agent --demo --llm-provider openai
  python -m google_adk_agent --demo --llm-provider google

  # Use a custom model
  python -m google_adk_agent --demo --llm-model anthropic/claude-3-5-sonnet-20241022

Environment Variables:
  ANTHROPIC_API_KEY  - Anthropic Claude API key (highest priority)
  OPENAI_API_KEY     - OpenAI API key (second priority)
  GOOGLE_API_KEY     - Google Gemini API key (lowest priority)
  LLM_PROVIDER       - Force provider: anthropic, openai, or google
  LLM_MODEL          - Override model string directly
""",
    )
    parser.add_argument(
        "--chat",
        "-c",
        action="store_true",
        help="Start interactive CLI chat (launches 'adk run')",
    )
    parser.add_argument(
        "--web",
        "-w",
        action="store_true",
        help="Start web interface (launches 'adk web')",
    )
    parser.add_argument(
        "--demo",
        "-d",
        action="store_true",
        help="Run a one-shot demo saga execution",
    )
    parser.add_argument(
        "--gateway-url",
        default=os.environ.get("AGENTGATEWAY_URL", "http://localhost:3000"),
        help="URL of the agentgateway instance",
    )
    parser.add_argument(
        "--llm-provider",
        choices=["anthropic", "openai", "google"],
        help="LLM provider to use (overrides auto-detection)",
    )
    parser.add_argument(
        "--llm-model",
        help="Specific model string to use (overrides provider default)",
    )
    parser.add_argument(
        "--show-provider",
        action="store_true",
        help="Show LLM provider info and exit",
    )

    args = parser.parse_args()

    # Set environment variables from CLI args (before any agent code runs)
    if args.llm_provider:
        os.environ["LLM_PROVIDER"] = args.llm_provider
    if args.llm_model:
        os.environ["LLM_MODEL"] = args.llm_model

    if args.show_provider:
        display_provider_info()
        sys.exit(0)
    elif args.chat:
        display_provider_info()
        run_interactive(web=False)
    elif args.web:
        display_provider_info()
        run_interactive(web=True)
    elif args.demo:
        run_with_gateway(args.gateway_url)
    else:
        # Default: show help
        parser.print_help()
        print("\nTip: Use --chat for interactive mode or --demo for a one-shot example.")


if __name__ == "__main__":
    main()
