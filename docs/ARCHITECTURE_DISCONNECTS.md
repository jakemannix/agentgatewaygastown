# Architecture Disconnects: Current State vs. Target Pathway

## Target Vision
```
TypeScript (constrained subset)
    ↓ compile
Schematized JSON IR
    ↓ deploy
Go/Rust runtime (as part of gateway)
```

## Current State Analysis

### vMCP Algebra (`mcp-algebra-ala-camel.md`)

**What it provides:**
- Pseudo-code DSL with TypeScript-like syntax (lines 1310-1371)
- Type signatures in functional notation
- Conceptual patterns (16+ integration patterns)
- Mermaid diagrams for visualization

**What it lacks:**
- ❌ No actual TypeScript implementation
- ❌ No compiler/transpiler
- ❌ No JSON IR specification
- ❌ No runtime implementation
- ❌ Purely conceptual/documentation

**Language:** Markdown + pseudo-code + Mermaid

### Agentgateway (`feature/virtual_tools_and_registry`)

**What it provides:**
- ✅ Production Rust runtime (part of gateway)
- ✅ JSON registry format for tool definitions
- ✅ Hot-reloadable configuration (file/HTTP)
- ✅ Tool transformation engine (rename, filter, transform output)

**What it lacks:**
- ❌ No TypeScript DSL for composition definitions
- ❌ No compiler from composition language → JSON IR
- ❌ Limited to single pattern: Tool Adapter (1:1 transforms)
- ❌ No composition orchestration (pipeline, router, scatter-gather, etc.)
- ❌ No formal IR specification

**Language:** Rust (implementation), YAML/JSON (configuration)

---

## The Three-Layer Disconnect

```
DESIRED:
┌─────────────────────────────────┐
│ TypeScript DSL                  │  User writes constrained TS
│ (what agents/users write)       │
└────────────┬────────────────────┘
             │ compile
┌────────────▼────────────────────┐
│ Schematized JSON IR             │  Portable, versionable
│ (canonical intermediate form)   │  deployment artifact
└────────────┬────────────────────┘
             │ interpret/execute
┌────────────▼────────────────────┐
│ Go/Rust Runtime (in gateway)    │  Fast, safe execution
│ (part of agentgateway)          │
└─────────────────────────────────┘


CURRENT STATE (fragmented):
┌─────────────────────────────────┐
│ vMCP Algebra (pseudo-code)      │  ← Conceptual, no implementation
│ (mostly documentation)          │
└─────────────────────────────────┘
                                   (no path from here to implementation)

┌─────────────────────────────────┐
│ Agentgateway Registry (JSON)    │  ← Practical, but incomplete
│ (tool virtualization only)      │
└─────────────────────────────────┘
                                   (no path from here to full composition)
```

---

## Key Disconnects

### 1. **No TypeScript → JSON Compiler Path**

**Current State:**
- vMCP shows pseudo-code examples
- Agentgateway accepts hand-written JSON registry files
- No defined workflow: "write TypeScript" → "compile to JSON" → "execute in gateway"

**What's Missing:**
```typescript
// What users would write (constrained TypeScript subset)
export const researchPipeline = (topic: string): Report =>
  pipeline(
    scatter_gather(
      targets: [web_search, arxiv_search],
      query: topic,
      timeout: 10s
    ),
    normalize(sources, to: ResearchDocument),
    filter(doc => doc.relevance_score > 0.7),
    enrich(doc => ({
      ...doc,
      citations: citation_tool.get(doc.id),
      summary: summarize_tool.run(doc.content)
    })),
    aggregate(synthesize_report)
  );
```

Should compile to:

```json
{
  "name": "research_pipeline",
  "pattern": "pipeline",
  "steps": [
    {
      "pattern": "scatter_gather",
      "targets": ["web_search", "arxiv_search"],
      "config": { "timeout": "10s" }
    },
    {
      "pattern": "normalize",
      "targetSchema": "ResearchDocument"
    },
    // ... rest of steps
  ]
}
```

**Disconnect Gap:** No compiler exists. No syntax defined. No IR spec.

---

### 2. **No Schematized JSON IR Specification**

**Current State:**
- Agentgateway has ad-hoc JSON format (registry.json)
- vMCP has no formal IR definition
- No standard way to represent compositions

**What's Missing:**

A formal spec like:

```protobuf
// Schematized JSON IR Specification

message Composition {
  string name = 1;
  string description = 2;
  CompositionPattern pattern = 3;
  repeated Step steps = 4;
  map<string, Type> inputs = 5;
  Type output = 6;
}

message Step {
  string id = 1;
  oneof operation {
    ToolInvocation tool = 2;
    PipelinePattern pipeline = 3;
    RouterPattern router = 4;
    ScatterGatherPattern scatter_gather = 5;
    // ... other patterns
  }
  repeated InputBinding inputs = 6;
}

message Type {
  string name = 1;
  oneof kind {
    PrimitiveType primitive = 2;
    ObjectType object = 3;
    ArrayType array = 4;
    UnionType union = 5;
  }
}

// etc.
```

**Current Reality:**
- Agentgateway registry is flat JSON (VirtualToolDef)
- vMCP pseudo-code mixes logic with syntax
- No versioning strategy for IR
- No schema validation rules

**Disconnect Gap:** No canonical IR definition. Composition definitions would be ad-hoc JSON without schema.

---

### 3. **Limited Pattern Support in Agentgateway**

**Current State:**
Agentgateway only implements **Tool Adapter** (1:1 transforms):
- ✅ Rename tool
- ✅ Override schema
- ✅ Hide fields
- ✅ Transform output via JSONPath
- ✅ Inject defaults

**Missing Patterns:**
```
❌ Pipeline (sequential execution)
❌ Router (conditional dispatch)
❌ Scatter-Gather (parallel + aggregate)
❌ Splitter (process collections)
❌ Aggregator (combine results)
❌ Enricher (augment data)
❌ Normalizer (schema unification)
❌ Filter (conditional pass/block)
❌ Circuit Breaker (resilience)
❌ Retry (error handling)
❌ Throttle (rate limiting)
❌ Idempotent (deduplication)
❌ Schema Mediator (transform between incompatible schemas)
❌ Capability Router (match by tool capabilities)
❌ Confidence Aggregator (weighted results)
```

**Disconnect Gap:** Agentgateway has the foundation but not the orchestration layer needed for full composition.

---

### 4. **Type System Mismatch**

**vMCP Approach:**
- Functional type signatures: `(A -> B) × [Tool] -> Tool`
- Mathematical notation
- Emphasis on composition algebra

**Agentgateway Approach:**
- JSON Schema for validation
- JSONPath for field extraction
- No formal type inference or composition rules

**Example of the Gap:**

vMCP says:
```
pipeline: [Tool] -> Tool
where each Tool: I → O, and O_n == I_{n+1}
```

Agentgateway would need:
```json
{
  "pattern": "pipeline",
  "steps": [
    { "toolName": "web_search", "inputType": "string", "outputType": "SearchResult[]" },
    { "toolName": "fetch_urls", "inputType": "URL[]", "outputType": "Document[]" },
    { "toolName": "summarize", "inputType": "Document[]", "outputType": "Summary" }
  ]
}
```

But there's no **type checker** that verifies `SearchResult[]` output from `web_search` can be transformed to `URL[]` input for `fetch_urls`.

**Disconnect Gap:** No formal type system. Composition type safety is manual/implicit.

---

### 5. **Control Plane vs. Data Plane Confusion**

**Current Architecture:**
```
vMCP (conceptual, no implementation)
    ↓
Agentgateway Data Plane (executes tools, transforms via registry)
    ↓
MCP Servers (primitive tools)
```

**The Gap:**
- vMCP is positioned as a "composition engine" but is purely conceptual
- Agentgateway is a "data plane proxy" but could embed composition execution
- No agreed boundary between what's data plane vs. control plane responsibility

**Questions Without Answers:**
- Should composition definitions live in registry (agentgateway) or separate?
- Should composition execution happen in gateway or external service?
- How do compositions relate to XDS configuration in agentgateway?
- Who owns the composition type system and validation?

**Disconnect Gap:** Unclear where orchestration belongs in the system.

---

## Recommended Fixes (Aligned to Your Target Pathway)

### Phase 1: Define Schematized JSON IR Spec (Week 1-2)

Create `IR_SPEC.md` that defines:

```protobuf
// In schema/ directory

message CompositionRegistry {
  string schemaVersion = 1;
  repeated Tool tools = 2;
  repeated Composition compositions = 3;
}

message Composition {
  string name = 1;
  string description = 2;
  repeated Parameter inputs = 3;
  Parameter output = 4;
  ExecutionSpec spec = 5;
}

message ExecutionSpec {
  oneof pattern {
    PipelineSpec pipeline = 1;
    RouterSpec router = 2;
    ScatterGatherSpec scatter_gather = 3;
    // ... 13 more patterns
  }
}

message PipelineSpec {
  repeated Step steps = 1;
}

message Step {
  string id = 1;
  oneof operation {
    ToolCall tool = 2;
    NestedComposition composition = 3;
  }
  DataFlowBinding input = 4;
}

// ... full specification
```

**Deliverable:** Proto file + JSON Schema equivalent (for validation without proto tooling)

### Phase 2: TypeScript DSL → JSON Compiler (Week 3-4)

Create `ts-composition-compiler` (small npm package):

```typescript
// Input: TypeScript AST
import { pipeline, scatter_gather, filter, enrich } from "@vmcp/dsl";

export const researchPipeline = (topic: string) =>
  pipeline([
    scatter_gather({
      targets: ["web_search", "arxiv_search"],
      query: topic,
    }),
    filter(doc => doc.relevance_score > 0.7),
    enrich(doc => ({ ...doc, citations: getCitations(doc.id) })),
  ]);

// Output: Schematized JSON IR (per spec from Phase 1)
// {
//   "name": "researchPipeline",
//   "inputs": [{ "name": "topic", "type": "string" }],
//   "output": { "type": "array", "itemType": "Document" },
//   "spec": { "pipeline": { "steps": [...] } }
// }
```

**Deliverable:** npm package that takes TypeScript → validates → outputs JSON IR

### Phase 3: Extend Agentgateway to Execute IR (Week 5-8)

Add composition executor to agentgateway:

```rust
// In crates/agentgateway/src/mcp/compositions/

pub struct CompositionExecutor {
    registry: Arc<CompiledRegistry>,
    patterns: Arc<PatternLibrary>,
}

impl CompositionExecutor {
    pub async fn execute(
        &self,
        composition: &CompositionIR,
        inputs: JsonValue,
    ) -> Result<JsonValue> {
        match &composition.spec {
            ExecutionSpec::Pipeline(pipeline) => self.execute_pipeline(pipeline, inputs).await,
            ExecutionSpec::Router(router) => self.execute_router(router, inputs).await,
            ExecutionSpec::ScatterGather(sg) => self.execute_scatter_gather(sg, inputs).await,
            // ... 13 more pattern implementations
        }
    }
}

pub struct PatternLibrary {
    pipeline: Box<dyn PatternExecutor>,
    router: Box<dyn PatternExecutor>,
    scatter_gather: Box<dyn PatternExecutor>,
    // ... etc.
}
```

**Deliverable:** Composition module in agentgateway that interprets IR

### Phase 4: Integration (Week 9-10)

Wire it all together:

```yaml
# agentgateway config.yaml
registry:
  source: "file:///etc/agentgateway/registry.json"
  refreshInterval: "5m"

# registry.json includes:
# {
#   "tools": [...],
#   "compositions": [
#     {
#       "name": "researchPipeline",
#       "spec": { "pipeline": { "steps": [...] } }
#     }
#   ]
# }
```

Agent discovers both tools and compositions → invokes through same interface → agentgateway routes to appropriate executor.

**Deliverable:** End-to-end: TypeScript → JSON IR → Agentgateway execution

---

## Summary of Disconnects

| Disconnect | Current State | Target State | Gap |
|------------|---------------|--------------|-----|
| **Language** | Markdown + pseudo-code | TypeScript DSL → JSON IR | No compiler |
| **IR Spec** | Ad-hoc JSON registry | Formalized proto/schema | No schema definition |
| **Patterns** | Tool Adapter only | 16+ patterns | 15 patterns missing |
| **Type System** | None | Formal composition types | No type checker |
| **Architecture** | Fragmented (vMCP + agentgateway) | Unified pathway | No clear ownership |
| **Execution** | Agentgateway proxy (no orchestration) | Composition executor in gateway | No orchestrator |

---

## Immediate Next Steps

1. **Define IR Spec** (highest priority)
   - Create proto definition for compositions
   - Publish as schema
   - Get alignment on pattern representation

2. **Create TypeScript DSL**
   - Minimal subset: `pipeline`, `router`, `scatter_gather`
   - Type-safe builder API
   - Compiles to IR spec

3. **Implement in Agentgateway**
   - Add composition executor
   - Load compositions from registry
   - Handle data flow between steps

This addresses your target pathway: **TypeScript → JSON IR → Rust runtime**

