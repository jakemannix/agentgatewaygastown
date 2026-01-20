"""Shared utilities for ecommerce demo agents."""

from .gateway_client import GatewayMCPClient, discover_tools, call_tool

__all__ = ["GatewayMCPClient", "discover_tools", "call_tool"]
