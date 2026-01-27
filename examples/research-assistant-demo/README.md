# Research Assistant Demo

A comprehensive demonstration of AgentGateway's virtual/composite tools capabilities for building a research assistant that orchestrates multiple microservices.

## Overview

This demo showcases how virtual tools can provide **distributed joins** and **orchestration** across intentionally decoupled microservices. Rather than building monolithic services, we decompose functionality into small, focused services and let the gateway compose them into powerful workflows.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Research Agent                         │
│              (Google ADK + Configurable LLM)            │
│                      :9001                               │
└───────────────────────┬─────────────────────────────────┘
                        │ MCP (Streamable HTTP)
                        │
┌───────────────────────▼─────────────────────────────────┐
│                   AgentGateway                           │
│           Virtual Tools & Compositions                   │
│                      :3000                               │
│                                                          │
│  Composition Patterns:                                   │
│  • Scatter-Gather (parallel search)                      │
│  • Pipeline (fetch → extract → process)                  │
│  • Cross-Service Pipeline (distributed joins)            │
└───┬───────┬───────┬───────┬───────┬─────────────────────┘
    │       │       │       │       │
    ▼       ▼       ▼       ▼       ▼
┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│Search │ │Fetch  │ │Entity │ │Category│ │ Tag   │
│Service│ │Service│ │Service│ │Service │ │Service│
│ :8001 │ │ :8002 │ │ :8003 │ │ :8004  │ │ :8005 │
└───────┘ └───────┘ └───────┘ └───────┘ └───────┘
    │                   │         │          │
    ▼                   ▼         ▼          ▼
 External         ┌────────────────────────────┐
  APIs            │   SQLite + sqlite-vec      │
(Exa,arXiv,       │  (Vector Embeddings)       │
GitHub,HF)        └────────────────────────────┘
```

### The "Intentionally Decoupled" Design

In a traditional design, you might build a single "ResearchService" with all functionality. Instead, this demo deliberately separates concerns into 5 microservices:

1. **search-service**: External API integration (Exa, arXiv, GitHub, HuggingFace)
2. **fetch-service**: URL fetching and content extraction
3. **entity-service**: Knowledge graph (entities + relations) with vector search
4. **category-service**: Hierarchical taxonomy management
5. **tag-service**: Content-category associations

This separation demonstrates how virtual tools **compose** these services into coherent workflows:

- **multi_source_search**: Scatter-gather across 4 search backends in parallel
- **deep_research**: Pipeline that searches → extracts URLs → fetches content
- **store_research_finding**: Cross-service pipeline that creates entity + registers content + tags

## Quick Start

### Prerequisites

- Rust 1.86+ (for building the gateway)
- Python 3.10+
- [uv](https://github.com/astral-sh/uv) (Python package manager)
- tmux (for running multiple services)

### 1. Build the Gateway

```bash
# From the repository root
cargo build -p agentgateway-app
```

### 2. Configure Environment

```bash
cd examples/research-assistant-demo

# Copy environment template
cp .env.example .env

# Edit .env with your API keys
```

#### Required Environment Variables

At minimum, you need **one LLM provider**:

| Variable | Required | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | One of these | Claude models (recommended) |
| `OPENAI_API_KEY` | One of these | GPT models |
| `GOOGLE_API_KEY` | One of these | Gemini models |

#### Search Service API Keys

Each search source requires its own API key. **Without a key, the search will return an error** (no mock data):

| Variable | Required | Description |
|----------|----------|-------------|
| `EXA_API_KEY` | For web search | Get at [exa.ai](https://exa.ai). Without it, `exa_search` returns an error |
| `GITHUB_TOKEN` | Optional | Increases rate limits. Without it, you may hit rate limits |
| `HF_TOKEN` | Optional | Increases rate limits for HuggingFace API |

**Note:** arXiv API is free and requires no key.

#### Example .env file

```bash
# LLM Provider (at least one required)
ANTHROPIC_API_KEY=sk-ant-...

# Search APIs
EXA_API_KEY=...              # Required for web search
GITHUB_TOKEN=ghp_...         # Optional, for higher rate limits
HF_TOKEN=hf_...              # Optional, for higher rate limits
```

### 3. Start Services

```bash
./start_services.sh
```

This starts all 5 microservices, the gateway, and the research agent in a tmux session.

### 4. Test the Agent

```bash
# Simple chat request
curl -X POST http://localhost:9001/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"Research transformer alternatives for 2025-2026"}'

# Check agent card
curl http://localhost:9001/.well-known/agent.json
```

### 5. Monitor & Debug

```bash
# View all service logs
tmux attach -t research-demo

# Navigate windows: Ctrl+B, then 0/1/2 for services/gateway/agent

# Stop everything
./stop_services.sh
```

## Virtual Tools Showcase

### Declarative Output Transformation

Each search backend returns its **native API format**. The gateway uses `outputTransform` mappings to normalize them into a common schema. This demonstrates declarative data transformation without code:

```json
{
  "name": "normalized_arxiv_search",
  "source": {"server": "search-service", "tool": "arxiv_search"},
  "outputTransform": {
    "mappings": {
      "query": {"path": "$.query"},
      "source": {"literal": {"stringValue": "arxiv"}},
      "error": {"path": "$.error"},
      "results": {"path": "$.papers[*]", "nested": {
        "mappings": {
          "source": {"literal": {"stringValue": "arxiv"}},
          "title": {"path": "$.title"},
          "url": {"coalesce": {"paths": ["$.pdf_url", "$.abs_url"]}},
          "snippet": {"path": "$.abstract"},
          "score": {"literal": {"numberValue": 1.0}},
          "metadata": {
            "nested": {"mappings": {"arxiv_id": {"path": "$.arxiv_id"}, ...}}
          }
        }
      }}
    }
  }
}
```

**Available mapping types:**
- `path`: JSONPath extraction (field renaming via target key)
- `literal`: Constant values (`stringValue`, `numberValue`, `boolValue`)
- `coalesce`: First non-null from multiple paths
- `nested`: Recursive object construction
- `template`: String interpolation with variables
- `concat`: Concatenate multiple paths

### Scatter-Gather: Parallel Multi-Source Search

The `multi_source_search` tool demonstrates parallel execution using normalized wrapper tools:

```json
{
  "name": "multi_source_search",
  "spec": {
    "scatterGather": {
      "targets": [
        {"tool": "normalized_exa_search"},
        {"tool": "normalized_arxiv_search"},
        {"tool": "normalized_github_search"},
        {"tool": "normalized_huggingface_search"}
      ],
      "aggregation": {
        "ops": [
          {"flatten": true},
          {"sort": {"field": "$.score", "order": "desc"}},
          {"limit": {"count": 40}}
        ]
      }
    }
  }
}
```

**What it demonstrates:**
- 4 external APIs called simultaneously
- Each tool applies its `outputTransform` to normalize native responses
- Results aggregated, sorted by relevance, limited to top 40
- Single tool call from the agent's perspective
- Error handling: failed sources return `error` field instead of results

### Pipeline: Sequential Processing

The `fetch_and_extract` tool chains operations:

```json
{
  "name": "fetch_and_extract",
  "spec": {
    "pipeline": {
      "steps": [
        {
          "id": "fetch",
          "operation": {"tool": {"name": "url_fetch", "server": "fetch-service"}}
        },
        {
          "id": "extract",
          "operation": {"tool": {"name": "extract_urls", "server": "fetch-service"}},
          "input": {
            "construct": {
              "fields": {
                "text": {"reference": {"step": "fetch", "path": "$.content"}}
              }
            }
          }
        }
      ]
    }
  }
}
```

**What it demonstrates:**
- Step 2 uses output from step 1
- Data flows through JSONPath references
- Combined into single atomic operation

### Cross-Service Pipeline: Distributed Joins

The `store_research_finding` tool demonstrates a pipeline that spans multiple services:

```json
{
  "name": "store_research_finding",
  "spec": {
    "pipeline": {
      "steps": [
        {
          "id": "create_entity",
          "operation": {"tool": {"name": "create_entity", "server": "entity-service"}},
          "input": {"construct": {"fields": {"name": {"input": {"path": "$.title"}}, ...}}}
        },
        {
          "id": "register_content",
          "operation": {"tool": {"name": "register_content", "server": "tag-service"}},
          "input": {
            "construct": {
              "fields": {
                "metadata": {
                  "construct": {
                    "fields": {
                      "entity_id": {"reference": {"step": "create_entity", "path": "$.entity.id"}}
                    }
                  }
                }
              }
            }
          }
        },
        {
          "id": "tag_content",
          "operation": {"tool": {"name": "tag_content", "server": "tag-service"}},
          "input": {
            "construct": {
              "fields": {
                "content_id": {"reference": {"step": "register_content", "path": "$.content.id"}}
              }
            }
          }
        }
      ]
    }
  }
}
```

**What it demonstrates:**
- Pipeline spans 2 different services (entity-service → tag-service)
- Step 2 receives the entity ID created in step 1
- Step 3 receives the content ID created in step 2
- "Distributed join" - linking data across service boundaries

### DAG with Nested Patterns: comprehensive_research

The `comprehensive_research` tool demonstrates a true DAG where parallel branches join:

```
                    ┌─► exa_search ────────┐
                    ├─► arxiv_search ──────┤
    ┌─► [external]──┼─► github_search ─────┼──┐
    │               └─► huggingface_search─┘  │
    │                                         ├──► merge ──► batch_fetch
    │               ┌─► entity_search ────┐   │
    └─► [internal]──┼─► search_categories─┼───┘
                    └─► search_content ───┘
```

```json
{
  "name": "comprehensive_research",
  "spec": {
    "pipeline": {
      "steps": [
        {
          "id": "parallel_search",
          "operation": {
            "pattern": {
              "scatterGather": {
                "targets": [
                  {
                    "pattern": {
                      "scatterGather": {
                        "targets": [
                          {"tool": "exa_search", "server": "search-service"},
                          {"tool": "arxiv_search", "server": "search-service"},
                          {"tool": "github_search", "server": "search-service"},
                          {"tool": "huggingface_search", "server": "search-service"}
                        ],
                        "aggregation": {"ops": [{"flatten": true}, {"sort": ...}, {"limit": ...}]}
                      }
                    }
                  },
                  {
                    "pattern": {
                      "scatterGather": {
                        "targets": [
                          {"tool": "entity_search", "server": "entity-service"},
                          {"tool": "search_categories", "server": "category-service"},
                          {"tool": "search_content", "server": "tag-service"}
                        ]
                      }
                    }
                  }
                ],
                "aggregation": {"ops": [{"merge": true}]}
              }
            }
          }
        },
        {
          "id": "fetch_top_external",
          "operation": {"tool": {"name": "batch_fetch", "server": "fetch-service"}},
          "input": {
            "construct": {
              "fields": {
                "urls": {"reference": {"step": "parallel_search", "path": "$[0].results[0:3].url"}}
              }
            }
          }
        }
      ]
    }
  }
}
```

**What it demonstrates:**
- **Nested scatter-gather**: Two groups of parallel searches run simultaneously
- **Branch 1**: External APIs (4 tools in parallel)
- **Branch 2**: Internal knowledge (3 tools in parallel)
- **Join/Merge**: Both branches merged into single result
- **Chained processing**: Merged results feed into batch_fetch

### Hybrid: research_with_context

Combines external search with internal knowledge lookup (flat scatter-gather):

```json
{
  "name": "research_with_context",
  "spec": {
    "scatterGather": {
      "targets": [
        {"tool": "exa_search", "server": "search-service"},
        {"tool": "arxiv_search", "server": "search-service"},
        {"tool": "github_search", "server": "search-service"},
        {"tool": "entity_search", "server": "entity-service"},
        {"tool": "search_categories", "server": "category-service"}
      ]
    }
  }
}
```

**What it demonstrates:**
- External APIs (Exa, arXiv, GitHub) searched in parallel
- Internal knowledge (entities, categories) searched simultaneously
- Agent gets comprehensive view: new findings + existing knowledge

## Service Details

### Search Service (8001)

External search API integrations. Each tool returns its **native API format** - normalization happens via gateway `outputTransform`:

| Tool | Source | API | Returns | Notes |
|------|--------|-----|---------|-------|
| `exa_search` | Web | Exa.ai | `ExaSearchResponse` | Requires `EXA_API_KEY` |
| `arxiv_search` | Academic | arXiv API | `ArxivSearchResponse` | Free, no key needed |
| `github_search` | Code | GitHub API | `GitHubSearchResponse` | `GITHUB_TOKEN` optional |
| `huggingface_search` | ML | HuggingFace API | `HuggingFaceSearchResponse` | `HF_TOKEN` optional |

**Virtual wrapper tools** normalize these into a common `NormalizedSearchResponse` format:
- `normalized_exa_search` - wraps `exa_search` with output transform
- `normalized_arxiv_search` - wraps `arxiv_search` with output transform
- `normalized_github_search` - wraps `github_search` with output transform
- `normalized_huggingface_search` - wraps `huggingface_search` with output transform

Each wrapper uses JSONPath mappings to transform native fields (e.g., `$.papers[*].abstract` → `$.results[*].snippet`).

### Fetch Service (8002)

Content retrieval and processing:

| Tool | Function |
|------|----------|
| `url_fetch` | Fetch URL content, extract text and metadata |
| `extract_urls` | Regex-based URL extraction with domain filtering |
| `batch_fetch` | Parallel fetch of multiple URLs |

### Entity Service (8003)

Knowledge graph with vector search:

| Tool | Function |
|------|----------|
| `entity_search` | Semantic search over entity descriptions |
| `create_entity` | Add entity (concept, paper, person, etc.) |
| `create_relation` | Link entities with predicates |
| `search_relations` | Find connections for an entity |

Uses sqlite-vec for efficient vector similarity search.

### Category Service (8004)

Hierarchical taxonomy:

| Tool | Function |
|------|----------|
| `search_categories` | Semantic category matching |
| `create_category` | Add category with optional parent |
| `get_category_tree` | Navigate taxonomy structure |

Categories support parent-child relationships with materialized paths.

### Tag Service (8005)

Content organization:

| Tool | Function |
|------|----------|
| `register_content` | Register URL/content for tagging |
| `tag_content` | Associate content with categories |
| `search_tagged_content` | Find content by category |
| `search_content` | Semantic search over summaries |

## LLM Configuration

The agent supports multiple LLM providers with automatic detection:

| Priority | Provider | Environment Variable | Default Model |
|----------|----------|---------------------|---------------|
| 1 | Anthropic | `ANTHROPIC_API_KEY` | claude-sonnet-4-20250514 |
| 2 | OpenAI | `OPENAI_API_KEY` | gpt-4o |
| 3 | Google | `GOOGLE_API_KEY` or `GEMINI_API_KEY` | gemini-2.0-flash |

Override with:
- `LLM_PROVIDER`: Force specific provider
- `LLM_MODEL`: Use specific model

## Extending the Demo

### Adding a New Search Source

1. Add tool to `mcp_tools/search_service/server.py`:

```python
@mcp.tool()
async def my_search(query: str, num_results: int = 10) -> dict:
    # Implement search...
    return {"query": query, "results": [...]}
```

2. Add to scatter-gather in `research_registry.json`:

```json
{
  "name": "multi_source_search",
  "spec": {
    "scatterGather": {
      "targets": [
        // ... existing targets ...
        {"tool": "my_search", "server": "search-service"}
      ]
    }
  }
}
```

### Adding a New Composite Tool

Add to `tools` array in `research_registry.json`:

```json
{
  "name": "my_composite_tool",
  "version": "1.0.0",
  "description": "What it does",
  "spec": {
    "pipeline": {
      "steps": [
        // Define steps...
      ]
    }
  },
  "inputSchema": { ... },
  "outputSchema": { ... }
}
```

## Troubleshooting

### Services not starting

```bash
# Check if ports are in use
lsof -i :8001 -i :8002 -i :8003 -i :8004 -i :8005 -i :3000 -i :9001

# Kill any orphaned processes
./stop_services.sh
```

### Gateway can't connect to services

```bash
# Verify services are running
curl http://localhost:8001/mcp  # Should respond
curl http://localhost:8002/mcp
# etc.

# Check gateway logs
tmux attach -t research-demo
# Press Ctrl+B, then 1 for gateway window
```

### No LLM configured

```bash
# Check your .env file has at least one API key:
cat .env | grep API_KEY

# Test LLM config
cd examples/research-assistant-demo
uv run python -c "from agents.shared.llm_config import print_llm_config; print_llm_config()"
```

### Search returns errors

Each search tool returns errors in the response body (not exceptions). Check the `error` field:

```json
{
  "query": "transformers",
  "source": "exa",
  "error": "EXA_API_KEY environment variable not set. Get an API key at https://exa.ai",
  "results": []
}
```

Common errors:
- `EXA_API_KEY environment variable not set` - Set the API key in `.env`
- `GitHub API error: HTTP 403 (rate limited)` - Set `GITHUB_TOKEN` for higher limits
- `arXiv API timeout` - arXiv can be slow; retry or increase timeout

## Files Reference

```
research-assistant-demo/
├── agents/
│   ├── research_agent/
│   │   ├── agent.py           # Agent definition and system prompt
│   │   └── __main__.py        # A2A server entry point
│   └── shared/
│       └── llm_config.py      # LLM provider configuration
├── mcp_tools/
│   ├── search_service/        # External search APIs
│   ├── fetch_service/         # URL fetching
│   ├── entity_service/        # Knowledge graph
│   ├── category_service/      # Taxonomy
│   ├── tag_service/           # Content tagging
│   └── shared/
│       ├── db_utils.py        # SQLite helpers
│       ├── embeddings.py      # Vector embedding utilities
│       └── http_runner.py     # MCP server runner
├── gateway-configs/
│   ├── config.yaml            # Gateway configuration
│   └── research_registry.json # Virtual tools definitions
├── data/
│   └── seed_data.py           # Database initialization
├── start_services.sh          # Start all services
├── stop_services.sh           # Stop all services
├── pyproject.toml             # Python dependencies
├── .env.example               # Environment template
└── README.md                  # This file
```
