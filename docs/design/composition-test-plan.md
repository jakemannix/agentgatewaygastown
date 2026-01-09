# Tool Composition System - Test Plan

This document provides a comprehensive test plan for the tool composition system implementation.

## Overview

The composition system adds support for:
- **Virtual tools** (1:1 mapping): Rename, hide fields, inject defaults, transform outputs
- **Compositions** (N:1 mapping): Orchestrate multiple tools with patterns like Pipeline, Scatter-Gather, Filter, SchemaMap, and MapEach

## 1. Rust Unit Tests

### Run All Registry Tests

```bash
# From the repository root

# Run all registry tests (including new composition tests)
cargo test --package agentgateway registry

# Run specific pattern tests
cargo test --package agentgateway patterns

# Run executor tests  
cargo test --package agentgateway executor
```

### Expected Test Coverage

| Module | Tests |
|--------|-------|
| `registry/types.rs` | Parsing unified ToolDefinition, legacy conversion |
| `registry/patterns/*.rs` | Pattern parsing, serialization, builder methods |
| `registry/compiled.rs` | Two-pass compilation, reference resolution, transforms |
| `registry/executor/*.rs` | Pattern execution, aggregation, filtering |

## 2. Integration Tests

```bash
# Run the registry integration tests
cargo test --package agentgateway --test integration registry
```

### Integration Test Cases

- `test_composition_parsing` - Parse composition from JSON
- `test_mixed_registry` - Source tools + compositions together
- `test_forward_reference_resolution` - Two-pass compilation
- `test_composition_references` - Tool reference collection
- `test_duplicate_tool_name_error` - Error handling
- `test_composition_output_transform` - Output transformation
- `test_all_pattern_types_parsing` - All 5 pattern types
- `test_prepare_call_args_composition_error` - Composition requires executor

## 3. Benchmarks

```bash
# Run composition benchmarks (requires internal_benches feature)
cargo bench --package agentgateway -F internal_benches
```

### Benchmark Cases

| Benchmark | Description |
|-----------|-------------|
| `compile_10_source_tools` | Compile 10 virtual tools |
| `compile_100_source_tools` | Compile 100 virtual tools |
| `compile_1000_source_tools` | Compile 1000 virtual tools |
| `compile_10_compositions` | Compile 10 compositions |
| `compile_100_compositions` | Compile 100 compositions |
| `compile_mixed_50_50` | Compile 50 source + 50 compositions |
| `compile_mixed_500_500` | Compile 500 source + 500 compositions |
| `lookup_tool_100_registry` | Lookup in 100-tool registry |
| `lookup_tool_1000_registry` | Lookup in 1000-tool registry |
| `transform_output_simple` | Simple output transformation |
| `transform_output_deep_path` | Deep JSONPath transformation |
| `inject_defaults_few` | Inject 2 defaults |
| `inject_defaults_many` | Inject 20 defaults |

## 4. JSON Parsing Test

Create a test file `test-registry.json`. Note that `defaults` and `hideFields` are nested inside the `source` object:

```json
{
  "schemaVersion": "1.0",
  "tools": [
    {
      "name": "get_weather",
      "description": "Get weather with defaults",
      "source": {
        "target": "weather-backend",
        "tool": "fetch_weather",
        "defaults": {
          "units": "metric"
        }
      }
    },
    {
      "name": "multi_search",
      "description": "Search multiple sources",
      "spec": {
        "scatterGather": {
          "targets": [
            { "tool": "search_web" },
            { "tool": "search_arxiv" }
          ],
          "aggregation": {
            "ops": [
              { "flatten": true },
              { "sort": { "field": "$.score", "order": "desc" } },
              { "limit": { "count": 10 } }
            ]
          },
          "timeoutMs": 5000
        }
      }
    },
    {
      "name": "research_pipeline",
      "description": "End-to-end research pipeline",
      "spec": {
        "pipeline": {
          "steps": [
            {
              "id": "search",
              "operation": { "tool": { "name": "multi_search" } },
              "input": { "input": { "path": "$.query" } }
            },
            {
              "id": "filter",
              "operation": {
                "pattern": {
                  "filter": {
                    "predicate": {
                      "field": "$.relevance",
                      "op": "gt",
                      "value": { "numberValue": 0.5 }
                    }
                  }
                }
              },
              "input": { "step": { "stepId": "search", "path": "$" } }
            },
            {
              "id": "normalize",
              "operation": {
                "pattern": {
                  "mapEach": {
                    "inner": {
                      "pattern": {
                        "schemaMap": {
                          "mappings": {
                            "title": { "path": "$.name" },
                            "url": { "coalesce": { "paths": ["$.pdf_url", "$.web_url"] } },
                            "source": { "literal": { "stringValue": "research" } }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          ]
        }
      }
    }
  ]
}
```

Test parsing in Rust:

```rust
use agentgateway::mcp::registry::{Registry, CompiledRegistry};

let json = std::fs::read_to_string("test-registry.json")?;
let registry: Registry = serde_json::from_str(&json)?;

println!("Loaded {} tools", registry.len());

let compiled = CompiledRegistry::compile(registry)?;

// Check tool types
for name in compiled.tool_names() {
    if compiled.is_composition(name) {
        println!("{}: composition", name);
    } else {
        println!("{}: source tool", name);
    }
}
```

## 5. TypeScript DSL Tests

### Setup

```bash
cd packages/vmcp-dsl

# Install dependencies
npm install

# Build
npm run build

# Run tests
npm test
```

### Example Usage

See `packages/vmcp-dsl/examples/test-tools.ts` for a complete example:

```bash
# Run the example directly
npx tsx examples/test-tools.ts

# Or compile to JSON using the CLI
node dist/bin/vmcp-compile.js examples/test-tools.ts -o registry.json --validate
```

### DSL API Overview

```typescript
import { 
  tool, 
  pipeline, 
  scatterGather, 
  agg, 
  filter, 
  schemaMap, 
  mapEach,
  createRegistry 
} from '@vmcp/dsl';

// Define a virtual tool (1:1 mapping)
const weatherTool = tool('get_weather')
  .description('Get weather information')
  .source('weather-backend', 'fetch_weather')
  .default('units', 'metric')
  .build();

// Define a scatter-gather composition
const multiSearch = tool('multi_search')
  .description('Search multiple sources')
  .composition(
    scatterGather()
      .targets('search_web', 'search_arxiv', 'search_wikipedia')
      .aggregate(
        agg()
          .flatten()
          .sortDesc('$.score')
          .dedupe('$.id')
          .limit(20)
      )
      .timeout(5000)
      .build()
  )
  .build();

// Define a pipeline composition
const researchPipeline = tool('research_pipeline')
  .description('End-to-end research')
  .composition(
    pipeline()
      .step('search', 'multi_search')
      .addStep({
        id: 'filter',
        operation: { 
          pattern: filter().field('$.relevance').gt(0.5).build() 
        },
      })
      .addStep({
        id: 'normalize',
        operation: {
          pattern: mapEach()
            .pattern(
              schemaMap()
                .field('title', '$.name')
                .coalesce('url', ['$.pdf_url', '$.web_url'])
                .literal('source', 'research')
                .build()
            )
            .build(),
        },
      })
      .build()
  )
  .build();

// Build and validate registry
const registry = createRegistry()
  .add(weatherTool)
  .add(multiSearch)
  .add(researchPipeline)
  .build();

const result = createRegistry()
  .addAll(weatherTool, multiSearch, researchPipeline)
  .validate();

console.log('Valid:', result.valid);
console.log(JSON.stringify(registry, null, 2));
```

### CLI Compiler

```bash
# Compile TypeScript definitions to JSON
node dist/bin/vmcp-compile.js <input.ts> [options]

Options:
  -o, --output <file>   Output file (default: stdout)
  --validate            Validate the registry before output
  --no-pretty           Output minified JSON
  -h, --help            Show this help message

Examples:
  node dist/bin/vmcp-compile.js tools.ts -o registry.json
  node dist/bin/vmcp-compile.js tools.ts --validate
```

## 6. Executor Test

The executor tests are included in the unit test suite. To run them:

```bash
cargo test --package agentgateway executor
```

Example of a programmatic executor test with a mock invoker:

```rust
use std::sync::Arc;
use agentgateway::mcp::registry::{
    CompiledRegistry, CompositionExecutor, 
    PatternSpec, PipelineSpec, PipelineStep, StepOperation, ToolCall,
    Registry, ToolDefinition, ToolInvoker, ExecutionError,
};
use serde_json::{json, Value};

// Mock tool invoker
struct MockInvoker;

#[async_trait::async_trait]
impl ToolInvoker for MockInvoker {
    async fn invoke(&self, tool_name: &str, args: Value) -> Result<Value, ExecutionError> {
        // Echo back with tool name
        Ok(json!({
            "tool": tool_name,
            "input": args,
            "result": "success"
        }))
    }
}

#[tokio::test]
async fn test_executor() {
    let composition = ToolDefinition::composition(
        "test_pipeline",
        PatternSpec::Pipeline(PipelineSpec {
            steps: vec![
                PipelineStep {
                    id: "step1".to_string(),
                    operation: StepOperation::Tool(ToolCall { 
                        name: "echo".to_string() 
                    }),
                    input: None,
                },
            ],
        }),
    );

    let registry = Registry::with_tool_definitions(vec![composition]);
    let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());
    let invoker = Arc::new(MockInvoker);

    let executor = CompositionExecutor::new(compiled.clone(), invoker);

    let result = executor
        .execute("test_pipeline", json!({"query": "test"}))
        .await
        .unwrap();

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());
    assert!(result.get("tool").is_some());
}
```

## 7. Test Checklist

| Component | Test Command | Status |
|-----------|-------------|--------|
| Pattern types parsing | `cargo test --package agentgateway patterns` | ✅ |
| Registry compilation | `cargo test --package agentgateway compiled` | ✅ |
| Executor patterns | `cargo test --package agentgateway executor` | ✅ |
| Integration tests | `cargo test --package agentgateway --test integration registry` | ✅ |
| Benchmarks | `cargo bench --package agentgateway -F internal_benches` | ⬜ (not yet implemented) |
| TypeScript DSL build | `cd packages/vmcp-dsl && npm run build` | ✅ |
| TypeScript DSL tests | `cd packages/vmcp-dsl && npm test` | ✅ (32 pass, 2 skipped) |
| CLI compiler | `node dist/bin/vmcp-compile.js examples/test-tools.ts` | ✅ |

## Known Limitations

1. **Full composition execution** requires implementing a `ToolInvoker` that bridges to the upstream pool - the session integration has a placeholder for this

2. **CEL routing** in compositions is not yet wired up - Router pattern is defined in proto but not implemented

3. **Tracing/telemetry** for composition steps needs to be added for observability

4. **Error handling** in scatter-gather partial failures needs more testing

5. **Streaming results** from compositions is not yet supported

6. **TypeScript DSL validation** - Missing tool name detection and unresolved tool reference warnings are not yet implemented

## Next Steps

1. Implement `ToolInvoker` bridge to `UpstreamGroup` for real backend calls
2. Add Router pattern with CEL condition evaluation
3. Add composition execution tracing/spans
4. Add streaming support for long-running compositions
5. Add circuit breaker pattern for resilience
6. Implement full validation in TypeScript DSL (missing names, unresolved references)
