"""
Modal deployment for agentgateway.

This module provides a serverless deployment of agentgateway on Modal.com,
enabling you to run the gateway as a scalable, managed service.

Usage:
    # Deploy to Modal
    modal deploy modal/modal_app.py

    # Run locally for testing
    modal serve modal/modal_app.py

Environment Variables (set via Modal Secrets):
    AGENTGATEWAY_CONFIG: Full YAML configuration for agentgateway
    AGENTGATEWAY_CONFIG_FILE: Path to config file (if bundled in image)

For more information, see the README.md in this directory.
"""

import os
import subprocess
from pathlib import Path

import modal

# App configuration
APP_NAME = "agentgateway"
DEFAULT_PORT = 3000

# Create the Modal app
app = modal.App(APP_NAME)

# Build the image from the project's Dockerfile
# This uses the existing multi-stage build for optimal image size
dockerfile_image = modal.Image.from_dockerfile(
    path=Path(__file__).parent.parent / "Dockerfile",
    context_mount=modal.Mount.from_local_dir(
        Path(__file__).parent.parent,
        remote_path="/build-context",
        # Exclude unnecessary files from build context
        condition=lambda path: not any(
            pattern in path
            for pattern in [".git", "target", "node_modules", ".cargo/registry"]
        ),
    ),
    # Use amd64 for better compatibility with Modal's infrastructure
    platform="linux/amd64",
)

# Alternative: Use a pre-built image from a registry
# Uncomment and modify if you prefer using a pre-built image
# prebuilt_image = modal.Image.from_registry(
#     "ghcr.io/agentgateway/agentgateway:latest"
# )


def get_config() -> str:
    """
    Get the agentgateway configuration from environment or defaults.

    Priority:
    1. AGENTGATEWAY_CONFIG environment variable (full YAML config)
    2. AGENTGATEWAY_CONFIG_FILE environment variable (path to config file)
    3. Default minimal configuration
    """
    if config := os.environ.get("AGENTGATEWAY_CONFIG"):
        return config

    config_file = os.environ.get("AGENTGATEWAY_CONFIG_FILE")
    if config_file and Path(config_file).exists():
        return Path(config_file).read_text()

    # Default configuration - minimal setup for testing
    return f"""
# Default agentgateway configuration for Modal deployment
# Override by setting AGENTGATEWAY_CONFIG in Modal Secrets
binds:
- port: {DEFAULT_PORT}
  listeners:
  - routes:
    - policies:
        cors:
          allowOrigins:
          - "*"
          allowHeaders:
          - mcp-protocol-version
          - content-type
          - cache-control
      backends:
      - mcp:
          targets: []
"""


@app.function(
    image=dockerfile_image,
    # Secrets can be created via: modal secret create agentgateway-config AGENTGATEWAY_CONFIG="..."
    secrets=[modal.Secret.from_name("agentgateway-config", required=False)],
    # Container configuration
    cpu=1.0,  # Adjust based on your needs
    memory=512,  # MB, adjust based on your needs
    # Keep container warm for faster cold starts (optional, incurs cost)
    # keep_warm=1,
    # Concurrency settings
    allow_concurrent_inputs=100,  # agentgateway handles concurrency well
    # Timeout for the container
    timeout=300,
)
@modal.web_server(port=DEFAULT_PORT, startup_timeout=60)
def serve():
    """
    Run agentgateway as a Modal web server.

    The gateway will be accessible at your Modal deployment URL.
    Configure MCP targets and other settings via the AGENTGATEWAY_CONFIG secret.
    """
    config = get_config()

    # Write config to a temporary file
    config_path = Path("/tmp/agentgateway-config.yaml")
    config_path.write_text(config)

    # Start agentgateway
    subprocess.Popen(
        ["/app/agentgateway", "-f", str(config_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )


# Alternative entry point for ASGI if needed in the future
# @app.function(image=dockerfile_image)
# @modal.asgi_app()
# def asgi_app():
#     """Alternative ASGI-based deployment."""
#     pass


@app.local_entrypoint()
def main():
    """
    Local entrypoint for testing and development.

    Run with: modal run modal/modal_app.py
    """
    print(f"Agentgateway Modal deployment ready!")
    print(f"Deploy with: modal deploy modal/modal_app.py")
    print(f"Serve locally with: modal serve modal/modal_app.py")
