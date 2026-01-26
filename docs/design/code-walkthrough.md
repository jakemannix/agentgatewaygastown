# Virtual Tools: Code Walkthrough

This document maps the virtual tools system to its implementation in the codebase.

## Directory Structure

```
crates/agentgateway/
├── proto/
│   └── registry.proto          # Proto definitions (source of truth)
├── src/
│   ├── types/
│   │   └── proto.rs            # Generated proto types with pbjson serde
│   └── mcp/
│       └── registry/
│           ├── mod.rs          # Module exports
│           ├── types.rs        # Hand-written runtime types
│           ├── types_compat.rs # Proto → hand-written conversion
│           ├── compiled.rs     # CompiledRegistry, CompiledTool
│           ├── client.rs       # Registry loading (file/HTTP)
│           ├── store.rs        # RegistryStore with hot-reload
│           ├── validation.rs   # Startup validation
│           ├── runtime_hooks.rs # Runtime validation hooks
│           └── executor/
│               ├── mod.rs      # CompositionExecutor
│               ├── pipeline.rs # Pipeline pattern executor
│               ├── scatter_gather.rs
│               ├── filter.rs
│               ├── map_each.rs
│               └── schema_map.rs

packages/vmcp-dsl/
├── src/
│   ├── index.ts              # Public API exports
│   ├── types.ts              # Hand-written types (deprecated)
│   ├── builder.ts            # Fluent builder API
│   ├── compiler.ts           # Registry validation & serialization
│   ├── path-builder.ts       # JSONPath helpers
│   ├── patterns/             # Pattern builders (pipeline, scatter-gather, etc.)
│   └── generated/
│       └── registry.ts       # Proto-generated types (161KB)
├── tool-builder/
│   └── index.html            # Visual composition builder
└── bin/
    └── vmcp-compile.ts       # CLI compiler
```

## Request Flow

### 1. Gateway Startup

```
config.yaml
    │
    ▼
RegistryStore::new()           # store.rs:45
    │
    ├── RegistryClient::fetch()  # client.rs:89
    │       │
    │       ▼
    │   parse_registry_from_proto()  # types_compat.rs:15
    │       │
    │       ├── Try proto types first (canonical v2 format)
    │       └── Fallback to hand-written (supports v1 "target" alias)
    │
    └── CompiledRegistry::new()  # compiled.rs:156
            │
            ├── resolve_schema_refs()    # Resolve $ref in schemas
            ├── compile_tool()           # For each tool
            │       │
            │       ├── Source tool → CompiledSourceTool
            │       └── Composition → CompiledComposition
            │
            └── validate_all()           # validation.rs
```

### 2. Tool Call (Source Tool)

```
POST /mcp { method: "tools/call", params: { name: "virtual_find_products" } }
    │
    ▼
MCPSession::handle_request()           # session.rs:180
    │
    ├── resolve_tool_call()            # session.rs:290
    │       │
    │       ▼
    │   CompiledRegistry::resolve_tool_call()  # compiled.rs:280
    │       │
    │       ├── Strip "virtual_" prefix
    │       ├── Look up CompiledTool
    │       └── Return ResolvedToolCall::Source { target, tool, args }
    │
    └── relay.send_single_with_output_transform()  # handler.rs:590
            │
            ├── Forward to backend MCP server
            ├── Apply outputTransform if present
            └── Return transformed result
```

### 3. Tool Call (Composition)

```
POST /mcp { method: "tools/call", params: { name: "virtual_personalized_search" } }
    │
    ▼
MCPSession::handle_request()           # session.rs:180
    │
    ├── resolve_tool_call()
    │       │
    │       └── Return ResolvedToolCall::Composition { name, args }
    │
    └── CompositionExecutor::execute_with_tracing()  # executor/mod.rs:85
            │
            ├── Look up CompiledComposition
            ├── Match pattern type:
            │       │
            │       ├── Pipeline → PipelineExecutor::execute()
            │       ├── ScatterGather → ScatterGatherExecutor::execute()
            │       ├── Filter → FilterExecutor::execute()
            │       └── etc.
            │
            └── Build CallToolResult with structuredContent
```

## Key Types

### Proto Types (Source of Truth)

```rust
// crates/agentgateway/src/types/proto.rs (generated)

pub struct Registry {
    pub schema_version: String,
    pub tools: Vec<ToolDefinition>,
    pub schemas: Vec<SchemaDefinition>,
    pub servers: Vec<ServerDefinition>,
    pub agents: Vec<AgentDefinition>,
}

pub struct ToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub implementation: Option<Implementation>,  // oneof: source or spec
    pub input_schema: Option<prost_wkt_types::Value>,
    pub output_schema: Option<prost_wkt_types::Value>,
    pub output_transform: Option<OutputTransform>,
}

pub enum Implementation {
    Source(SourceTool),
    Spec(PatternSpec),
}

pub struct PatternSpec {
    pub pattern: Option<Pattern>,  // oneof: pipeline, scatter_gather, etc.
}
```

### Runtime Types

```rust
// crates/agentgateway/src/mcp/registry/compiled.rs

pub struct CompiledRegistry {
    tools_by_name: HashMap<String, CompiledTool>,
    schemas: HashMap<String, Arc<serde_json::Value>>,
    // ...
}

pub struct CompiledTool {
    pub def: ToolDefinition,
    pub compiled: CompiledImplementation,
}

pub enum CompiledImplementation {
    Source(CompiledSourceTool),
    Composition(CompiledComposition),
}
```

### TypeScript Types

```typescript
// packages/vmcp-dsl/src/generated/registry.ts (generated)

export interface Registry {
  schemaVersion: string;
  tools: ToolDefinition[];
  schemas: SchemaDefinition[];
  servers: ServerDefinition[];
  agents: AgentDefinition[];
}

export const Registry = {
  fromJSON(object: any): Registry { ... },
  toJSON(message: Registry): unknown { ... },
};
```

## Pattern Executors

Each pattern has a dedicated executor:

| Pattern | File | Entry Point |
|---------|------|-------------|
| Pipeline | `executor/pipeline.rs` | `PipelineExecutor::execute()` |
| Scatter-Gather | `executor/scatter_gather.rs` | `ScatterGatherExecutor::execute()` |
| Filter | `executor/filter.rs` | `FilterExecutor::execute()` |
| MapEach | `executor/map_each.rs` | `MapEachExecutor::execute()` |
| SchemaMap | `executor/schema_map.rs` | `SchemaMapExecutor::execute()` |

### Data Binding Resolution

```rust
// executor/mod.rs

impl CompositionExecutor {
    pub fn resolve_binding(
        &self,
        binding: &DataBinding,
        input: &Value,
        step_results: &HashMap<String, Value>,
    ) -> Result<Value, ExecutionError> {
        match binding {
            DataBinding::Input(ib) => {
                // JSONPath from composition input
                jsonpath_select(input, &ib.path)
            }
            DataBinding::Step(sb) => {
                // JSONPath from previous step's output
                let step_output = step_results.get(&sb.step_id)?;
                jsonpath_select(step_output, &sb.path)
            }
            DataBinding::Constant(cb) => {
                // Literal value
                Ok(cb.value.clone())
            }
            DataBinding::Construct(cb) => {
                // Build object from multiple bindings
                let mut obj = Map::new();
                for (key, field_binding) in &cb.fields {
                    obj.insert(key.clone(), self.resolve_binding(field_binding, input, step_results)?);
                }
                Ok(Value::Object(obj))
            }
        }
    }
}
```

## Tests

### Rust Tests

```bash
# All registry tests
cargo test -p agentgateway registry

# Pattern-specific
cargo test -p agentgateway pipeline
cargo test -p agentgateway scatter_gather

# Golden JSON tests (proto compatibility)
cargo test -p agentgateway golden_json

# Integration with ecommerce demo
cargo test -p agentgateway ecommerce
```

### TypeScript Tests

```bash
cd packages/vmcp-dsl
npm test

# Specific test files
npm test -- builder.test.ts
npm test -- patterns.test.ts
npm test -- compiler.test.ts
```

## Adding a New Pattern

1. **Define in proto**: Add to `PatternSpec.pattern` oneof in `registry.proto`
2. **Regenerate**: `make gen` or `cargo build -p agentgateway`
3. **Add conversion**: Implement `From<proto::NewPattern>` in `types_compat.rs`
4. **Add executor**: Create `executor/new_pattern.rs`
5. **Wire up**: Add match arm in `CompositionExecutor::execute_pattern()`
6. **TypeScript**: Add pattern builder in `packages/vmcp-dsl/src/patterns/`
7. **Tests**: Add to `tests/fixtures/registry/` and test files
