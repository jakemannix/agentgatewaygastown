"""HTTP runner for MCP services using streamable-http transport."""

import argparse
import logging
import uvicorn
from mcp.server.fastmcp import FastMCP

logger = logging.getLogger(__name__)


def run_http_server(mcp: FastMCP, port: int, host: str = "0.0.0.0"):
    """Run an MCP server with streamable-http transport.

    Args:
        mcp: The FastMCP server instance
        port: Port to listen on
        host: Host to bind to (default 0.0.0.0)
    """
    # Get the ASGI app from FastMCP for streamable HTTP
    app = mcp.streamable_http_app()

    logger.info(f"Starting {mcp.name} on http://{host}:{port}")
    uvicorn.run(app, host=host, port=port, log_level="info")


def create_arg_parser(service_name: str, default_port: int) -> argparse.ArgumentParser:
    """Create standard argument parser for MCP services.

    Args:
        service_name: Name of the service for help text
        default_port: Default port for this service

    Returns:
        Configured ArgumentParser
    """
    parser = argparse.ArgumentParser(description=f"{service_name} MCP Service")
    parser.add_argument(
        "--port",
        type=int,
        default=default_port,
        help=f"Port to run on (default: {default_port})",
    )
    parser.add_argument(
        "--host",
        default="0.0.0.0",
        help="Host to bind to (default: 0.0.0.0)",
    )
    return parser
