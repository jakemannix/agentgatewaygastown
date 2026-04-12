---
type: reference
original_path: docs/design/code-walkthrough.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Virtual Tools: Code Walkthrough (imported)

Source: `docs/design/code-walkthrough.md`

Maps the virtual tools system to its implementation in the codebase.

## Directory Structure

**Rust (crates/agentgateway/):**
- `proto/registry.proto` — proto definitions (source of truth)
- `src/types/proto.rs` — generated proto types with pbjson serde
- `src/mcp/registry/types.rs` — hand-written runtime types
- `src/mcp/registry/types_compat.rs` — proto-to-runtime conversion
- `src/mcp/registry/compiled.rs` — CompiledRegistry, CompiledTool
- `src/mcp/registry/client.rs` — registry loading (file/HTTP)
- `src/mcp/registry/store.rs` — RegistryStore with hot-reload
- `src/mcp/registry/validation.rs` — startup validation
- `src/mcp/registry/runtime_hooks.rs` — runtime validation hooks
- `src/mcp/registry/executor/` — pattern executors (pipeline, scatter_gather, filter, map_each, schema_map)

**TypeScript (packages/vmcp-dsl/):**
- `src/generated/registry.ts` — proto-generated types (161KB)
- `src/types.ts` — hand-written types (deprecated)
- `src/builder.ts` — fluent builder API
- `src/compiler.ts` — registry validation and serialization
- `tool-builder/index.html` — visual composition builder
- `bin/vmcp-compile.ts` — CLI compiler

## Request Flow

### Gateway Startup
`config.yaml` -> `RegistryStore::new()` -> `RegistryClient::fetch()` -> `parse_registry_from_proto()` (try proto first, fallback to hand-written for v1 "target" alias) -> `CompiledRegistry::new()` (resolve schema refs, compile tools, validate)

### Tool Call (Source Tool)
MCP request -> `MCPSession::handle_request()` -> `CompiledRegistry::resolve_tool_call()` (strip "virtual_" prefix, look up CompiledTool) -> `relay.send_single_with_output_transform()` (forward to backend, apply outputTransform)

### Tool Call (Composition)
MCP request -> resolve as `ResolvedToolCall::Composition` -> `CompositionExecutor::execute_with_tracing()` -> match pattern type -> dispatch to PipelineExecutor/ScatterGatherExecutor/etc. -> build `CallToolResult` with structuredContent

## Data Binding Resolution
Four binding types: `Input` (JSONPath from composition input), `Step` (JSONPath from previous step output), `Constant` (literal value), `Construct` (build object from multiple bindings).

## Adding a New Pattern
1. Define in proto (`PatternSpec.pattern` oneof in `registry.proto`)
2. Regenerate (`make gen`)
3. Add conversion (`From<proto::NewPattern>` in `types_compat.rs`)
4. Add executor (`executor/new_pattern.rs`)
5. Wire up (match arm in `CompositionExecutor::execute_pattern()`)
6. TypeScript (pattern builder in `packages/vmcp-dsl/src/patterns/`)
7. Tests (add to `tests/fixtures/registry/`)

## Related Notes
- [[references/imported/virtual-tools-vision|Virtual Tools Vision]] — feature status
- [[references/imported/composition-test-plan|Composition Test Plan]] — how to test
- [[references/imported/proto-codegen-migration|Proto Codegen]] — the type system architecture
