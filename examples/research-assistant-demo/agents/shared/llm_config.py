"""LLM configuration with cascading environment variable support.

Supports multiple LLM providers with automatic detection based on available API keys.
Priority order: Anthropic > OpenAI > Google (Gemini)

Environment variables checked:
- ANTHROPIC_API_KEY -> Claude models
- OPENAI_API_KEY -> GPT models
- GOOGLE_API_KEY or GEMINI_API_KEY -> Gemini models

Can override with:
- LLM_PROVIDER: "anthropic", "openai", or "google"
- LLM_MODEL: specific model name
"""

import os
from dataclasses import dataclass
from typing import Literal

from dotenv import load_dotenv

# Load .env file if present
load_dotenv()


@dataclass
class LLMConfig:
    """Configuration for LLM provider and model."""
    provider: Literal["anthropic", "openai", "google"]
    model: str
    api_key: str


def get_llm_config() -> LLMConfig:
    """Get LLM configuration based on available environment variables.

    Checks for API keys and returns the first available provider.
    Can be overridden with LLM_PROVIDER and LLM_MODEL env vars.
    """
    # Check for explicit override
    explicit_provider = os.getenv("LLM_PROVIDER", "").lower()
    explicit_model = os.getenv("LLM_MODEL", "")

    # Get API keys
    anthropic_key = os.getenv("ANTHROPIC_API_KEY", "")
    openai_key = os.getenv("OPENAI_API_KEY", "")
    google_key = os.getenv("GOOGLE_API_KEY", "") or os.getenv("GEMINI_API_KEY", "")

    # If explicit provider specified, use it
    if explicit_provider:
        if explicit_provider == "anthropic":
            if not anthropic_key:
                raise ValueError("ANTHROPIC_API_KEY required when LLM_PROVIDER=anthropic")
            model = explicit_model or "claude-sonnet-4-20250514"
            return LLMConfig(provider="anthropic", model=model, api_key=anthropic_key)
        elif explicit_provider == "openai":
            if not openai_key:
                raise ValueError("OPENAI_API_KEY required when LLM_PROVIDER=openai")
            model = explicit_model or "gpt-4o"
            return LLMConfig(provider="openai", model=model, api_key=openai_key)
        elif explicit_provider == "google":
            if not google_key:
                raise ValueError("GOOGLE_API_KEY or GEMINI_API_KEY required when LLM_PROVIDER=google")
            model = explicit_model or "gemini-2.0-flash"
            return LLMConfig(provider="google", model=model, api_key=google_key)
        else:
            raise ValueError(f"Unknown LLM_PROVIDER: {explicit_provider}")

    # Auto-detect based on available keys (priority: Anthropic > OpenAI > Google)
    if anthropic_key:
        model = explicit_model or "claude-sonnet-4-20250514"
        return LLMConfig(provider="anthropic", model=model, api_key=anthropic_key)
    elif openai_key:
        model = explicit_model or "gpt-4o"
        return LLMConfig(provider="openai", model=model, api_key=openai_key)
    elif google_key:
        model = explicit_model or "gemini-2.0-flash"
        return LLMConfig(provider="google", model=model, api_key=google_key)
    else:
        raise ValueError(
            "No LLM API key found. Set one of: "
            "ANTHROPIC_API_KEY, OPENAI_API_KEY, GOOGLE_API_KEY, or GEMINI_API_KEY"
        )


def get_adk_model_string(config: LLMConfig) -> str:
    """Get the model string formatted for Google ADK.

    ADK uses format like "anthropic/claude-sonnet-4-20250514" for Anthropic models.
    """
    if config.provider == "anthropic":
        return f"anthropic/{config.model}"
    elif config.provider == "openai":
        return f"openai/{config.model}"
    elif config.provider == "google":
        # Google models don't need prefix in ADK
        return config.model
    else:
        raise ValueError(f"Unknown provider: {config.provider}")


def print_llm_config():
    """Print the current LLM configuration for debugging."""
    try:
        config = get_llm_config()
        print(f"LLM Provider: {config.provider}")
        print(f"LLM Model: {config.model}")
        print(f"API Key: {'*' * 8}...{config.api_key[-4:] if len(config.api_key) > 4 else '****'}")
    except ValueError as e:
        print(f"LLM Config Error: {e}")


if __name__ == "__main__":
    print_llm_config()
