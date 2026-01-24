# FastMCP Transforms vs AgentGateway vMCP: Design Comparison

**Status**: Analysis Document
**Date**: 2026-01-21
**Context**: FastMCP 3.0 introduced a transforms system for modifying MCP components. This document compares it with AgentGateway's vMCP (virtual tools and compositions) to identify overlap, gaps, and the implications for different stakeholders.

## Executive Summary

FastMCP 3.0 transforms and AgentGateway's vMCP serve **complementary rather than competing roles**:

| System | Where It Runs | Primary User | Scope |
|--------|---------------|--------------|-------|
| **FastMCP Transforms** | In MCP server process | Server authors | Single-server tool shaping |
| **AgentGateway vMCP** | Proxy/gateway layer | Agent authors, governance | Cross-server orchestration |

FastMCP transforms can replace ~20-30% of vMCP's 1:1 aliasing features, but **cannot replace** compositions, output transformations, or gateway-level orchestration—the core differentiators.

---

## 1. Feature Comparison Matrix

### 1.1 Tool Transformation Capabilities

| Capability | AgentGateway vMCP | FastMCP Transforms | Winner |
|------------|-------------------|-------------------|--------|
| **Tool Renaming** | `source.tool` mapping | `ToolTransform` name override | Tie |
| **Description Override** | `description` field | `ToolTransformConfig.description` | Tie |
| **Hide Arguments** | `hideFields` array | `ArgTransform(hide=True)` | FastMCP (more ergonomic) |
| **Inject Defaults** | `defaults` with `${VAR}` substitution | `ArgTransform(default=...)` | Tie (different strengths) |
| **Argument Renaming** | Via `inputSchema` override | `ArgTransform(name=...)` | FastMCP (cleaner API) |
| **Namespace Prefixing** | Not built-in | `Namespace` transform | FastMCP |
| **Custom Transform Logic** | Rust execution layer | `transform_fn` Python callable | FastMCP (more accessible) |

### 1.2 Output Transformation

| Capability | AgentGateway vMCP | FastMCP Transforms |
|------------|-------------------|-------------------|
| **JSONPath Field Extraction** | `outputTransform.mappings` | **Not supported** |
| **Field Renaming** | `{"newField": {"path": "$.oldField"}}` | **Not supported** |
| **Coalesce (first non-null)** | `{"coalesce": ["$.a", "$.b"]}` | **Not supported** |
| **Template Interpolation** | `{"template": "${x} ${y}", "vars": {...}}` | **Not supported** |
| **Array Element Mapping** | `repos[*].name` syntax | **Not supported** |

**Verdict**: AgentGateway has a significant capability gap that FastMCP does not address.

### 1.3 Composition Patterns (N:1)

| Pattern | AgentGateway vMCP | FastMCP Transforms |
|---------|-------------------|-------------------|
| **Pipeline** | Sequential steps with data binding | **None** |
| **Scatter-Gather** | Parallel execution + aggregation | **None** |
| **Filter** | Predicate-based array filtering | **None** |
| **SchemaMap** | JSONPath field transformation | **None** |
| **MapEach** | Apply operation to array elements | **None** |
| **Retry/Circuit Breaker** | Roadmap (types defined) | **None** |
| **Saga** | Roadmap (compensation patterns) | **None** |

**Verdict**: Compositions are AgentGateway's core differentiator. FastMCP has no equivalent.

### 1.4 MCP-Specific Transforms

| Capability | AgentGateway vMCP | FastMCP Transforms |
|------------|-------------------|-------------------|
| **Resources as Tools** | N/A (not MCP resources) | `ResourcesAsTools` |
| **Prompts as Tools** | N/A (not MCP prompts) | `PromptsAsTools` |

These are FastMCP-specific features for bridging MCP capabilities to tool-only clients.

---

## 2. Locus of Control Analysis

The critical question isn't just "what can each system do?" but "who controls what, and when?"

### 2.1 The Three Stakeholders

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Control Hierarchy                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────┐                                                       │
│  │  MCP SERVER      │  "I built this tool, here's its interface"            │
│  │  AUTHOR          │  - Defines native schema, behavior                    │
│  │                  │  - May use FastMCP transforms for internal shaping    │
│  └────────┬─────────┘                                                       │
│           │                                                                  │
│           │ publishes                                                        │
│           ▼                                                                  │
│  ┌──────────────────┐                                                       │
│  │  GOVERNANCE      │  "These tools are approved for these agents"          │
│  │  BODY            │  - Curates which tools agents can access              │
│  │  (Platform/Org)  │  - Defines virtual tools (aliases, compositions)      │
│  │                  │  - Enforces policies (rate limits, auth, audit)       │
│  └────────┬─────────┘                                                       │
│           │                                                                  │
│           │ exposes via registry                                             │
│           ▼                                                                  │
│  ┌──────────────────┐                                                       │
│  │  AGENT           │  "I need to accomplish this task with these tools"    │
│  │  AUTHOR          │  - Sees curated tool surface                          │
│  │                  │  - May compose tools for specific workflows           │
│  │                  │  - Trusts governance layer for security               │
│  └──────────────────┘                                                       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Who Controls What?

| Concern | MCP Server Author | Governance Body | Agent Author |
|---------|-------------------|-----------------|--------------|
| **Native tool implementation** | ✅ Full control | ❌ | ❌ |
| **Server-side transforms** | ✅ FastMCP transforms | ❌ | ❌ |
| **Tool visibility/access** | ❌ | ✅ Gateway config | ❌ |
| **Virtual tool definitions** | ❌ | ✅ Registry | ⚠️ Can request |
| **Composition patterns** | ❌ | ✅ Registry | ⚠️ Can define own |
| **Output shaping for context** | ❌ | ✅ outputTransform | ⚠️ Post-process |
| **Rate limits, auth, audit** | ⚠️ Can implement | ✅ Gateway policy | ❌ |
| **Cross-server workflows** | ❌ | ✅ Compositions | ⚠️ Via registry |

### 2.3 FastMCP Transforms: Server Author Control

FastMCP transforms give **server authors** fine-grained control over how their tools appear:

```python
# Server author controls this (at server level)
from fastmcp import FastMCP
from fastmcp.server.transforms import ToolTransform, Namespace

mcp = FastMCP("CRM Server")

@mcp.tool
def query_contacts(soql: str) -> list:
    """Execute SOQL query against contacts"""
    ...

# Server author decides to expose a friendlier interface
mcp.add_transform(ToolTransform({
    "query_contacts": ToolTransformConfig(
        name="search_contacts",
        description="Search for contacts by name or email"
    )
}))

mcp.add_transform(Namespace("crm"))  # All tools get crm_ prefix
```

**Implications:**
- Server authors can hide internal complexity
- Server authors control their tools' "API surface"
- Changes require server redeployment
- Governance/agents must accept what server provides

### 2.4 AgentGateway vMCP: Governance/Agent Control

vMCP gives **governance bodies and agent authors** control without modifying servers:

```json
{
  "tools": [
    {
      "name": "get_customer_emails",
      "description": "Get email addresses for customers at a company",
      "source": {
        "target": "salesforce-server",
        "tool": "query_contacts"
      },
      "defaults": {
        "soql": "SELECT Email, FirstName, LastName FROM Contact WHERE Account.Name = '${company}'"
      },
      "hideFields": ["soql"],
      "outputTransform": {
        "mappings": {
          "emails[*].address": {"path": "$.Email"},
          "emails[*].name": {"template": "${FirstName} ${LastName}"}
        }
      }
    }
  ]
}
```

**Implications:**
- Governance can curate tools without server cooperation
- Agent authors can shape tools for their specific context window budget
- Changes don't require server redeployment
- Can compose tools from servers that don't know about each other

### 2.5 The Trust Boundary Question

```
                    Trust Boundary
                          │
    Server Author         │         Governance / Agent Author
    Controls              │         Controls
                          │
    ┌─────────────────┐   │   ┌─────────────────────────────────┐
    │ FastMCP Server  │   │   │        AgentGateway             │
    │                 │   │   │                                 │
    │ @mcp.tool       │   │   │  Registry:                      │
    │ def raw_tool()  │──►│──►│  - Virtual tools                │
    │                 │   │   │  - Compositions                 │
    │ Transforms:     │   │   │  - Output transforms            │
    │ - Namespace     │   │   │                                 │
    │ - ArgTransform  │   │   │  Policy:                        │
    │ - ToolTransform │   │   │  - Rate limits                  │
    │                 │   │   │  - Auth/authz                   │
    └─────────────────┘   │   │  - Audit logging                │
                          │   └─────────────────────────────────┘
                          │                 │
                          │                 ▼
                          │         ┌───────────────┐
                          │         │    Agent      │
                          │         │               │
                          │         │ Sees curated  │
                          │         │ tool surface  │
                          │         └───────────────┘
```

**Key Insight**: The governance body operates at a trust boundary where they:
1. Don't necessarily trust server authors to expose the right interface
2. Don't necessarily trust agents to handle raw tool outputs responsibly
3. Need to enforce organizational policies regardless of server implementation

FastMCP transforms operate **inside** the server author's domain.
AgentGateway vMCP operates **at the trust boundary** under governance control.

---

## 3. Use Case Analysis

### 3.1 When to Use FastMCP Transforms

**Scenario**: You're building an MCP server and want to expose a clean interface.

```python
# You control the server - use FastMCP transforms
@mcp.tool
def internal_data_processor(
    query: str,
    internal_format: str = "json",  # Hide this
    debug_mode: bool = False,        # Hide this
    api_version: int = 3             # Hide this
) -> dict:
    ...

mcp.add_transform(ToolTransform({
    "internal_data_processor": ToolTransformConfig(
        name="process_data",
        description="Process data with a natural language query"
    )
}))
```

**Best for:**
- Server authors shaping their own tool interfaces
- Hiding implementation details at the source
- Namespace isolation when composing servers you control

### 3.2 When to Use AgentGateway vMCP

**Scenario A**: You're a platform team curating tools from various vendors.

```json
{
  "tools": [
    {
      "name": "unified_search",
      "description": "Search across all knowledge bases",
      "spec": {
        "scatterGather": {
          "targets": [
            {"tool": "confluence_search"},
            {"tool": "sharepoint_search"},
            {"tool": "slack_search"}
          ],
          "aggregation": {
            "ops": [
              {"flatten": true},
              {"dedupe": {"field": "$.url"}},
              {"sort": {"field": "$.relevance", "order": "desc"}},
              {"limit": {"count": 20}}
            ]
          }
        }
      }
    }
  ]
}
```

**Best for:**
- Composing tools from servers you don't control
- Enforcing organizational policies (what tools agents can see)
- Optimizing LLM context with output transforms
- Creating higher-level workflows from primitive tools

**Scenario B**: You're an agent author optimizing for context window.

```json
{
  "name": "get_repo_summary",
  "source": {"target": "github", "tool": "get_repository"},
  "outputTransform": {
    "mappings": {
      "name": {"path": "$.full_name"},
      "description": {"path": "$.description"},
      "stars": {"path": "$.stargazers_count"},
      "url": {"path": "$.html_url"}
    }
  }
}
```

Reduces 80+ field GitHub response to 4 fields the agent actually needs.

### 3.3 Hybrid Deployment

The systems work together:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Agent Client                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          AgentGateway                                    │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │  Registry (vMCP)                                                   │  │
│  │  - Cross-server compositions (pipeline, scatter-gather)           │  │
│  │  - Output transforms (JSONPath extraction)                        │  │
│  │  - Governance policies (visibility, rate limits)                  │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
           │                        │                        │
           ▼                        ▼                        ▼
   ┌───────────────┐       ┌───────────────┐       ┌───────────────┐
   │  FastMCP      │       │  FastMCP      │       │  Raw MCP      │
   │  Server A     │       │  Server B     │       │  Server C     │
   │               │       │               │       │               │
   │  Transforms:  │       │  Transforms:  │       │  (No control  │
   │  - Namespace  │       │  - Hide args  │       │   over this   │
   │  - Clean API  │       │  - Defaults   │       │   server)     │
   └───────────────┘       └───────────────┘       └───────────────┘
```

**Principle**:
- FastMCP transforms handle **server-local concerns** (what the server author wants to expose)
- AgentGateway vMCP handles **gateway-level concerns** (what governance/agents need)

---

## 4. Implications for AgentGateway Development

### 4.1 What to Keep (Core Differentiators)

These capabilities have **no FastMCP equivalent** and represent AgentGateway's unique value:

1. **Composition Patterns**
   - Pipeline, ScatterGather, Filter, SchemaMap, MapEach
   - Stateful patterns roadmap (Retry, Cache, CircuitBreaker, Saga)
   - These enable declarative multi-tool workflows

2. **Output Transformations**
   - JSONPath-based field extraction
   - Template interpolation
   - Array mapping
   - Critical for LLM context optimization

3. **Gateway-Level Operation**
   - Transform tools from servers you don't control
   - Cross-server compositions
   - Centralized governance and policy enforcement

4. **Registry System**
   - Versioned tool definitions
   - SBOM tracking
   - Dependency-scoped discovery

### 4.2 What Could Be Simplified

For users already using FastMCP servers, some overlap could be acknowledged:

| Current vMCP Feature | FastMCP Alternative | Recommendation |
|---------------------|---------------------|----------------|
| Simple tool aliasing | `ToolTransform` | Document that FastMCP users can do this at server level |
| `hideFields` | `ArgTransform(hide=True)` | Keep for non-FastMCP servers |
| `defaults` | `ArgTransform(default=...)` | Keep - supports `${ENV_VAR}` substitution |
| Namespace prefixing | `Namespace` transform | Consider adding if needed |

### 4.3 Documentation Recommendations

Add guidance for users:

```markdown
## Choosing Where to Transform

| If you... | Use |
|-----------|-----|
| Control the MCP server code | FastMCP transforms (server-side) |
| Need to compose multiple servers | AgentGateway registry (gateway-side) |
| Need to reduce output size | AgentGateway outputTransform |
| Need governance/policy enforcement | AgentGateway policies |
| Are using servers you don't control | AgentGateway registry |
```

### 4.4 Potential Future Integration

Consider deeper integration paths:

1. **FastMCP Server Auto-Discovery**
   - Gateway could introspect FastMCP servers to understand their transform capabilities
   - Avoid double-transforming (server transforms + gateway transforms)

2. **Transform Delegation**
   - For FastMCP servers, gateway could delegate simple transforms to the server
   - Gateway handles compositions and cross-server orchestration

3. **Unified DSL**
   - TypeScript DSL could generate either FastMCP Python or registry JSON
   - Same mental model, different deployment targets

---

## 5. The Sidecar Pattern: Challenging the Dichotomy

A key architectural pattern challenges the clean "server author vs gateway" separation:

### 5.1 FastMCP as Agent-Local Proxy

FastMCP can run as a **sidecar proxy** next to an agent, connecting to remote MCP servers:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Agent Host                                │
│  ┌─────────────┐      ┌─────────────────────────────────────┐   │
│  │   Agent     │◄────►│  FastMCP Sidecar                    │   │
│  │             │ stdio│                                     │   │
│  │             │  or  │  - ToolTransform (rename, hide)     │   │
│  └─────────────┘ http │  - ArgTransform (defaults)          │   │
│                       │  - Custom transform_fn (Python)     │   │
│                       └──────────────┬──────────────────────┘   │
└──────────────────────────────────────│──────────────────────────┘
                                       │ streamable-http
                                       ▼
                              ┌─────────────────┐
                              │  Remote MCP     │
                              │  Server         │
                              │  (no control)   │
                              └─────────────────┘
```

```python
from fastmcp import FastMCP
from fastmcp.server.proxy import ProxyServer

# Agent-local sidecar that proxies to remote server
sidecar = FastMCP("Agent Sidecar")

# Proxy to remote MCP server (streamable-http)
sidecar.mount_proxy("https://remote-mcp.example.com/sse", namespace="remote")

# Agent author controls these transforms
sidecar.add_transform(ToolTransform({
    "remote_query_contacts": ToolTransformConfig(
        name="get_customers",
        description="Get customer list"
    )
}))

# Can even add custom Python logic
@sidecar.tool
async def enriched_search(query: str) -> dict:
    """Search and enrich results"""
    # Call proxied tool
    results = await sidecar.call_tool("remote_search", {"q": query})
    # Post-process (output transformation via code)
    return {
        "items": [{"title": r["name"], "url": r["link"]} for r in results],
        "count": len(results)
    }
```

### 5.2 Revised Locus of Control

This pattern shifts control to the **agent author**:

| Capability | Server-Embedded FastMCP | Sidecar FastMCP | AgentGateway |
|------------|------------------------|-----------------|--------------|
| **Who deploys** | Server author | Agent author | Platform/governance |
| **Who controls transforms** | Server author | Agent author | Governance body |
| **Can transform remote servers** | No | **Yes** | Yes |
| **Output transformation** | Via `transform_fn` | Via `transform_fn` | JSONPath declarative |
| **Compositions** | Imperative Python | Imperative Python | Declarative JSON |
| **Multi-server orchestration** | Manual coding | Manual coding | Built-in patterns |
| **Governance enforcement** | None | None | Policy layer |

### 5.3 What the Sidecar Pattern CAN Do

With FastMCP as a sidecar, agent authors gain:

1. **Input transforms on remote servers** - rename args, hide fields, inject defaults
2. **Output transforms via Python** - custom `transform_fn` can reshape responses
3. **Custom compositions** - write Python code to orchestrate multiple calls
4. **Namespace isolation** - prefix tools from different remotes

```python
# Output transformation via transform_fn
from fastmcp.tools import Tool
from fastmcp.tools.tool_transform import forward

async def slim_output(query: str) -> list:
    result = await forward(query=query)  # Call original
    # Equivalent to AgentGateway's outputTransform
    return [{"name": r["full_name"], "stars": r["stargazers_count"]}
            for r in result.get("items", [])]

slim_search = Tool.from_tool(
    original_search_tool,
    name="search",
    transform_fn=slim_output
)
```

### 5.4 What the Sidecar Pattern CANNOT Do

Even with sidecar deployment, FastMCP lacks:

| Gap | Why It Matters |
|-----|----------------|
| **Declarative compositions** | Must write Python code for pipelines, scatter-gather |
| **Cross-server data binding** | No `$.steps.search.items[0]` style references |
| **Aggregation primitives** | No built-in flatten, dedupe, sort, limit |
| **Governance separation** | Agent author and sidecar controller are same person |
| **Centralized policy** | Each agent has its own sidecar with its own rules |
| **SBOM/dependency tracking** | No registry of what tools exist or their versions |

### 5.5 The Governance Gap

The critical difference is **organizational control**:

```
Sidecar Pattern:                    Gateway Pattern:

Agent Author ──► Sidecar            Governance ──► Gateway ──► Agents
     │              │                    │             │
     └──────────────┘                    │             │
     Same person/team                    └─────────────┘
                                         Different roles
```

**Sidecar**: Agent authors transform for their own convenience
**Gateway**: Governance transforms for organizational control

An enterprise might want:
- "All agents MUST use the `approved_search` virtual tool, not raw `web_search`"
- "All Salesforce queries MUST go through `curated_contacts`, never raw SOQL"
- "These 5 agents can see these 20 tools; those 3 agents see these 10 tools"

The sidecar pattern gives this control to individual agent authors, not governance.

### 5.6 Architectural Comparison

| Deployment | Who Controls | Best For |
|------------|--------------|----------|
| **FastMCP in server** | Server author | Clean API from source |
| **FastMCP as sidecar** | Agent author | Individual agent customization |
| **AgentGateway** | Governance/platform | Organizational policy, multi-agent |

### 5.7 Hybrid: Sidecar + Gateway

Both can coexist:

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Agent Host                                 │
│  ┌─────────────┐      ┌──────────────────┐                          │
│  │   Agent     │◄────►│  FastMCP Sidecar │  Agent-specific          │
│  │             │      │  (optional)      │  customizations          │
│  └─────────────┘      └────────┬─────────┘                          │
└────────────────────────────────│────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        AgentGateway                                  │
│  - Governance-approved virtual tools                                 │
│  - Cross-server compositions                                         │
│  - Policy enforcement (auth, rate limits, audit)                     │
│  - SBOM tracking                                                     │
└─────────────────────────────────────────────────────────────────────┘
                                 │
              ┌──────────────────┼──────────────────┐
              ▼                  ▼                  ▼
       ┌───────────┐      ┌───────────┐      ┌───────────┐
       │  MCP      │      │  MCP      │      │  MCP      │
       │  Server A │      │  Server B │      │  Server C │
       └───────────┘      └───────────┘      └───────────┘
```

In this model:
- **Gateway** enforces what's allowed (governance)
- **Sidecar** customizes within allowed bounds (agent convenience)

---

## 6. Summary

### What FastMCP Transforms Do Well

- Clean Python API for transforms
- Ergonomic argument hiding/renaming
- First-class namespace support
- Custom `transform_fn` for arbitrary Python logic
- **Can run as sidecar proxy** to remote servers (not just server-embedded)

### What AgentGateway vMCP Does That FastMCP Cannot

- **Declarative compositions** (pipeline, scatter-gather) without imperative code
- **JSONPath-based output transforms** as configuration, not code
- **Cross-server data binding** (`$.steps.search.items[0]`)
- **Aggregation primitives** (flatten, dedupe, sort, limit)
- **Governance separation** from agent authors
- **Centralized policy enforcement** across all agents
- **SBOM/dependency tracking** and versioned tool registry

### The Locus of Control Principle (Revised)

The control landscape has **three deployment patterns**, not two:

| Pattern | Controller | Governance | Best For |
|---------|-----------|------------|----------|
| **FastMCP in server** | Server author | None | Clean API from source |
| **FastMCP as sidecar** | Agent author | None | Individual customization |
| **AgentGateway** | Governance body | Built-in | Organizational policy |

**Key insight**: FastMCP sidecar gives agent authors transform capabilities, but doesn't provide governance separation. An enterprise platform team wants to control what tools agents can access and how—that requires the gateway pattern.

### When to Use What

| Scenario | Recommendation |
|----------|----------------|
| You author an MCP server | FastMCP transforms in-server |
| You're an agent author wanting custom tool shapes | FastMCP sidecar |
| You need cross-server compositions | AgentGateway |
| You need organizational governance over tools | AgentGateway |
| You need declarative (not imperative) orchestration | AgentGateway |
| You want both governance + agent customization | Gateway + optional sidecar |

**They are complementary, serving different roles in the control hierarchy.**

---

## References

- [FastMCP Transforms Documentation](https://gofastmcp.com/servers/transforms/transforms)
- [AgentGateway Virtual Tools](../virtual-tools.md)
- [vMCP Compositional Algebra](../mcp-algebra-ala-camel.md)
- [Registry v2 Design](./registry-v2.md)
