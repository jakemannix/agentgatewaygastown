# vMCP Tool Integration Algebra ↔ Agentgateway Alignment Analysis

## Executive Summary

The `agentgateway` repository (branch `feature/virtual_tools_and_registry`) provides a **production-grade implementation** of the core **Tool Adapter** pattern from the vMCP algebra. It demonstrates how the algebra's foundational composition primitive can be embedded in a real data plane, providing a clear blueprint for extending toward full compositional support.

**Alignment Score: 8.5/10**

The implementation captures the essential tool adaptation mechanics (renaming, schema transformation, output projection, default injection) but operates at the **data plane level** rather than the **composition orchestration level**. This is intentional and complementary—agentgateway handles the runtime mechanics of tool virtualization, while vMCP provides the declarative patterns for composing those virtualized tools.

---

## Architecture Comparison

### vMCP: Composition Algebra (Conceptual Framework)

```
Agent Layer
    ↓
MCP Registry (discovery)
    ↓
vMCP Runtime (composition engine)
    ├─ Composition patterns (pipeline, scatter-gather, router, etc.)
    ├─ Schema transformation & mediation
    ├─ Error handling & resilience
    └─ Observability
    ↓
Primitive MCP Servers
```

**Focus**: Declarative composition of tools without imperative glue code

### Agentgateway: Data Plane Virtualization (Production Reality)

```
Agent/LLM
    ↓
Agentgateway (proxy + policy enforcement)
    ├─ Local config (file-based + hot-reload)
    ├─ XDS control plane (remote configuration)
    ├─ MCP Handler + Tool Registry
    ├─ Tool transformation (rename, schema filter, output projection)
    ├─ Policy enforcement (CEL-based)
    └─ Observability (metrics, tracing)
    ↓
Backends (MCP servers, LLMs, HTTP services)
```

**Focus**: Runtime proxy with security, observability, and governance

### Key Insight: Layered Separation of Concerns

```
┌─────────────────────────────────────────────────────────────────┐
│ vMCP Composition Algebra                                         │
│ (What we're proposing)                                          │
├─────────────────────────────────────────────────────────────────┤
│ • Declarative patterns (Pipeline, Router, Scatter-Gather, etc.) │
│ • Schema mediation & transformation                             │
│ • Pattern composition algebra (types, inference)                │
│ • Runtime execution planning & optimization                    │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ Agentgateway Virtual Tools & Registry                           │
│ (Production implementation starting point)                      │
├─────────────────────────────────────────────────────────────────┤
│ • Tool adaptation (rename, hide fields, transform output)      │
│ • Default injection                                             │
│ • Output schema filtering via JSONPath                         │
│ • Registry client (file, HTTP with auth)                       │
│ • Hot-reloadable configuration                                 │
│ • Policy enforcement (CEL)                                     │
│ • Data plane proxy with observability                          │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ Primitive MCP Servers & Backends                                │
│ (Tools provided by agents/users)                                │
└─────────────────────────────────────────────────────────────────┘
```

---

## The Virtual Tools & Registry Implementation

### What Agentgateway Implements

The registry module (`crates/agentgateway/src/mcp/registry/`) provides:

#### 1. **Virtual Tool Definition** (`VirtualToolDef`)
```rust
pub struct VirtualToolDef {
    pub name: String,                           // Exposed name (renamed)
    pub source: ToolSource,                     // Backend reference
    pub description: Option<String>,            // Override description
    pub input_schema: Option<serde_json::Value>, // Override input schema
    pub defaults: HashMap<String, Value>,       // Default field injection
    pub hide_fields: Vec<String>,               // Fields to hide from agent
    pub output_schema: Option<OutputSchema>,    // Output transformation
    pub version: Option<String>,                // Semantic version
    pub metadata: HashMap<String, Value>,       // Arbitrary metadata
}
```

**This directly implements the `Tool Adapter` pattern from vMCP:**
- ✅ 1:1 tool transformation
- ✅ Rename for clarity
- ✅ Input schema override
- ✅ Output schema transformation
- ✅ Default field injection
- ✅ Context optimization (field hiding)

#### 2. **Output Transformation** (`OutputSchema` + `OutputField`)
```rust
pub struct OutputSchema {
    pub properties: HashMap<String, OutputField>,
}

pub struct OutputField {
    pub field_type: String,                 // JSON Schema type
    pub source_field: Option<String>,       // JSONPath to extract from response
    pub description: Option<String>,        // Field description
}
```

**Maps to vMCP's output transformation:**
- JSONPath-based field extraction (equivalent to vMCP's concept)
- Type annotation for schema validation
- Selective projection to reduce context bloat

**Example from tests:**
```rust
// Transform weather backend output to minimal schema
let properties = HashMap::from([
    ("temperature", OutputField::new("number", "$.data.temp")),
    ("conditions", OutputField::new("string", "$.data.weather")),
]);

// Raw output has 50+ fields; transformed output has 2 fields
// Agent sees only what's needed—tokens saved!
```

#### 3. **Compiled Registry** (`CompiledRegistry`)
```rust
pub struct CompiledRegistry {
    tools_by_name: HashMap<String, Arc<CompiledVirtualTool>>,
    tools_by_source: HashMap<(String, String), Vec<String>>,
}
```

- Pre-compiled JSONPath expressions for performance
- Bidirectional lookup (virtual name ↔ source tool)
- Supports multiple virtual aliases for same source tool

#### 4. **Registry Sources** (`RegistryClient`)
```
Supported sources:
- file:///path/to/registry.json       (local file with hot-reload)
- http://host/path                    (HTTP with optional auth)
- https://host/path                   (HTTPS with cert validation)
```

- Bearer token authentication
- Basic authentication
- Environment variable substitution
- Configurable refresh intervals (default 5 minutes)

### Configuration Integration

The registry is wired into the configuration pipeline (`LocalConfig` → `NormalizedLocalConfig`):

```yaml
# Example config
registry:
  source: "file:///etc/agentgateway/tools.json"
  refreshInterval: "5m"
  auth:
    bearer: "${REGISTRY_TOKEN}"
```

Processed at startup → creates `RegistryStore` → injected into handlers → applied to all tool operations.

---

## Alignment with vMCP Patterns

### ✅ Fully Aligned: Tool Adapter Pattern

| vMCP Concept | Agentgateway Implementation | Status |
|--------------|----------------------------|--------|
| **Rename tool** | `VirtualToolDef.name` overrides source tool name | ✅ Complete |
| **Override description** | `VirtualToolDef.description` optional field | ✅ Complete |
| **Transform input schema** | `VirtualToolDef.input_schema` optional override | ✅ Complete |
| **Inject defaults** | `VirtualToolDef.defaults` HashMap | ✅ Complete |
| **Hide fields** | `VirtualToolDef.hide_fields` Vec<String> | ✅ Complete |
| **Transform output** | `VirtualToolDef.output_schema` with JSONPath | ✅ Complete |
| **Metadata** | `VirtualToolDef.metadata` HashMap | ✅ Complete |

**Example: Salesforce wrapper (from vMCP docs)**

```json
{
  "name": "get_customer_emails",
  "source": { "target": "salesforce", "tool": "query_contacts" },
  "description": "Get email addresses for customers",
  "hideFields": ["AccountId", "Phone", ...47 more fields],
  "outputSchema": {
    "type": "object",
    "properties": {
      "name": { "type": "string", "sourceField": "$.FirstName $.LastName" },
      "email": { "type": "string", "sourceField": "$.Email" }
    }
  }
}
```

Agentgateway **can implement this exactly** with its registry.

---

### ⚠️ Partially Aligned: Higher-Order Patterns

Agentgateway implements tool virtualization but does **not yet implement** the composition patterns (pipeline, router, scatter-gather, etc.). This is intentional:

| vMCP Pattern | Agentgateway | Why |
|--------------|-------------|-----|
| **Pipeline** | ❌ Not implemented | Requires orchestration engine—beyond data plane scope |
| **Router** | ⚠️ Via CEL policies | Can route based on conditions, but not as a composition primitive |
| **Scatter-Gather** | ❌ Not implemented | Requires parallel invocation coordination—control plane function |
| **Enricher** | ❌ Not implemented | Requires multi-tool orchestration—not a proxy concern |
| **Aggregator** | ❌ Not implemented | Requires result collection/merging—composition layer |
| **Circuit Breaker** | ⚠️ Via retry policies | Basic resilience; not full CB with fallback chains |
| **Filter** | ⚠️ Via output schema | Output filtering, but not conditional request filtering |
| **Normalizer** | ⚠️ Via output transform | Single tool output; not multi-source normalization |

**Why this makes sense:**
- Agentgateway is a **data plane proxy**—it handles one request ↔ response at a time
- Composition patterns require **control plane** logic—orchestration across multiple requests
- Separation of concerns: agentgateway provides the tools; vMCP composes them

---

## How to Extend Agentgateway Toward Full vMCP

### Phase 1: Registry Enhancement (Current)

✅ Already done in feature branch:
- Virtual tool definitions
- Output transformation via JSONPath
- Default injection
- Field hiding
- Hot-reloadable registry

**Registry JSON Schema:**
```json
{
  "schemaVersion": "1.0",
  "tools": [
    {
      "name": "virtual_tool_name",
      "source": { "target": "backend_name", "tool": "source_tool_name" },
      "description": "Optional override",
      "inputSchema": { /* Optional JSON Schema override */ },
      "defaults": { "field": "value" },
      "hideFields": ["field1", "field2"],
      "outputSchema": {
        "type": "object",
        "properties": {
          "field": {
            "type": "string",
            "sourceField": "$.path.to.field"
          }
        }
      },
      "version": "1.0",
      "metadata": { /* Arbitrary */ }
    }
  ]
}
```

### Phase 2: Composition Framework (Future)

Would require adding:

1. **Composition Definitions** (extending registry)
   ```json
   {
     "compositions": [
       {
         "name": "research_pipeline",
         "pattern": "pipeline",
         "steps": [
           { "tool": "web_search", "args": {...} },
           { "tool": "fetch_urls", "input": "${previous.urls}" },
           { "tool": "summarize", "input": "${previous.content}" }
         ]
       }
     ]
   }
   ```

2. **Composition Execution Engine**
   - Build DAGs from composition definitions
   - Orchestrate tool invocations
   - Handle data flow (output → input)
   - Error handling & retry policies

3. **Pattern Library**
   - Implement each vMCP pattern
   - Pre-compiled, optimized versions
   - Reusable across compositions

4. **Control Plane Integration**
   - Push compositions to proxies via XDS
   - Hot-reload composition definitions
   - Monitor composition health

### Example: Adding Pipeline Support

```rust
// In registry module: add to types.rs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompositionDef {
    pub name: String,
    pub pattern: CompositionPattern,
    pub spec: serde_json::Value,  // Pattern-specific config
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum CompositionPattern {
    #[serde(rename = "pipeline")]
    Pipeline { steps: Vec<PipelineStep> },

    #[serde(rename = "scatter-gather")]
    ScatterGather { targets: Vec<String>, aggregate: AggregationStrategy },

    // ... other patterns
}

// In composition executor (new module)
pub async fn execute_pipeline(
    steps: Vec<PipelineStep>,
    initial_input: serde_json::Value,
    registry: &CompiledRegistry,
    backends: &BackendPool,
) -> Result<serde_json::Value> {
    let mut current = initial_input;

    for step in steps {
        let tool = registry.get_tool(&step.tool)?;
        let args = substitute_variables(&step.args, &current)?;

        current = backends.invoke_tool(&tool, args).await?;
    }

    Ok(current)
}
```

---

## Integration Points

### 1. Registry Discovery Flow

**Current (Data Plane Only):**
```
Agent/LLM → Agentgateway → Fetch available tools
Agentgateway queries backends → Collects tool list
Applies virtual tool transformations → Returns to agent
Agent selects tool → Invokes through Agentgateway
```

**With vMCP Compositions:**
```
Agent/LLM → Agentgateway → Fetch available tools + compositions
Agentgateway queries backends → Collects tool list
Queries composition registry → Loads compositions
Applies virtual tool transformations → Returns to agent
Agent selects tool OR composition → Invokes
If composition: Agentgateway orchestrates workflow
```

### 2. Configuration Hierarchy

**Current:**
```
Static Config (ports, logging)
    ↓
Local Config (YAML/JSON file, hot-reload)
    ├─ Binds, listeners, routes, backends
    └─ Registry source (file:// or http://)
    ↓
XDS Config (remote control plane)
    └─ Syncs with local, can override
```

**Enhanced with Compositions:**
```
Static Config (ports, logging)
    ↓
Local Config (YAML/JSON file, hot-reload)
    ├─ Binds, listeners, routes, backends
    ├─ Registry source (virtual tools)
    └─ Composition source (NEW: pipeline definitions)
    ↓
XDS Config (remote control plane)
    └─ Syncs with local, can override
```

### 3. Tool Invocation Path

**Current:**
```
Agent request
  ↓
MCP Handler
  ├─ Lookup tool in registry
  ├─ Transform input (apply defaults, validate schema)
  ├─ Invoke backend
  ├─ Transform output (apply JSONPath projections)
  └─ Return to agent
```

**With Compositions:**
```
Agent request
  ↓
MCP Handler
  ├─ Is this a tool or composition?
  ├─ If tool:
  │    ├─ Lookup tool in registry
  │    ├─ Transform input
  │    ├─ Invoke backend
  │    └─ Transform output
  └─ If composition:
       ├─ Load composition definition
       ├─ Execute orchestrator for pattern
       ├─ Handle data flow between steps
       ├─ Apply error handling
       └─ Return aggregated result
```

---

## Code-Level Alignment

### Agentgateway's Registry Implementation

**File: `crates/agentgateway/src/mcp/registry/types.rs`**

```rust
/// This is exactly what vMCP's Tool Adapter pattern needs!
pub struct VirtualToolDef {
    pub name: String,                              // vMCP: rename
    pub source: ToolSource,                        // vMCP: source reference
    pub description: Option<String>,               // vMCP: curate description
    pub input_schema: Option<serde_json::Value>,  // vMCP: simplify input
    pub defaults: HashMap<String, Value>,          // vMCP: inject defaults
    pub hide_fields: Vec<String>,                  // vMCP: filter schema
    pub output_schema: Option<OutputSchema>,       // vMCP: transform output
}
```

**File: `crates/agentgateway/src/mcp/registry/compiled.rs`**

```rust
/// Pre-compiled registry for runtime performance
pub struct CompiledRegistry {
    tools_by_name: HashMap<String, Arc<CompiledVirtualTool>>,
    tools_by_source: HashMap<(String, String), Vec<String>>,
}

/// Compiled tool with JSONPath expressions ready
pub struct CompiledVirtualTool {
    pub def: VirtualToolDef,
    pub output_paths: Option<HashMap<String, CompiledOutputField>>,
    pub effective_schema: Option<serde_json::Value>,
}
```

**File: `crates/agentgateway/tests/tests/registry.rs`**

Comprehensive tests cover:
- Registry loading from file
- Compilation and lookup
- Output transformation with JSONPath
- Default injection
- Field hiding

---

## Strengths of the Agentgateway Approach

1. **Production-Grade Implementation**
   - Written in Rust (performance, safety)
   - Hot-reloadable configuration
   - Comprehensive error handling
   - Well-tested (unit + integration tests)

2. **Proxy Pattern is Natural**
   - Request/response interception point
   - Can inspect and transform tool operations
   - Perfect for adding observability
   - Security policies (CEL-based)

3. **Registry Flexibility**
   - File-based for development
   - HTTP-based for enterprise (centralized management)
   - Auth support (bearer, basic)
   - Environment variable substitution

4. **Type-Safe**
   - JSON Schema for input/output validation
   - Compile-time safety in Rust
   - Pre-compiled JSONPath expressions

5. **Extensible Architecture**
   - New patterns can be added as submodules
   - XDS control plane ready for dynamic updates
   - Clear separation of concerns

---

## Gaps (Intentional & Future)

### Not In Scope for Data Plane

1. **Composition Orchestration**
   - Requires control plane coordination
   - Would belong in a separate composition service
   - Agentgateway can invoke composed tools via registry

2. **Multi-Tool Workflows**
   - Scatter-gather, aggregation, enrichment
   - Requires state management across requests
   - Better handled as a composition engine (vMCP runtime)

3. **Type System**
   - vMCP proposes formal types (A → B, [Tool] → Tool, etc.)
   - Agentgateway uses JSON Schema + JSONPath (practical)
   - Could integrate formal types at composition level

### Recommended for Future

1. **Composition Definitions in Registry**
   - Extend `Registry` struct with `compositions` field
   - Define compositions alongside tools
   - Reuse existing registry loading/refresh mechanism

2. **Composition Execution Module**
   - Implement each vMCP pattern as executor
   - Hook into MCP handler at invocation time
   - Leverage existing error handling & observability

3. **Pattern Library**
   - Pre-built implementations of 16+ patterns
   - Reusable across compositions
   - Tested and optimized

4. **Formal Type Inference (Optional)**
   - Validate composition type safety
   - Catch schema mismatches at definition time
   - Optional; JSON Schema validation sufficient for MVP

---

## Recommended Path Forward

### Step 1: Expand Registry (Next Sprint)
✅ Already started on `feature/virtual_tools_and_registry`

- [x] Virtual tool definitions
- [x] Output transformation
- [x] Default injection
- [x] Field hiding
- [x] Registry loading (file + HTTP)

**Recommendation**: Merge this branch as-is. It's solid, well-tested, and immediately valuable.

### Step 2: Add Composition Definitions (Q1)

Extend the registry schema to include compositions:

```json
{
  "schemaVersion": "1.0",
  "tools": [...],  // Existing
  "compositions": [  // NEW
    {
      "name": "research_pipeline",
      "pattern": "pipeline",
      "steps": [...]
    }
  ]
}
```

**Work**: ~2 weeks
- Extend `Registry` type
- Update loader/compiler
- Add composition tests

### Step 3: Implement Composition Executor (Q2)

Orchestrate multi-tool workflows:

```rust
// Pseudo-code
pub async fn handle_composition(
    composition: &CompositionDef,
    initial_input: Value,
    registry: &CompiledRegistry,
    backends: &BackendPool,
) -> Result<Value> {
    match composition.pattern {
        Pattern::Pipeline => execute_pipeline(...),
        Pattern::Router => execute_router(...),
        Pattern::ScatterGather => execute_scatter_gather(...),
        // ...
    }
}
```

**Work**: ~4 weeks
- Implement each pattern executor
- Handle data flow, error handling, timeouts
- Comprehensive testing

### Step 4: Control Plane Integration (Q3)

Push compositions to proxies dynamically:

- Extend XDS with composition resources
- Add composition to runtime state
- Support hot-reload of composition definitions

**Work**: ~3 weeks
- Design XDS messages for compositions
- Integrate with existing config system
- End-to-end testing

---

## Conclusion: Complementary Strengths

**vMCP (this repo)**: Proposes the **algebra of composition**
- What patterns exist?
- How do they compose?
- What are the type signatures?
- Why does this matter for LLM context?

**Agentgateway**: Implements the **foundation layer**
- Virtual tool adaptation (Tool Adapter pattern)
- Hot-reloadable configuration
- Data plane proxy with governance
- Production-ready in Rust

**Together**: Full toolkit for agentic tool integration
- Theory (vMCP) meets practice (Agentgateway)
- Foundation (virtualization) supports composition (orchestration)
- Separation of concerns: proxy handles individual tools, orchestrator composes them

### Next Steps for Alignment

1. **Keep the branches separate**: vMCP stays conceptual, Agentgateway stays operational
2. **Reference each other**: vMCP's Tool Adapter section should cite Agentgateway as implementation; Agentgateway's docs should reference vMCP composition patterns as future work
3. **Share the vision**: Both are solving "how do AI agents use tools effectively?"—from different angles (theory vs. practice)
4. **Plan integration**: Once Agentgateway has composition executor, it can implement any vMCP pattern

This is **aligned**, not duplicative. vMCP provides the conceptual framework; Agentgateway provides the operational reality.

