"""Research Agent - Google ADK agent for research assistance.

This agent uses the gateway's virtual/composite tools to:
- Search across multiple sources (web, arXiv, GitHub, HuggingFace) in parallel
- Store findings in a knowledge graph
- Organize content with hierarchical categories
- Build relationships between research topics

The LLM is configurable via environment variables (see llm_config.py).
"""

import logging
import os
from typing import Optional

from google.adk import Agent
from google.adk.runners import Runner
from google.adk.sessions import InMemorySessionService
from google.adk.tools.mcp_tool import McpToolset
from google.adk.tools.mcp_tool.mcp_toolset import StreamableHTTPConnectionParams
from google.genai import types

logger = logging.getLogger(__name__)

# Configuration
AGENT_NAME = "research-agent"
AGENT_VERSION = "1.0.0"
GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:3000")

# LLM Provider Configuration (using LiteLLM format for non-Google models)
DEFAULT_MODELS = {
    "anthropic": "anthropic/claude-sonnet-4-20250514",
    "openai": "openai/gpt-4o",
    "google": "gemini-2.0-flash",
}

API_KEY_ENV_VARS = {
    "anthropic": "ANTHROPIC_API_KEY",
    "openai": "OPENAI_API_KEY",
    "google": "GOOGLE_API_KEY",
}

# Provider detection priority
PROVIDER_PRIORITY = ["anthropic", "openai", "google"]


def detect_llm_provider() -> str | None:
    """Detect which LLM provider is available based on API keys."""
    for provider in PROVIDER_PRIORITY:
        env_var = API_KEY_ENV_VARS[provider]
        if os.environ.get(env_var):
            return provider
    return None


def get_configured_model() -> str:
    """Get the configured LLM model with multi-provider support.

    Checks in order:
    1. LLM_MODEL env var (explicit model override)
    2. LLM_PROVIDER env var (explicit provider selection)
    3. Auto-detect from available API keys (Anthropic > OpenAI > Google)
    """
    # Check for explicit model override
    if model := os.environ.get("LLM_MODEL"):
        logger.info(f"Using explicit model override: {model}")
        return model

    # Check for explicit provider selection
    if provider := os.environ.get("LLM_PROVIDER", "").lower():
        if provider in DEFAULT_MODELS:
            model = DEFAULT_MODELS[provider]
            logger.info(f"Using {provider} provider: {model}")
            return model

    # Auto-detect from API keys
    provider = detect_llm_provider()
    if provider:
        model = DEFAULT_MODELS[provider]
        logger.info(f"Auto-detected {provider} provider: {model}")
        return model

    # No API key found - this will fail at runtime
    raise RuntimeError(
        "No LLM API key found. Set one of: "
        f"{', '.join(API_KEY_ENV_VARS.values())}"
    )


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


def _create_mcp_toolset() -> McpToolset:
    """Create McpToolset connected to the gateway.

    The gateway filters tools based on agent identity (via clientInfo.name
    in the MCP initialize request). Tool filtering is handled server-side
    based on the registry's agent dependencies configuration.
    """
    return McpToolset(
        connection_params=StreamableHTTPConnectionParams(
            url=f"{GATEWAY_URL}/mcp",
            headers={
                "X-Agent-Name": AGENT_NAME,
                "X-Agent-Version": AGENT_VERSION,
            },
        ),
    )


# Global session service (shared across requests)
_session_service: Optional[InMemorySessionService] = None
_runner: Optional[Runner] = None
_mcp_toolset: Optional[McpToolset] = None


def _get_session_service() -> InMemorySessionService:
    """Get or create the session service."""
    global _session_service
    if _session_service is None:
        _session_service = InMemorySessionService()
    return _session_service


def _get_mcp_toolset() -> McpToolset:
    """Get or create the MCP toolset."""
    global _mcp_toolset
    if _mcp_toolset is None:
        logger.info(f"Creating MCP toolset connected to gateway: {GATEWAY_URL}/mcp")
        logger.info(f"Agent identity: {AGENT_NAME} v{AGENT_VERSION}")
        _mcp_toolset = _create_mcp_toolset()
    return _mcp_toolset


def create_research_agent() -> Agent:
    """Create the research agent with configured LLM and MCP tools.

    Returns:
        Configured Agent instance
    """
    model = get_configured_model()
    mcp_toolset = _get_mcp_toolset()

    logger.info(f"Creating research agent with model: {model}")

    return Agent(
        name="research_agent",
        model=model,
        description="Research assistant that helps discover and organize technical knowledge",
        instruction=SYSTEM_PROMPT,
        tools=[mcp_toolset],
    )


def get_runner() -> Runner:
    """Get or create the Runner with session service.

    Returns:
        Configured Runner instance
    """
    global _runner
    if _runner is None:
        agent = create_research_agent()
        session_service = _get_session_service()
        _runner = Runner(
            app_name="research_assistant_app",
            agent=agent,
            session_service=session_service,
        )
    return _runner


APP_NAME = "research_assistant_app"


async def _ensure_session(session_service: InMemorySessionService, user_id: str, session_id: str):
    """Ensure a session exists, creating it if needed."""
    session = await session_service.get_session(
        app_name=APP_NAME,
        user_id=user_id,
        session_id=session_id,
    )
    if session is None:
        # Session doesn't exist, create it
        session = await session_service.create_session(
            app_name=APP_NAME,
            user_id=user_id,
            session_id=session_id,
        )
    return session


async def run_research_query(
    query: str,
    user_id: str = "default-user",
    session_id: str = "default-session",
) -> str:
    """Run a single query through the research agent.

    Args:
        query: The user's research question or request
        user_id: The user ID for personalization
        session_id: Session ID for conversation continuity

    Returns:
        Agent's response

    Raises:
        RuntimeError: If no LLM API key is configured or agent fails
    """
    # Validate that at least one provider is configured
    provider = detect_llm_provider()
    if not provider:
        raise RuntimeError(
            "No LLM API key found. Set one of: "
            f"{', '.join(API_KEY_ENV_VARS.values())}"
        )

    runner = get_runner()
    session_service = _get_session_service()

    # Ensure session exists
    await _ensure_session(session_service, user_id, session_id)

    # Create the message
    message = types.Content(
        role="user",
        parts=[types.Part(text=query)],
    )

    # Run the agent
    logger.info(f"Running agent for user={user_id}, session={session_id}")
    response_parts = []

    async for event in runner.run_async(
        user_id=user_id,
        session_id=session_id,
        new_message=message,
    ):
        # Collect response parts
        if hasattr(event, "content") and event.content:
            for part in event.content.parts:
                if hasattr(part, "text") and part.text:
                    response_parts.append(part.text)

    if not response_parts:
        raise RuntimeError("Agent returned empty response")

    return "".join(response_parts)


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
