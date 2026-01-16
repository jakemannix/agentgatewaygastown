# Virtual Tools and Registry

This document describes the virtual tools system in agentgateway, which provides tool aliasing, input/output transformation, and composition patterns.

## Overview

The registry system allows you to:

1. **Alias tools** - Expose backend tools under different names
2. **Transform inputs** - Inject defaults, hide fields, use templates
3. **Transform outputs** - Restructure responses using JSONPath mappings
4. **Compose tools** - Combine multiple tools into pipelines or parallel operations

## Running the Demo

```bash
# Build (debug mode for faster iteration)
cargo build

# Run the gateway with the demo config
./target/debug/agentgateway -f demo/agentgateway-demo.yaml

# Or with GitHub API access (needed for search_repositories, search_repos):
GITHUB_TOKEN=your_token_here ./target/debug/agentgateway -f demo/agentgateway-demo.yaml

# In another terminal, run the demo UI (from mcp-gateway-prototype repo)
cd ~/src/open_src/mcp-gateway-prototype/demo/ui
GATEWAY_BACKEND=agentgateway GATEWAY_URL=http://localhost:3000 uv run python main.py
```

The demo UI will be available at http://localhost:8501

### API Keys

Some MCP servers require API keys:
- **github-server**: Set `GITHUB_TOKEN` env var for `search_repositories`, `search_repos`, and `research_github_repo` pipeline
- **cloudflare-radar**: Requires OAuth (will prompt in browser)
- **cloudflare-docs**: No auth needed
- **fetch-server**, **time-server**, **memory-server**: No auth needed

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     MCP Client (Agent)                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      AgentGateway                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                    Registry                          │    │
│  │  - Virtual tool definitions                          │    │
│  │  - Input transformations (defaults, templates)       │    │
│  │  - Output transformations (JSONPath mappings)        │    │
│  │  - Compositions (pipelines, scatter-gather)          │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │  MCP     │   │  MCP     │   │  Remote  │
        │  Server  │   │  Server  │   │  MCP     │
        │ (stdio)  │   │ (stdio)  │   │ (HTTP)   │
        └──────────┘   └──────────┘   └──────────┘
```

## Configuration

### Gateway Config (YAML)

The gateway config (`demo/agentgateway-demo.yaml`) defines:
- MCP server connections (targets)
- Registry source location

```yaml
binds:
- port: 3000
  listeners:
  - routes:
    - backends:
      - mcp:
          targets:
          - name: time-server
            stdio:
              cmd: uvx
              args: ["mcp-server-time"]
          - name: fetch-server
            stdio:
              cmd: uvx
              args: ["mcp-server-fetch"]

registry:
  source: file://./demo/registries/showcase.json
  refreshInterval: 30s
```

### Registry (JSON)

The registry (`demo/registries/showcase.json`) defines virtual tools:

```json
{
  "schemaVersion": "1.0",
  "tools": [
    { /* tool definitions */ }
  ]
}
```

## Tool Definition Types

### 1. Simple Alias

Maps a virtual tool name to a backend tool:

```json
{
  "name": "browse",
  "description": "Browse a website",
  "source": {"target": "fetch-server", "tool": "fetch"}
}
```

### 2. With Input Transformation

Inject defaults, use templates, or hide fields:

```json
{
  "name": "get_github_repo",
  "description": "Fetch GitHub repo info",
  "source": {"target": "fetch-server", "tool": "fetch"},
  "inputSchema": {
    "type": "object",
    "properties": {
      "owner": {"type": "string"},
      "repo": {"type": "string"}
    },
    "required": ["owner", "repo"]
  },
  "defaults": {
    "url": "https://api.github.com/repos/${owner}/${repo}"
  }
}
```

The `${owner}` and `${repo}` placeholders are replaced with input values.

### 3. With Output Transformation

Transform backend responses using JSONPath:

```json
{
  "name": "get_time_structured",
  "source": {"target": "time-server", "tool": "get_current_time"},
  "outputSchema": {
    "type": "object",
    "properties": {
      "timezone": {"type": "string"},
      "datetime": {"type": "string"},
      "day_of_week": {"type": "string"},
      "is_dst": {"type": "boolean"}
    }
  },
  "outputTransform": {
    "mappings": {
      "timezone": {"path": "$.timezone"},
      "datetime": {"path": "$.datetime"},
      "day_of_week": {"path": "$.day_of_week"},
      "is_dst": {"path": "$.is_dst"}
    }
  }
}
```

**Important distinction:**
- `outputSchema` - **WHAT** the output looks like (JSON Schema sent to MCP clients)
- `outputTransform` - **HOW** to generate the output (JSONPath mappings, internal to gateway)

If a virtual tool declares an `outputSchema` but the backend tool doesn't have one, you **must** provide an `outputTransform` to define how to create the structured output.

### 4. Compositions

Combine multiple tools into a single virtual tool.

#### Pipeline

Sequential execution with data flow between steps:

```json
{
  "name": "research_pipeline",
  "description": "Search GitHub then fetch the top result",
  "spec": {
    "pipeline": {
      "steps": [
        {
          "id": "search",
          "operation": {"tool": {"name": "search_repositories"}}
        },
        {
          "id": "fetch_top",
          "operation": {"tool": {"name": "fetch"}},
          "input": {"step": {"stepId": "search", "path": "$.items[0].html_url"}}
        }
      ]
    }
  },
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {"type": "string", "description": "GitHub search query"}
    },
    "required": ["query"]
  }
}
```

#### Scatter-Gather

Parallel execution with result aggregation:

```json
{
  "name": "multi_search",
  "description": "Search multiple sources in parallel",
  "spec": {
    "scatterGather": {
      "targets": [
        {"tool": "search_github"},
        {"tool": "search_docs"}
      ],
      "aggregation": {
        "ops": [{"flatten": true}]
      },
      "timeoutMs": 10000,
      "failFast": false
    }
  }
}
```

## Output Transform Mappings

The `outputTransform.mappings` field supports several patterns:

### JSONPath Extraction

```json
{"path": "$.result.title"}
```

### Literal Values

```json
{"literal": {"stringValue": "web"}}
```

### Coalesce (First Non-Null)

```json
{"coalesce": ["$.primary", "$.fallback", "$.default"]}
```

### Template Interpolation

```json
{"template": "${name} (${count} items)", "vars": {"name": "$.title", "count": "$.total"}}
```

### Array Item Mapping

For transforming arrays, use the `[*]` syntax:

```json
{
  "repos": {"path": "$.items[*]"},
  "repos[*].name": {"path": "$.full_name"},
  "repos[*].stars": {"path": "$.stargazers_count"}
}
```

## Available Pattern Types

| Pattern | Description | Status |
|---------|-------------|--------|
| `pipeline` | Sequential steps with data flow | Implemented |
| `scatterGather` | Parallel calls with aggregation | Implemented |
| `filter` | Filter array elements by predicate | Implemented |
| `schemaMap` | Transform fields using mappings | Implemented |
| `mapEach` | Apply operation to each array element | Implemented |

## Debug Logging

Enable verbose logging for virtual tools:

```bash
RUST_LOG=virtual_tools=debug ./target/debug/agentgateway -f demo/agentgateway-demo.yaml
```

This shows:
- Tool resolution (virtual -> backend)
- Input transformation (defaults injection)
- Output transformation (JSONPath extraction)
- Composition execution

## File Locations

| File | Purpose |
|------|---------|
| `demo/agentgateway-demo.yaml` | Gateway config with MCP targets |
| `demo/registries/showcase.json` | Registry with virtual tool definitions |
| `crates/agentgateway/src/mcp/registry/` | Rust implementation |
| `crates/agentgateway/src/mcp/registry/types.rs` | Type definitions |
| `crates/agentgateway/src/mcp/registry/compiled.rs` | Compiled registry and transformations |
| `crates/agentgateway/src/mcp/registry/patterns/` | Composition pattern implementations |
