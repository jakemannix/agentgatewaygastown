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
# Full registry (40+ tools including raw backend tools)
./start_services.sh

# Minimal registry (10 tools — recommended for getting started)
./start_services.sh gateway-configs/minimal_config.yaml
```

This starts all 5 microservices, the gateway, the research agent, and a web UI in a tmux session.

### 4. Use the Web UI

Open [http://localhost:8080](http://localhost:8080) in your browser for an interactive chat interface.

### 5. Or Test via CLI / curl

```bash
# Interactive CLI chat
uv run python chat_cli.py
```

Or test the ADK agent directly:

```bash
# Create a session
SESSION=$(curl -s -X POST http://localhost:9001/apps/research_agent/users/test/sessions | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])")

# Send a message
curl -s -X POST http://localhost:9001/run \
  -H "Content-Type: application/json" \
  -d "{\"app_name\":\"research_agent\",\"user_id\":\"test\",\"session_id\":\"$SESSION\",\"new_message\":{\"role\":\"user\",\"parts\":[{\"text\":\"Search for ColBERT information retrieval\"}]}}"
```

### 6. Monitor & Debug

```bash
# View all service logs in tmux
tmux attach -t research-demo
# Navigate windows: Ctrl+B, then 0/1/2/3 for services/gateway/agent/webui

# Check agent tool call logs (timing)
grep 'research_agent' logs/agent.log

# Check gateway composition logs
grep 'composition' logs/gateway.log

# Stop everything
./stop_services.sh
```

#### Agent Logging

The agent logs tool calls with timing at INFO level and full inputs/outputs at DEBUG:

```bash
# Set in .env to control verbosity
LOG_LEVEL=INFO    # Tool names + elapsed time (default)
LOG_LEVEL=DEBUG   # Also shows tool arguments and results
```

Example output:
```
2026-03-20 22:44:19 research_agent INFO tool_call_start  tool=virtual_multi_source_search
2026-03-20 22:44:20 research_agent INFO tool_call_done   tool=virtual_multi_source_search elapsed=696ms
```

#### Gateway Schema Validation

The gateway validates virtual tool output transforms against their declared `outputSchema` at startup. If a transform can produce null where the schema requires a value, you'll see warnings:

```
WARN virtual_tools outputSchema requires non-null 'snippet' but transform can produce null
  — add a coalesce "default" or change schema type to ["string", "null"]
  tool=normalized_exa field=snippet
```

For runtime validation (per-request), start the gateway with debug logging:

```bash
RUST_LOG=info,virtual_tools=debug ./target/debug/agentgateway -f config.yaml
```

### 7. Claude Code Integration

Add to your `.mcp.json` to use the gateway as an MCP server:

```json
{
  "mcpServers": {
    "research-gateway": {
      "type": "sse",
      "url": "http://localhost:3000/mcp"
    }
  }
}
```

**Key tools for Claude Code:**
- `research_and_fetch` - Primary mega-tool: searches 4 sources + KG context + fetches content
- `multi_source_search` - Search-only (no fetch) for browsing results first
- `create_entity`, `create_category`, `create_relation`, `tag_content` - Build up the knowledge graph

**Tool visibility note:** Claude Code sees all tools (40+) because it doesn't set the `X-Agent-Name` / `X-Agent-Version` headers that enable gateway tool filtering. We rely on clear descriptions to guide tool selection. See `STATUS.md` for details on the filtering mechanism and a potential "default visibility" enhancement for anonymous callers.

## Virtual Tools Showcase

### Declarative Output Transformation

Each search backend returns its **native API format**. The gateway uses `outputTransform` mappings to normalize them into a common schema. This demonstrates declarative data transformation without code:

```json
{
  "name": "normalized_arxiv",
  "source": {"server": "search-service", "tool": "arxiv_search"},
  "outputTransform": {
    "mappings": {
      "results": {
        "arrayMap": {
          "over": "$.papers",
          "each": {
            "title": {"coalesce": {"paths": ["$.title"], "default": ""}},
            "url": {"coalesce": {"paths": ["$.pdf_url", "$.abs_url"], "default": ""}},
            "snippet": {"coalesce": {"paths": ["$.abstract"], "default": ""}},
            "source": {"literal": {"stringValue": "arxiv"}},
            "source_type": {"literal": {"stringValue": "paper"}}
          }
        }
      }
    }
  },
  "outputSchema": {"$ref": "#/schemas/NormalizedSearchResponse"}
}
```

**Available mapping types:**
- `path`: JSONPath extraction (field renaming via target key)
- `literal`: Constant values (`stringValue`, `numberValue`, `boolValue`)
- `coalesce`: First non-null from multiple paths, with optional `"default"` fallback
- `nested`: Recursive object construction
- `template`: String interpolation with variables
- `concat`: Concatenate multiple paths
- `arrayMap`: Iterate over an array and apply mappings to each element

### Scatter-Gather: Parallel Multi-Source Search

The `multi_source_search` tool demonstrates parallel execution using normalized wrapper tools:

```json
{
  "name": "multi_source_search",
  "spec": {
    "scatterGather": {
      "targets": [
        {"tool": "normalized_exa"},
        {"tool": "normalized_arxiv"},
        {"tool": "normalized_github"},
        {"tool": "normalized_huggingface"}
      ],
      "aggregation": {
        "ops": [
          {"extract": {"path": "$.results"}},
          {"flatten": true},
          {"dedupe": {"field": "$.url"}},
          {"wrap": {"field": "results"}}
        ]
      },
      "timeoutMs": 30000,
      "failFast": false
    }
  },
  "outputSchema": {"$ref": "#/schemas/NormalizedSearchResponse"}
}
```

**What it demonstrates:**
- 4 external APIs called simultaneously via normalized wrapper tools
- Each wrapper applies `outputTransform` with `arrayMap` to normalize native responses
- Results extracted from each wrapper's `results` array, flattened, deduplicated by URL
- Wrapped back into `{"results": [...]}` to match the `NormalizedSearchResponse` schema
- `failFast: false` — partial results returned even if some sources fail

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

**Virtual wrapper tools** normalize these into a common `NormalizedSearchResponse` schema:
- `normalized_exa` - wraps `exa_search` with `arrayMap` over `$.results`
- `normalized_arxiv` - wraps `arxiv_search` with `arrayMap` over `$.papers`
- `normalized_github` - wraps `github_search` with `arrayMap` over `$.repos`
- `normalized_huggingface` - wraps `huggingface_search` with `arrayMap` over `$.models`

Each wrapper uses `arrayMap` + `coalesce` (with `"default": ""` fallbacks) to transform native fields into the common `{title, url, snippet, source, source_type}` schema.

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

Override with `LLM_MODEL` in `.env`:

```bash
# Use a specific model (LiteLLM format: provider/model)
LLM_MODEL=anthropic/claude-haiku-4-5-20251001  # Cheaper, faster
LLM_MODEL=anthropic/claude-sonnet-4-20250514   # Default if ANTHROPIC_API_KEY set
LLM_MODEL=openai/gpt-4o                        # OpenAI
LLM_MODEL=gemini-2.0-flash                     # Google (no prefix needed)
```

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

### Agent returns HTTP 500

Check `logs/agent.log` for the actual error. Common causes:

**API key not loaded:**
```
AnthropicException - "Your credit balance is too low..."
```
The `.env` file's API key is being overridden by a shell-level export (e.g., from `~/.zshrc`). The `main.py` uses `load_dotenv(override=True)` to handle this, but if you see this error, verify your `.env` key is valid:
```bash
source .env && curl -s https://api.anthropic.com/v1/messages \
  -H "x-api-key: $ANTHROPIC_API_KEY" -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-haiku-4-5-20251001","max_tokens":10,"messages":[{"role":"user","content":"hi"}]}'
```

**MCP session error:**
```
ConnectionError: Failed to create MCP session
```
The gateway wasn't ready when the agent tried to connect. Restart the services — the gateway needs a few seconds to start.

**outputSchema validation:**
```
RuntimeError: Invalid structured content returned by tool virtual_multi_source_search: None is not of type 'string'
```
A tool's output transform is producing null where the `outputSchema` requires a string. Check the gateway startup log for warnings that tell you exactly which field needs a `coalesce` default. See "Gateway Schema Validation" above.

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
cat logs/gateway.log | grep -i 'error\|warn'
```

### No LLM configured

```bash
# Check your .env file has at least one API key:
grep API_KEY .env

# Test LLM config
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
│   │   ├── agent.py           # Agent definition with tool call logging
│   │   └── __main__.py        # A2A server entry point
│   └── shared/
│       ├── a2a_server.py      # A2A server base implementation
│       └── llm_config.py      # LLM provider configuration (load_dotenv override)
├── web_ui/
│   └── chat_app.py            # FastHTML web chat interface
├── mcp_tools/
│   ├── search_service/        # External search APIs (Exa, arXiv, GitHub, HuggingFace)
│   ├── fetch_service/         # URL fetching and content extraction
│   ├── entity_service/        # Knowledge graph with vector search
│   ├── category_service/      # Hierarchical taxonomy
│   ├── tag_service/           # Content tagging
│   └── shared/
│       ├── db_utils.py        # SQLite helpers
│       ├── embeddings.py      # Vector embedding utilities
│       └── http_runner.py     # MCP server runner
├── gateway-configs/
│   ├── config.yaml                    # Full gateway config (40+ tools)
│   ├── minimal_config.yaml            # Minimal config (10 tools, recommended)
│   ├── research_registry.json         # Full virtual tools registry
│   └── minimal_research_registry.json # Minimal registry (research_and_fetch + KG writes)
├── docs/
│   ├── tool-hierarchy.mmd     # Mermaid: tool dependency tree
│   ├── tool-hierarchy.svg     # Rendered SVG
│   ├── data-flow.mmd          # Mermaid: research_and_fetch data flow
│   └── data-flow.svg          # Rendered SVG
├── data/
│   └── seed_data.py           # Database initialization
├── logs/                      # Runtime logs (created by start_services.sh)
├── start_services.sh          # Start all services (accepts config file arg)
├── stop_services.sh           # Stop all services
├── chat_cli.py                # Interactive CLI chat client
├── main.py                    # ADK FastAPI server (logging + dotenv setup)
├── pyproject.toml             # Python dependencies
├── .env.example               # Environment template
└── README.md                  # This file
```

## Integration Tests

Integration tests verify the gateway's virtual tool compositions without requiring an LLM, enabling:
- **CI/CD integration**: Run tests as part of the build pipeline
- **Regression testing**: Verify tool compositions work after gateway changes
- **Fast feedback**: No API costs or rate limits during development

### Test Architecture

```
tests/
├── conftest.py                 # Pytest fixtures: services, gateway, MCP client
├── test_arraymap_transforms.py # ArrayMap field source tests
├── test_scatter_gather.py      # Parallel execution + aggregation
├── test_pipelines.py           # Sequential step execution
├── test_aggregation_ops.py     # Extract, flatten, dedupe, merge
└── test_error_handling.py      # Error cases, partial failures
```

### Prerequisites

1. **Build the gateway** (if not already done):
   ```bash
   cargo build -p agentgateway-app
   ```

2. **Install test dependencies**:
   ```bash
   cd examples/research-assistant-demo
   uv sync --dev
   ```

### Running Tests

```bash
cd examples/research-assistant-demo

# Run all integration tests
uv run pytest tests/ -v

# Run specific test file
uv run pytest tests/test_scatter_gather.py -v

# Run a single test
uv run pytest tests/test_arraymap_transforms.py::test_normalized_github_schema -v

# Run with coverage
uv run pytest tests/ --cov=. --cov-report=html
```

The tests automatically start the backend services and gateway as session-scoped fixtures, so no manual setup is needed.

### Test Categories

| Test File | What It Tests |
|-----------|---------------|
| `test_arraymap_transforms.py` | Each normalized search tool produces common schema (title, url, snippet, source) |
| `test_scatter_gather.py` | Parallel execution, result aggregation, failFast behavior |
| `test_pipelines.py` | Sequential steps, data flow between steps |
| `test_aggregation_ops.py` | Extract, flatten, dedupe, merge operations |
| `test_error_handling.py` | Invalid tools, missing params, partial failures |

### Skipped Tests

Some tests are skipped by default because they require API keys:
- `test_normalized_exa_schema` - Requires `EXA_API_KEY`
- `test_multi_source_search_all_four` - Requires all 4 search API keys

To run these tests, ensure the API keys are set in your environment.
