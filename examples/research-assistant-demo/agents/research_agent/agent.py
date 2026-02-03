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

import os

from google.adk import Agent
from google.adk.tools.mcp_tool import McpToolset
from google.adk.tools.mcp_tool.mcp_toolset import StreamableHTTPConnectionParams

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
- **virtual_multi_source_search**: Search web (Exa), arXiv papers, GitHub repos, and HuggingFace in parallel
- **virtual_academic_search**: Focused search on arXiv papers and HuggingFace models/datasets
- **virtual_code_search**: Focused search on GitHub repos and HuggingFace for implementations

### Content Tools
- **virtual_fetch_and_extract**: Fetch a URL and extract all links from it

### Knowledge Management
- **virtual_store_research_finding**: Save a discovery to the knowledge graph with entity + tags
- **virtual_link_entities**: Create relationships between concepts in your knowledge
- **virtual_get_knowledge_for_topic**: Query what you already know about a topic
- **virtual_explore_entity_network**: See how an entity connects to others

### Organization
- **virtual_find_or_create_category**: Find matching categories or create new ones
- **virtual_browse_taxonomy**: Explore the category hierarchy

## Research Workflow

When a user asks you to research a topic:

1. **Understand the request**: Clarify what aspects they're interested in (papers, code, both?)

2. **Search broadly first**: Use virtual_multi_source_search to get an overview

3. **Go deep on promising leads**: Use virtual_fetch_and_extract on interesting URLs

4. **Organize findings**:
   - Store important discoveries with virtual_store_research_finding
   - Create appropriate categories with virtual_find_or_create_category
   - Link related concepts with virtual_link_entities

5. **Synthesize**: Summarize what you found and how it connects

## Best Practices

- When searching for ML/AI topics, include year qualifiers (e.g., "transformers 2025 2026")
- For implementation questions, prefer virtual_code_search
- For academic/theoretical questions, prefer virtual_academic_search
- Always check existing knowledge with virtual_get_knowledge_for_topic before starting new research
- Create meaningful relationships between entities (uses predicates like "extends", "cites", "implements", "related_to")

## Communication Style

- Be thorough but concise in summaries
- Cite sources with URLs
- Highlight key insights and connections
- Suggest follow-up research directions when appropriate
"""

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
)
