#!/usr/bin/env python3
"""List all tools available from the gateway.

This script helps verify that:
1. The gateway is running and accessible
2. Tools are correctly registered in the registry
3. Composition tools are properly exposed

Usage:
    python list_gateway_tools.py [--gateway-url http://localhost:3000]
"""

import asyncio
import argparse
import sys
import os

# Add the demo directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from agents.shared.gateway_client import GatewayMCPClient


async def main(gateway_url: str):
    print(f"\nConnecting to gateway at: {gateway_url}")

    client = GatewayMCPClient(gateway_url=gateway_url, agent_name="tool-lister")

    try:
        tools = await client.list_tools()

        print(f"\n{'='*70}")
        print(f"Found {len(tools)} tools from gateway")
        print(f"{'='*70}\n")

        # Known composition tool names from the registry
        known_compositions = {"personalized_search", "product_with_availability"}

        # Group tools
        compositions = []
        regular = []

        for tool in tools:
            name = tool.get("name", "unknown")
            # Check by known names or _composition/ prefix
            if name in known_compositions or name.startswith("_composition/"):
                compositions.append(tool)
            else:
                regular.append(tool)

        if compositions:
            print(f"COMPOSITION TOOLS ({len(compositions)}):")
            print("-" * 50)
            for tool in sorted(compositions, key=lambda t: t.get("name", "")):
                name = tool.get("name", "unknown")
                desc = tool.get("description", "No description")
                print(f"  * {name}")
                print(f"      {desc[:70]}...")
                # Show input schema summary
                schema = tool.get("inputSchema", {})
                props = schema.get("properties", {})
                required = schema.get("required", [])
                if props:
                    params = []
                    for p, v in props.items():
                        req = "*" if p in required else ""
                        params.append(f"{p}{req}")
                    print(f"      params: {', '.join(params)}")
            print()

        print(f"REGULAR TOOLS ({len(regular)}):")
        print("-" * 50)
        for tool in sorted(regular, key=lambda t: t.get("name", "")):
            name = tool.get("name", "unknown")
            desc = tool.get("description", "No description")[:50]
            print(f"  {name}: {desc}...")

        print()
        print("="*70)
        print("Summary:")
        print(f"  - Total tools: {len(tools)}")
        print(f"  - Composition tools: {len(compositions)}")
        print(f"  - Regular tools: {len(regular)}")

        # Check for expected composition tools
        all_names = {t.get("name") for t in tools}
        expected_compositions = {"personalized_search", "product_with_availability"}
        missing = expected_compositions - all_names
        if missing:
            print(f"\n  WARNING: Missing expected composition tools: {missing}")
        else:
            print(f"\n  All expected composition tools present")
        print("="*70)

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="List gateway tools")
    parser.add_argument(
        "--gateway-url",
        default="http://localhost:3000",
        help="Gateway URL (default: http://localhost:3000)",
    )
    args = parser.parse_args()

    asyncio.run(main(args.gateway_url))
