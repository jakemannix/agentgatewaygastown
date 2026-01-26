"""HTTP runner for MCP servers using Streamable HTTP transport."""

import argparse
import asyncio
import logging
import signal
import sys
from typing import TYPE_CHECKING

import uvicorn
from mcp.server.fastmcp import FastMCP

if TYPE_CHECKING:
    pass

logger = logging.getLogger(__name__)


def run_http_server(mcp: FastMCP, default_port: int = 8000):
    """Run the MCP server with Streamable HTTP transport.

    Args:
        mcp: The FastMCP server instance
        default_port: Default port if not specified via CLI
    """
    parser = argparse.ArgumentParser(description=f"Run {mcp.name} MCP Server")
    parser.add_argument("--port", type=int, default=default_port, help="Port to run on")
    parser.add_argument("--host", type=str, default="0.0.0.0", help="Host to bind to")
    args = parser.parse_args()

    # Configure logging
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
    )

    logger.info(f"Starting {mcp.name} on {args.host}:{args.port}")

    # Create ASGI app with Streamable HTTP transport
    app = mcp.streamable_http_app()

    # Run with uvicorn
    config = uvicorn.Config(
        app,
        host=args.host,
        port=args.port,
        log_level="info",
    )
    server = uvicorn.Server(config)

    # Handle shutdown gracefully
    def handle_shutdown(signum, frame):
        logger.info("Shutting down...")
        sys.exit(0)

    signal.signal(signal.SIGINT, handle_shutdown)
    signal.signal(signal.SIGTERM, handle_shutdown)

    asyncio.run(server.serve())
