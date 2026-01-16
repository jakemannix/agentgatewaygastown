#!/usr/bin/env python3
"""Entry point for running the package with python -m or uv run."""

import asyncio
import sys

# When running as a script directly
if __name__ == "__main__":
    # Import here to handle both direct script and module execution
    from agent import main
    sys.exit(asyncio.run(main()))
