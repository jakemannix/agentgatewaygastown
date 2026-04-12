---
component: registry-config
parent_feature: research-assistant-demo
type: component
doctrack_version: 3.0.0
status: active
last_updated: 2026-04-12T00:00:00.000Z
files:
  - examples/research-assistant-demo/gateway-configs/research_registry.json
  - >-
    examples/research-assistant-demo/gateway-configs/minimal_research_registry.json
  - examples/research-assistant-demo/gateway-configs/config.yaml
  - examples/research-assistant-demo/gateway-configs/minimal_config.yaml
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Registry Configuration

The registry JSON files define virtual tools using [[decisions/chose-declarative-composition|Declarative Composition]] -- all orchestration is expressed in JSON, no code required. The gateway loads these at startup and compiles the composition patterns.

## Registry Variants

| File | Tools | Purpose |
|------|-------|---------|
| `research_registry.json` | ~40+ (15 virtual + all backend) | Full demo with all composition patterns |
| `minimal_research_registry.json` | 10 | Claude Code usage: `research_and_fetch` + search + KG writes |

Both use `schemaVersion: "2.0"`.

## Registry Structure

The [[interfaces/registry-format|Registry Format]] has three top-level sections:

### 1. schemas

Reusable JSON Schema definitions, referenced by `$ref`:

| Schema | Purpose |
|--------|---------|
| `NormalizedSearchResult` | Common search result: `{title, url, snippet, source, source_type}` |
| `NormalizedSearchResponse` | Wrapper: `{results: NormalizedSearchResult[]}` |
| `Entity` | Knowledge graph entity |
| `Category` | Taxonomy category |

### 2. servers

Backend service declarations mapping names to tool lists:

```json
{
  "name": "search-service",
  "version": "1.0.0",
  "provides": [
    {"tool": "exa_search", "version": "1.0.0", "inputSchema": {...}},
    {"tool": "arxiv_search", "version": "1.0.0"},
    ...
  ]
}
```

Five servers declared: `search-service`, `fetch-service`, `entity-service`, `category-service`, `tag-service`. The `provides` entries can include `inputSchema` overrides (see `exa_search` with its extensive filter options).

### 3. tools

Virtual tool definitions. Four types by composition pattern:

#### Normalizers (outputTransform + arrayMap)

Transform backend-native responses into a common schema. Example for arXiv:

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

**Mapping types used:**
- `coalesce` with `default: ""` -- first non-null from paths, falls back to empty string
- `literal` with `stringValue` -- constant values injected per item
- `arrayMap` with `over`/`each` -- iterate array, apply mappings to each element

| Tool | Source | Array Path | Source Field |
|------|--------|-----------|-------------|
| `web_research` | `exa_search` | `$.results` | `source: "exa"` |
| `normalized_arxiv` | `arxiv_search` | `$.papers` | `source: "arxiv"` |
| `normalized_github` | `github_search` | `$.repos` | `source: "github"` |
| `normalized_huggingface` | `huggingface_search` | `$.models` | `source: "huggingface"` |

#### Scatter-Gather Compositions

Parallel execution with aggregation. Targets can be virtual tools (not just backend tools).

| Tool | Targets | Aggregation |
|------|---------|-------------|
| `multi_source_search` | 4 normalizers | extract -> flatten -> dedupe -> wrap |
| `academic_search` | `normalized_arxiv`, `normalized_huggingface` | extract -> flatten -> dedupe -> wrap |
| `code_search` | `normalized_github`, `normalized_huggingface` | extract -> flatten -> dedupe -> wrap |
| `explore_entity_network` | `get_entity`, `search_relations` | merge |
| `get_knowledge_for_topic` | `entity_search`, `search_categories`, `search_content` | merge |

Key config fields: `timeoutMs`, `failFast: false` (partial results on failure).

#### Pipelines

Sequential steps with data flow via JSONPath step references.

| Tool | Steps | Pattern |
|------|-------|---------|
| `fetch_and_extract` | url_fetch -> extract_urls | `step.stepId` + `path` references |
| `research_and_fetch` | scatter-gather -> batch_fetch -> schemaMap | Nested: step 1 is inline scatter-gather |

Data flow syntax:
- `{"step": {"stepId": "fetch", "path": "$.content"}}` -- reference previous step output
- `{"constant": 3000}` -- literal value injection
- `{"input": {"path": "$"}}` -- pass original input through

#### Forwarding/Alias Tools

Simple pass-through to a backend tool with optional `inputSchema` override for better descriptions:

| Tool | Backend |
|------|---------|
| `store_research_finding` | `entity-service/create_entity` |
| `link_entities` | `entity-service/create_relation` |
| `find_or_create_category` | `category-service/search_categories` |
| `browse_taxonomy` | `category-service/list_root_categories` |
| `create_entity` | `entity-service/create_entity` |
| `create_relation` | `entity-service/create_relation` |
| `create_category` | `category-service/create_category` |
| `tag_content` | `tag-service/tag_content` |

## Gateway Config (YAML)

The YAML config file wires the registry to backend services:

```yaml
registry:
  source: file://./examples/research-assistant-demo/gateway-configs/research_registry.json
  refreshInterval: 30s

binds:
  - port: 3000
    listeners:
      - routes:
          - backends:
              - mcp:
                  targets:
                    - name: search-service
                      mcp:
                        host: http://localhost:8001/mcp
                    # ... 4 more services
```

The `name` in each target must match the `server` names used in `source.server` and tool target declarations in the registry JSON.

## Validation

The gateway validates `outputTransform` mappings against `outputSchema` at startup. Warnings for:
- Transform can produce null where schema requires non-null
- Missing `coalesce.default` for nullable fields

Runtime validation available via `RUST_LOG=info,virtual_tools=debug`.

## Parent

- [[features/research-assistant-demo|Research Assistant Demo]]

## Related

- [[components/research-assistant-demo/backend-services|Backend Services]] -- what these tools compose
- [[components/research-assistant-demo/test-infrastructure|Test Infrastructure]] -- how compositions are tested
- [[interfaces/registry-format|Registry Format]] -- formal specification
- [[concepts/virtual-tool-composition|Virtual Tool Composition]] -- composition patterns
