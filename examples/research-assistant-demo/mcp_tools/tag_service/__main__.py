"""Entry point for tag service."""

import sys
import os

# Add parent to path for shared imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from mcp_tools.tag_service.server import mcp
from mcp_tools.shared.http_runner import run_http_server

if __name__ == "__main__":
    run_http_server(mcp, default_port=8005)
