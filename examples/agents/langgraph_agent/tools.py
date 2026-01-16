"""LangChain tool adapters for MCP tools."""

from __future__ import annotations

import json
from typing import Any, Callable, Type

from langchain_core.tools import BaseTool, StructuredTool
from pydantic import BaseModel, Field, create_model

from .mcp_client import MCPClient, ToolInfo


def _create_pydantic_model_from_schema(
    name: str, schema: dict[str, Any]
) -> Type[BaseModel]:
    """Create a Pydantic model from a JSON schema."""
    properties = schema.get("properties", {})
    required = set(schema.get("required", []))

    field_definitions: dict[str, tuple[type, Any]] = {}
    for prop_name, prop_schema in properties.items():
        prop_type = _json_type_to_python(prop_schema.get("type", "string"))
        default = ... if prop_name in required else None
        description = prop_schema.get("description", "")
        field_definitions[prop_name] = (
            prop_type,
            Field(default=default, description=description),
        )

    if not field_definitions:
        # Empty schema - create model with no fields
        return create_model(name)

    return create_model(name, **field_definitions)


def _json_type_to_python(json_type: str) -> type:
    """Convert JSON schema type to Python type."""
    type_map = {
        "string": str,
        "number": float,
        "integer": int,
        "boolean": bool,
        "array": list,
        "object": dict,
    }
    return type_map.get(json_type, str)


class MCPToolProvider:
    """Provides LangChain-compatible tools from an MCP endpoint.

    This class connects to an agentgateway MCP endpoint and creates
    LangChain tools that can be used with LangGraph agents.
    """

    def __init__(self, mcp_client: MCPClient):
        self.mcp_client = mcp_client
        self._tools: list[BaseTool] = []
        self._tool_infos: list[ToolInfo] = []

    def load_tools(self) -> list[BaseTool]:
        """Load tools from the MCP endpoint and create LangChain tools.

        Returns a list of LangChain tools that wrap the MCP tools.
        """
        self._tool_infos = self.mcp_client.list_tools()
        self._tools = []

        for tool_info in self._tool_infos:
            langchain_tool = self._create_tool(tool_info)
            self._tools.append(langchain_tool)

        return self._tools

    async def load_tools_async(self) -> list[BaseTool]:
        """Load tools from the MCP endpoint asynchronously.

        Returns a list of LangChain tools that wrap the MCP tools.
        """
        self._tool_infos = await self.mcp_client.list_tools_async()
        self._tools = []

        for tool_info in self._tool_infos:
            langchain_tool = self._create_tool(tool_info)
            self._tools.append(langchain_tool)

        return self._tools

    def _create_tool(self, tool_info: ToolInfo) -> BaseTool:
        """Create a LangChain tool from an MCP tool info."""
        # Create args schema from MCP input schema
        args_schema = _create_pydantic_model_from_schema(
            f"{tool_info.name.replace(':', '_').title()}Args",
            tool_info.input_schema,
        )

        # Create wrapper function that calls the MCP tool
        mcp_client = self.mcp_client
        tool_name = tool_info.name

        def invoke_tool(**kwargs: Any) -> str:
            """Invoke the MCP tool with the given arguments."""
            result = mcp_client.call_tool(tool_name, kwargs)
            if isinstance(result, str):
                return result
            return json.dumps(result, indent=2)

        async def ainvoke_tool(**kwargs: Any) -> str:
            """Invoke the MCP tool asynchronously with the given arguments."""
            result = await mcp_client.call_tool_async(tool_name, kwargs)
            if isinstance(result, str):
                return result
            return json.dumps(result, indent=2)

        return StructuredTool(
            name=tool_info.name.replace(":", "_"),  # LangChain doesn't like colons
            description=tool_info.description or f"MCP tool: {tool_info.name}",
            func=invoke_tool,
            coroutine=ainvoke_tool,
            args_schema=args_schema,
        )

    @property
    def tools(self) -> list[BaseTool]:
        """Get the loaded tools."""
        return self._tools

    @property
    def tool_infos(self) -> list[ToolInfo]:
        """Get the raw MCP tool information."""
        return self._tool_infos

    def get_tool_by_name(self, name: str) -> BaseTool | None:
        """Get a specific tool by name."""
        # Handle both original MCP name and LangChain-safe name
        normalized = name.replace(":", "_")
        for tool in self._tools:
            if tool.name == normalized or tool.name == name:
                return tool
        return None
