"""Research Agent - Google ADK agent for research assistance.

This agent uses the gateway's virtual/composite tools to:
- Search across multiple sources (web, arXiv, GitHub, HuggingFace) in parallel
- Store findings in a knowledge graph
- Organize content with hierarchical categories
- Build relationships between research topics

Following ADK conventions:
- Agent defined at module level as `root_agent`
- McpToolset configured to connect to gateway
- Uses `get_fast_api_app` for serving (see main.py)
"""

import json
import logging
import os
import time
from typing import Any

from google.adk import Agent
from google.adk.tools.base_tool import BaseTool
from google.adk.tools.mcp_tool import McpToolset
from google.adk.tools.mcp_tool.mcp_toolset import StreamableHTTPConnectionParams
from google.adk.tools.tool_context import ToolContext

logger = logging.getLogger("research_agent")

# ==============================================================================
# CONFIGURATION
# ==============================================================================

AGENT_NAME = "research-agent"
AGENT_VERSION = "1.0.0"

# Gateway URL - set via environment variable or default to localhost
GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:3000")

# Model selection - auto-detect from available API keys
# Priority: Anthropic > OpenAI > Google
# Note: For non-Google models, ADK uses LiteLLM provider format
def _detect_model() -> str:
    """Auto-detect model based on available API keys."""
    if explicit := os.environ.get("LLM_MODEL"):
        return explicit
    if os.environ.get("ANTHROPIC_API_KEY"):
        # LiteLLM format: provider/model (no "litellm/" prefix)
        return "anthropic/claude-sonnet-4-20250514"
    if os.environ.get("OPENAI_API_KEY"):
        return "openai/gpt-4o"
    return "gemini-2.0-flash"

MODEL = _detect_model()

SYSTEM_PROMPT = """You are a Research Assistant specialized in helping users explore and organize knowledge about technical topics, particularly in AI/ML, software engineering, and related fields.

## Your Capabilities

You have access to powerful research tools that work across multiple sources:

### Search Tools
- **virtual_multi_source_search**: PRIMARY SEARCH TOOL. Searches all 4 sources (Exa web, arXiv, GitHub, HuggingFace) in parallel and returns unified results. Fast (~1s). Always use this for research queries.

### Single-Source Search (when you need just one source)
- **virtual_web_research**: Web search via Exa
- **virtual_normalized_arxiv**: arXiv paper search
- **virtual_normalized_github**: GitHub repo search
- **virtual_normalized_huggingface**: HuggingFace model/dataset search

### Content Tools
- **fetch-service_url_fetch**: Fetch a single URL and extract its content
- **fetch-service_batch_fetch**: Fetch multiple URLs at once
- **fetch-service_extract_urls**: Extract all links from a page

### Knowledge Management
- **virtual_store_research_finding**: Save a finding (paper, concept, tool, person, etc.) to the knowledge graph. Stored findings are surfaced by future searches when semantically relevant.
- **virtual_create_relation**: Link two entities (e.g., "paper X cites paper Y", "tool A implements concept B")
- **entity-service_entity_search**: Search your existing knowledge graph for relevant entities
- **entity-service_search_relations**: Explore how entities connect to each other

### Organization
- **virtual_create_category**: Add a category to the taxonomy for organizing content
- **virtual_tag_content**: Tag content with categories
- **category-service_search_categories**: Find existing categories
- **category-service_get_category_tree**: Browse the category hierarchy

## Research Workflow

When a user asks you to research a topic:

1. **Search**: Use virtual_multi_source_search to get results from all 4 sources fast.

2. **Store findings**: ALWAYS follow up search with virtual_store_research_finding to save the important discoveries (papers, tools, concepts, people) to the knowledge graph. Do this for each noteworthy result — this is the core value of the assistant.

3. **Build connections**:
   - Link related concepts with virtual_create_relation ("extends", "cites", "implements")
   - Create categories with virtual_create_category to build a taxonomy

4. **Fetch if needed**: If the user wants to read the actual content of specific results, use fetch-service_url_fetch on individual URLs.

5. **Synthesize**: Summarize what you found and how it connects.

## Best Practices

- Default workflow: virtual_multi_source_search → virtual_store_research_finding for important results → virtual_create_relation to connect them
- When searching for ML/AI topics, include year qualifiers (e.g., "transformers 2025 2026")
- Use individual normalized searches (virtual_normalized_arxiv, etc.) when you only need one source
- Check existing knowledge with entity-service_entity_search before starting new research
- Create meaningful relationships between entities (uses predicates like "extends", "cites", "implements", "related_to")

## Communication Style

- Be thorough but concise in summaries
- Cite sources with URLs
- Highlight key insights and connections
- Suggest follow-up research directions when appropriate
"""

# ==============================================================================
# TOOL CALL LOGGING
# ==============================================================================

# Store start times keyed by (tool_name, id(tool_context)) to handle concurrent calls
_tool_start_times: dict[str, float] = {}


# ANSI color codes for terminal output
import re

DARK_RED = "\033[31m"
DARK_BLUE = "\033[34m"
RESET = "\033[0m"

_SOURCES = ("exa", "arxiv", "github", "huggingface")


def _truncate(value: Any, max_len: int = 500) -> str:
    """Truncate a value for debug logging."""
    s = json.dumps(value, default=str) if not isinstance(value, str) else value
    if len(s) > max_len:
        return s[:max_len] + f"... ({len(s)} chars)"
    return s


def _color_tool(name: str) -> str:
    """Wrap tool name in dark red ANSI escape."""
    return f"{DARK_RED}{name}{RESET}"


def _color_sources(s: str) -> str:
    """Highlight source names in dark blue, handling both raw and escaped JSON."""
    for src in _SOURCES:
        s = s.replace(f'\\"source\\":\\"{src}\\"', f'\\"source\\":\\"{DARK_BLUE}{src}{RESET}\\"')
        s = s.replace(f'"source":"{src}"', f'"source":"{DARK_BLUE}{src}{RESET}"')
        s = s.replace(f'"source": "{src}"', f'"source": "{DARK_BLUE}{src}{RESET}"')
    return s


def before_tool(*, tool: BaseTool, args: dict[str, Any], tool_context: ToolContext, **_: Any) -> None:
    """Log tool call start. In debug mode, also log arguments."""
    key = f"{tool.name}:{id(tool_context)}"
    _tool_start_times[key] = time.monotonic()
    logger.info("tool_call_start  tool=%s", _color_tool(tool.name))
    if logger.isEnabledFor(logging.DEBUG):
        logger.debug("tool_call_args   tool=%s args=%s", _color_tool(tool.name), _truncate(args))


def after_tool(
    *, tool: BaseTool, args: dict[str, Any], tool_context: ToolContext,
    tool_response: dict | None = None, **_: Any,
) -> None:
    """Log tool call completion with timing. In debug mode, also log result."""
    key = f"{tool.name}:{id(tool_context)}"
    start = _tool_start_times.pop(key, None)
    elapsed_ms = int((time.monotonic() - start) * 1000) if start else -1
    logger.info("tool_call_done   tool=%s elapsed=%dms", _color_tool(tool.name), elapsed_ms)
    if logger.isEnabledFor(logging.DEBUG):
        result_str = _color_sources(_truncate(tool_response))
        logger.debug("tool_call_result tool=%s result=%s", _color_tool(tool.name), result_str)


# ==============================================================================
# MCP TOOLSET
# ==============================================================================

# Create MCP toolset connecting to the gateway
# This is defined at module level as required by ADK
mcp_toolset = McpToolset(
    connection_params=StreamableHTTPConnectionParams(
        url=f"{GATEWAY_URL}/mcp",
        headers={
            "X-Agent-Name": AGENT_NAME,
            "X-Agent-Version": AGENT_VERSION,
        },
    ),
)

# ==============================================================================
# AGENT DEFINITION
# ==============================================================================

# Define agent at module level as required by ADK
# This is exported as `root_agent` for get_fast_api_app to discover
root_agent = Agent(
    name="research_agent",
    model=MODEL,
    description="Research assistant that helps discover and organize technical knowledge",
    instruction=SYSTEM_PROMPT,
    tools=[mcp_toolset],
    before_tool_callback=before_tool,
    after_tool_callback=after_tool,
)
