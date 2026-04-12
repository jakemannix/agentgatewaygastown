---
type: reference
original_path: docs/design/composition-test-plan.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Tool Composition System Test Plan (imported)

Source: `docs/design/composition-test-plan.md`

Comprehensive test plan covering virtual tools (1:1 mapping) and compositions (N:1 mapping).

## Test Layers

### 1. Rust Unit Tests
- `cargo test --package agentgateway registry` — all registry tests
- `cargo test --package agentgateway patterns` — pattern parsing/serialization
- `cargo test --package agentgateway executor` — pattern execution

### 2. Integration Tests
- `cargo test --package agentgateway --test integration registry`
- Covers: composition parsing, mixed registries, forward reference resolution, duplicate name errors, output transforms, all pattern types

### 3. Benchmarks
Compilation benchmarks for 10/100/1000 tools and compositions. Lookup and transform benchmarks. Requires `internal_benches` feature flag.

### 4. TypeScript DSL Tests
- `cd packages/vmcp-dsl && npm test` — 32 pass, 2 skipped
- `npx vmcp-compile examples/test-tools.ts` — CLI compiler test

### 5. Executor Tests
Mock `ToolInvoker` pattern for testing composition execution without real backends.

## Test Checklist Status
- Pattern types parsing: PASS
- Registry compilation: PASS
- Executor patterns: PASS
- Integration tests: PASS
- TypeScript DSL build/tests: PASS (32 pass, 2 skipped)
- CLI compiler: PASS
- Benchmarks: Not yet implemented

## Known Limitations
1. Full composition execution needs `ToolInvoker` bridge to upstream pool
2. CEL routing in Router pattern not yet wired up
3. Tracing/telemetry for composition steps needed
4. Streaming results from compositions not supported
5. TypeScript DSL missing tool name detection / unresolved reference warnings

## Related Notes
- [[references/imported/TESTING-OUTPUT-TRANSFORM|Testing Output Transform]] — focused output transform testing
- [[references/imported/virtual-tools-vision|Virtual Tools Vision]] — current status of all features
- [[references/imported/code-walkthrough|Code Walkthrough]] — where the tested code lives
