"""
Modal deployment for agentgateway using a pre-built container image.

This is a simpler alternative to modal_app.py that uses a pre-built
container image instead of building from the Dockerfile. This results
in faster deployments but requires a published container image.

Usage:
    # Deploy to Modal
    modal deploy modal/modal_app_prebuilt.py

    # Run locally for testing
    modal serve modal/modal_app_prebuilt.py

Environment Variables (set via Modal Secrets):
    AGENTGATEWAY_CONFIG: Full YAML configuration for agentgateway
    AGENTGATEWAY_IMAGE: Container image to use (default: ghcr.io/agentgateway/agentgateway:latest)
"""

import os
import subprocess
from pathlib import Path

import modal

# App configuration
APP_NAME = "agentgateway"
DEFAULT_PORT = 3000
DEFAULT_IMAGE = "ghcr.io/agentgateway/agentgateway:latest"

# Create the Modal app
app = modal.App(f"{APP_NAME}-prebuilt")

# Get the image from environment or use default
image_name = os.environ.get("AGENTGATEWAY_IMAGE", DEFAULT_IMAGE)

# Use the pre-built image from container registry
prebuilt_image = modal.Image.from_registry(
    image_name,
    # Add Python for the Modal runtime
    add_python="3.12",
)


def get_config() -> str:
    """
    Get the agentgateway configuration from environment or defaults.
    """
    if config := os.environ.get("AGENTGATEWAY_CONFIG"):
        return config

    # Default configuration
    return f"""
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
    image=prebuilt_image,
    secrets=[modal.Secret.from_name("agentgateway-config", required=False)],
    cpu=1.0,
    memory=512,
    allow_concurrent_inputs=100,
    timeout=300,
)
@modal.web_server(port=DEFAULT_PORT, startup_timeout=60)
def serve():
    """
    Run agentgateway as a Modal web server using pre-built image.
    """
    config = get_config()
    config_path = Path("/tmp/agentgateway-config.yaml")
    config_path.write_text(config)

    subprocess.Popen(
        ["/app/agentgateway", "-f", str(config_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )


@app.local_entrypoint()
def main():
    """Local entrypoint for testing."""
    print(f"Using image: {image_name}")
    print(f"Deploy with: modal deploy modal/modal_app_prebuilt.py")
