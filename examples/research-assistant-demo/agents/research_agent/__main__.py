"""Research Agent entry point - redirects to main.py.

For ADK standard deployment, use main.py in the parent directory:
    python main.py
    # or
    uvicorn main:app --port 9001

This file is kept for backwards compatibility with the start_services.sh script.
"""

import os
import sys

# Add parent directories to path
demo_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, demo_dir)

if __name__ == "__main__":
    # Import and run the main module
    import main
    # main.py runs uvicorn when executed directly
