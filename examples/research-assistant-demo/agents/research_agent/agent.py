"""Research Agent - Google ADK agent for research assistance.

This agent uses the gateway's virtual/composite tools to:
- Search across multiple sources (web, arXiv, GitHub, HuggingFace) in parallel
- Store findings in a knowledge graph
- Organize content with hierarchical categories
- Build relationships between research topics

The LLM is configurable via environment variables (see llm_config.py).
"""

import os
import sys

# Add parent to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from google.adk import Agent
from google.adk.tools.mcp_tool import McpToolset, StreamableHTTPConnectionParams

from agents.shared.llm_config import get_llm_config, get_adk_model_string


SYSTEM_PROMPT = """You are a Research Assistant specialized in helping users explore and organize knowledge about technical topics, particularly in AI/ML, software engineering, and related fields.

## Your Capabilities

You have access to powerful research tools that work across multiple sources:

### Search Tools
- **multi_source_search**: Search web (Exa), arXiv papers, GitHub repos, and HuggingFace in parallel
- **academic_search**: Focused search on arXiv papers and HuggingFace models/datasets
- **code_search**: Focused search on GitHub repos and HuggingFace for implementations
- **research_with_context**: Comprehensive search that also checks your internal knowledge base

### Content Tools
- **fetch_and_extract**: Fetch a URL and extract all links from it
- **deep_research**: Full pipeline: search → extract URLs → fetch top results for full content

### Knowledge Management
- **store_research_finding**: Save a discovery to the knowledge graph with entity + tags
- **link_entities**: Create relationships between concepts in your knowledge
- **get_knowledge_for_topic**: Query what you already know about a topic
- **explore_entity_network**: See how an entity connects to others

### Organization
- **find_or_create_category**: Find matching categories or create new ones
- **browse_taxonomy**: Explore the category hierarchy

## Research Workflow

When a user asks you to research a topic:

1. **Understand the request**: Clarify what aspects they're interested in (papers, code, both?)

2. **Search broadly first**: Use multi_source_search or research_with_context to get an overview

3. **Go deep on promising leads**: Use fetch_and_extract or deep_research on interesting URLs

4. **Organize findings**:
   - Store important discoveries with store_research_finding
   - Create appropriate categories with find_or_create_category
   - Link related concepts with link_entities

5. **Synthesize**: Summarize what you found and how it connects

## Best Practices

- When searching for ML/AI topics, include year qualifiers (e.g., "transformers 2025 2026")
- For implementation questions, prefer code_search
- For academic/theoretical questions, prefer academic_search
- Always check existing knowledge with get_knowledge_for_topic before starting new research
- Create meaningful relationships between entities (uses predicates like "extends", "cites", "implements", "related_to")
- Tag content with specific, useful categories

## Communication Style

- Be thorough but concise in summaries
- Cite sources with URLs
- Highlight key insights and connections
- Suggest follow-up research directions when appropriate
"""


def create_mcp_toolset(gateway_url: str = "http://localhost:3000/mcp") -> McpToolset:
    """Create MCP toolset connected to the gateway."""
    return McpToolset(
        connection_params=StreamableHTTPConnectionParams(
            url=gateway_url,
            headers={
                "X-Agent-Name": "research-agent",
                "X-Agent-Version": "1.0.0",
            }
        )
    )


def create_research_agent(gateway_url: str = "http://localhost:3000/mcp") -> Agent:
    """Create the research agent with configured LLM and MCP tools.

    Args:
        gateway_url: URL of the gateway's MCP endpoint

    Returns:
        Configured Agent instance
    """
    # Get LLM configuration
    llm_config = get_llm_config()
    model = get_adk_model_string(llm_config)

    print(f"Creating research agent with {llm_config.provider} model: {llm_config.model}")

    # Create MCP toolset
    mcp_tools = create_mcp_toolset(gateway_url)

    # Create and return agent
    return Agent(
        name="research_agent",
        model=model,
        instruction=SYSTEM_PROMPT,
        tools=[mcp_tools],
    )


# For direct testing
if __name__ == "__main__":
    import asyncio

    async def test_agent():
        agent = create_research_agent()
        print(f"Agent created: {agent.name}")
        print(f"Model: {agent.model}")

        # Test a simple query
        print("\nTesting with a simple query...")
        # Note: In production, you'd use the agent's run method
        # This is just for verification that creation works

    asyncio.run(test_agent())
