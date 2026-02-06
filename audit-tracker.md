# Codebase Audit Tracker

## Fork Summary
- **Fork point:** `3f23c90` (Add WP3/WP4 failing tests for validation and runtime hooks)
- **Commits since fork:** 92
- **Rust files changed:** 42 (6 new, 36 modified)
- **Net lines:** +7759 / -270
- **Test results:** 625 passed, 53 failed (all failures in upstream proxy/openapi/gateway tests due to DNS/network unavailability in sandbox; 0 failures in fork-specific code)

## Files Reviewed

### Chunk A: Core proxy/gateway modifications
- [x] `crates/agentgateway/src/config.rs`
- [x] `crates/agentgateway/src/lib.rs`
- [x] `crates/agentgateway/src/mcp/handler.rs`
- [x] `crates/agentgateway/src/mcp/mod.rs`
- [x] `crates/agentgateway/src/mcp/router.rs`
- [x] `crates/agentgateway/src/mcp/session.rs`
- [x] `crates/agentgateway/src/mcp/upstream/mod.rs`
- [x] `crates/agentgateway/src/telemetry/log.rs`
- [x] `crates/agentgateway/src/telemetry/trc.rs`
- [x] `crates/agentgateway/src/types/proto.rs`
- [x] `crates/agentgateway/build.rs`
- [x] `crates/agentgateway/proto/registry.proto`

### Chunk B: DAG orchestrator engine
- [x] `crates/agentgateway/src/mcp/registry/executor/mod.rs`
- [x] `crates/agentgateway/src/mcp/registry/executor/pipeline.rs`
- [x] `crates/agentgateway/src/mcp/registry/executor/scatter_gather.rs`
- [x] `crates/agentgateway/src/mcp/registry/executor/context.rs`
- [x] `crates/agentgateway/src/mcp/registry/executor/timeout.rs`
- [x] `crates/agentgateway/src/mcp/registry/executor/throttle.rs`
- [x] `crates/agentgateway/src/mcp/registry/executor/composition_tracing.rs`
- [x] `crates/agentgateway/src/mcp/registry/execution_graph.rs`
- [x] `crates/agentgateway/src/mcp/registry/runtime_hooks.rs`
- [x] `crates/agentgateway/src/mcp/registry/error.rs`

### Chunk C: Backend integrations
- [x] `crates/agentgateway/src/mcp/registry/client.rs`
- [x] `crates/agentgateway/src/mcp/registry/store.rs`
- [x] `crates/agentgateway/src/mcp/identity.rs`

### Chunk D: Payload transformation logic
- [x] `crates/agentgateway/src/mcp/registry/types.rs`
- [x] `crates/agentgateway/src/mcp/registry/types_compat.rs`
- [x] `crates/agentgateway/src/mcp/registry/compiled.rs`
- [x] `crates/agentgateway/src/mcp/registry/patterns/mod.rs`
- [x] `crates/agentgateway/src/mcp/registry/patterns/pipeline.rs`
- [x] `crates/agentgateway/src/mcp/registry/patterns/scatter_gather.rs`
- [x] `crates/agentgateway/src/mcp/registry/patterns/schema_map.rs`
- [x] `crates/agentgateway/src/mcp/registry/mod.rs`
- [x] `crates/agentgateway/src/mcp/registry/validation.rs`

### Chunk E: Test suite
- [x] `crates/agentgateway/tests/tests/registry.rs`
- [x] `crates/agentgateway/tests/tests/mod.rs`
- [x] `crates/agentgateway/src/mcp/mcp_tests.rs`
- [x] `crates/agentgateway/src/mcp/registry/tests/golden_json.rs`
- [x] `crates/agentgateway/src/mcp/registry/tests/mod.rs`
- [x] `crates/agentgateway/src/test_helpers/proxymock.rs`
- [x] `crates/agentgateway/benches/bench_tests.rs`
- [x] `crates/agentgateway/src/http/jwt_tests.rs`

---

## Findings

### P0 - Correctness/Safety Issues

**P0-1 [Chunk B] No cycle detection in composition executor - infinite recursion risk**
- File: `executor/mod.rs:322-343` (`execute_tool`)
- If composition A calls composition B which calls A, `execute_tool` recurses infinitely via `Box::pin`. The validation layer (`validation.rs`) does DFS cycle detection, but only for tools with explicit `depends` fields. Compositions reference tools by name in `ToolCall`, and these references are NOT tracked in the `depends` field during parsing. This means a cycle like `comp_A -> step calls comp_B -> step calls comp_A` would NOT be detected by validation and WILL cause a stack overflow at runtime.
- Same issue in `RelayToolInvoker::invoke` (`handler.rs:485-512`): creates new `CompositionExecutor` on each nested call with `Arc::new(self.clone())`, enabling unbounded recursion.
- **Fix:** Add a recursion depth counter to `ExecutionContext` (or a `HashSet<String>` of in-flight compositions), checked in `execute_tool` before recursing. The MAX_REFERENCE_DEPTH constant (100) in compiled.rs is only used during compilation, not at runtime.

**P0-2 [Chunk A] Stringly-typed tool routing via underscore delimiter is fundamentally fragile**
- File: `handler.rs:36` (`const DELIMITER: &str = "_"`)
- File: `handler.rs:360` (`format!("{}_{}", srv, name)`)
- File: `executor/mod.rs:361` (`format!("{}_{}", srv, name)`)
- Tool names containing underscores (extremely common: `github_search`, `arxiv_search`) cause ambiguous parsing. `parse_resource_name` splits on `_` and tries heuristics but can misroute. The code already acknowledges this in CLAUDE.md's "Design TODO: Proper Tool Algebra" section.
- Impact: A backend tool named `foo_bar` on server `baz` becomes `baz_foo_bar`, which could be parsed as server=`baz_foo`, tool=`bar`. This is mitigated by the `virtual_` prefix check, but any non-virtual tool with underscores in the server name will fail.
- This is the #1 architectural debt in the fork.

**P0-3 [Chunk A] `serde_json::to_string(&result).unwrap_or_default()` silently drops data**
- File: `session.rs:450` (in the composition result handler)
- If `serde_json::to_string` fails (e.g., contains non-finite floats), the text content of the MCP response becomes an empty string, but `structured_content` still contains the Value. The downstream client receives contradictory content.
- **Fix:** Handle the serialization error explicitly or use `to_string_pretty` with error propagation.

**P0-4 [Chunk D] `types_compat.rs` loses data in proto-to-hand-written type conversions**
- File: `types_compat.rs` (993 lines)
- This 993-line file converts between protobuf-generated types and hand-written types. Several conversions use `unwrap_or_default()` on optional fields, silently swallowing `None` values. For example, if a proto `SchemaMapSpec` has no mappings, it becomes an empty map rather than an error.
- No roundtrip tests exist to verify that `proto -> hand-written -> proto` preserves all data. This is a significant gap given the file's size and criticality.

**P0-5 [Chunk B] `aggregate()` uses stale `values` variable after mutation**
- File: `executor/scatter_gather.rs:137-153`
- `fn aggregate(mut values: Vec<Value>, ops: &[AggregationOp])` clones `values` into `result` on line 138 (`Value::Array(values.clone())`), then iterates ops mutating `result`. But the `Merge` op on line 147 takes `&mut values` (the original, now potentially stale). If a previous op modified `result` (e.g., Flatten), the Merge op operates on the pre-transformation data, not the current pipeline state.
- **Fix:** `Merge` should operate on `result`, not `values`.

**P0-6 [Chunk A] `RelayToolInvoker::invoke` clones args unnecessarily and creates allocation pressure**
- File: `handler.rs:407` - `args.clone()` is done on line where `resolve_tool_call` is called, even though args are consumed in the match arm. The original `args` is also captured in the closure for the `effective_tool_name` computation but never used again after `resolve_tool_call`.

### P1 - Design/Architecture Issues

**P1-1 [Chunk A] Virtual tool prefix/stripping layering violation**
- File: `handler.rs:407-470` (`RelayToolInvoker::invoke`)
- The invoker adds `virtual_` prefix to registry tools before calling `resolve_tool_call`, which then strips it. This round-trip is a layering violation: the executor works with registry names, the relay works with MCP names, and the invoker bridges them by string manipulation. A cleaner design would have the invoker call a different resolution path that doesn't require the prefix.

**P1-2 [Chunk B] Pipeline executor is sequential even when steps are independent**
- File: `executor/pipeline.rs:18-104`
- Steps `a`, `b`, `c` that all depend only on input are executed sequentially. The tests even document this with a comment at line 341: "Currently executes sequentially, but produces correct results." The execution_graph.rs module has wave-based analysis that COULD enable parallelism, but it's not used by the executor.
- This is correctly identified as future work but is worth noting as a performance gap for scatter-gather-heavy workloads that are modeled as pipelines.

**P1-3 [Chunk D] Dual type system: hand-written types + proto-generated types + compat layer**
- Files: `types.rs` (654 new lines), `types_compat.rs` (993 new lines), `registry.proto`
- Three representations of the same data: proto types (generated), hand-written Rust types (with serde), and compiled types. The `types_compat.rs` conversion layer is almost 1000 lines of mechanical mapping that must stay in sync with both proto and hand-written types.
- This creates a significant maintenance burden and is a frequent source of bugs. Consider either generating the hand-written types from proto (prost + custom derives) or dropping the proto types entirely if they're not used for wire format.

**P1-4 [Chunk B] `ExecutionError` collapses context into strings**
- File: `executor/mod.rs:39-76`
- All error variants use `String` payloads except `TypeError` and `Timeout`. This means error chains are lost: `ToolExecutionFailed(e.to_string())` at handler.rs:464 discards the original error type, making it impossible to distinguish between network errors, timeout errors, and logic errors in upstream code.
- Consider using `#[from]` or `#[source]` with `anyhow::Error` or a proper error chain.

**P1-5 [Chunk A] `Session` uses `Arc<RwLock<Option<CallerIdentity>>>` - overly complex for single-writer**
- File: `session.rs:37`
- `caller_identity` is only written once (during `InitializeRequest`) and read during `tools/list`. A simpler `OnceCell` or `OnceLock` would be more appropriate and communicate the single-assignment semantics.

**P1-6 [Chunk C] `agent_tool_dependencies` parses SBOM extension via string-keyed JSON**
- File: `compiled.rs:327-362`
- The SBOM extension is parsed by checking `ext.uri == "urn:agentgateway:sbom"` and then navigating `params.get("depends")` -> array -> each item's `get("type")` and `get("name")`. This is pure stringly-typed JSON navigation where a proper typed struct would prevent typos and provide compile-time safety.

**P1-7 [Chunk B] `runtime_hooks.rs` is largely unimplemented stubs**
- File: `runtime_hooks.rs` (+266 lines)
- The runtime hooks infrastructure defines types and trait methods but most hook points are `// TODO: implement`. The `HookExecutor` and hook chain exist in types but are never invoked from the executor. This is dead code that adds complexity without functionality.

**P1-8 [Chunk D] `validation.rs` cycle detection only covers explicit `depends` field**
- File: `validation.rs:131-200`
- The DFS cycle detection only walks `tool.depends` relationships. But compositions reference tools by name in `ToolCall` (e.g., pipeline steps, scatter targets). These implicit dependencies are NOT tracked in `depends`, so cycles through composition graphs are invisible to validation. See P0-1.

### P2 - Test Coverage Gaps

**P2-1 [Chunk E] No test for recursive composition depth limiting (related to P0-1)**
- No test verifies that deeply nested or cyclically referencing compositions are caught. The test `test_invoker_should_handle_nested_compositions` (executor/mod.rs:816) tests 2-level nesting but doesn't test the case where compositions form a cycle.

**P2-2 [Chunk E] No roundtrip tests for types_compat.rs conversions**
- File: `types_compat.rs` (993 lines, 0 test coverage)
- The largest new file has zero tests. Given it converts between two type systems, property-based or roundtrip tests (`proto -> hand-written -> proto == identity`) are essential.

**P2-3 [Chunk E] Scatter-gather partial failure behavior undertested**
- File: `executor/scatter_gather.rs` tests
- Tests verify `fail_fast=false` with all successes and `fail_fast=true`, but don't test the partial failure case where some targets succeed and others fail with `fail_fast=false`. The "AllTargetsFailed" path is also untested.

**P2-4 [Chunk E] No test for timeout propagation through nested compositions**
- The `TimeoutExecutor` wraps a single pattern, but there's no test verifying that a timeout on an outer composition properly cancels in-progress inner compositions.

**P2-5 [Chunk E] Pipeline step binding with invalid step_id has no integration test**
- `resolve_binding` returns `InvalidInput` for missing step_id, but tests only cover the happy path. A step referencing a nonexistent previous step should be tested.

**P2-6 [Chunk E] `execution_graph.rs` wave analysis is tested but never used at runtime**
- The execution graph module computes dependency waves for parallelization, has tests, but is never called from the pipeline executor. This means the wave analysis correctness is tested in isolation but the integration with execution is not.

**P2-7 [Chunk E] No test for `merge_tools_with_identity` agent dependency filtering**
- The `merge_tools_with_identity` function in handler.rs is the critical integration point for caller identity filtering. The `session_identity_from_client_info_filters_tools` test exists but fails due to network (DNS) requirements, meaning this path has NO passing test.

**P2-8 [Chunk E] Golden JSON tests don't cover error cases**
- File: `tests/golden_json.rs` (527 lines)
- Golden tests verify successful parsing of demo registries but don't test malformed JSON, missing required fields, or schema validation failures.

### P3 - Performance Concerns

**P3-1 [Chunk B] Excessive `Value::clone()` in pipeline execution**
- File: `executor/pipeline.rs:24` (`input.clone()`), line 32 (`current_result.clone()`), line 99 (`result.clone()`)
- Every pipeline step clones both the input and the current result. For large payloads (e.g., search results with many items), this creates O(steps * payload_size) allocation overhead. Consider using `Arc<Value>` or passing ownership.

**P3-2 [Chunk B] Scatter-gather clones input for every target**
- File: `executor/scatter_gather.rs:32` (`input.clone()` in `map`)
- Each scatter target gets a clone of the full input. For N targets with large inputs, this is N full copies. Consider `Arc<Value>` for the broadcast case where all targets get the same input.

**P3-3 [Chunk B] JSONPath re-parsed on every invocation**
- File: `executor/pipeline.rs:141` (`JsonPath::parse(path)`)
- File: `executor/scatter_gather.rs:172,218,243` (multiple `JsonPath::parse` calls)
- JSONPaths in aggregation ops are parsed from string on every execution. Since these come from compiled patterns (static configuration), they should be parsed once during compilation and stored as `CompiledFieldSource` already does for output transforms.

**P3-4 [Chunk A] `format!` allocations in hot path for tool name construction**
- File: `handler.rs:407-470` (RelayToolInvoker::invoke)
- `format!("{}_{}", VIRTUAL_SERVER_NAME, tool_name)` is called on every tool invocation to check if a tool needs the virtual prefix. This could use a pre-allocated buffer or SmallString.

**P3-5 [Chunk B] `aggregate()` clones the entire values vec up front**
- File: `executor/scatter_gather.rs:138` (`Value::Array(values.clone())`)
- The initial `values.clone()` creates a full copy of all scatter results just to wrap them in an Array. Consider taking ownership: `Value::Array(values)` (the original vec is only used by Merge, which has the stale-data bug P0-5).

### P4 - Style/Maintenance

**P4-1 [Chunk B] Extensive debug logging in executor could be noisy**
- Multiple files in executor/ have `tracing::debug!` with `target: "virtual_tools"` on nearly every operation. While useful for development, this volume of debug logging in production could impact performance and log storage.

**P4-2 [Chunk A] Dead code: `_composition` string replaced by `VIRTUAL_SERVER_NAME`**
- File: `session.rs`, `handler.rs`
- The old `"_composition"` string has been replaced with `VIRTUAL_SERVER_NAME` but some comments still reference the old name.

**P4-3 [Chunk D] `types.rs` has large Default impls that could use `#[derive(Default)]`**
- Several structs in `types.rs` implement `Default` manually when `#[derive(Default)]` would suffice.

**P4-4 [Chunk B] Test code in production modules**
- `MockToolInvoker`, `RegistryAwareInvoker`, and `OrderTrackingInvoker` are behind `#[cfg(test)]` in production modules, which is fine but adds significant line count to already-large files.

---

## Status Summary After 5-File Batches

### After files 1-12 (Chunk A)
The core proxy modifications are well-structured. The `virtual_` prefix convention is a pragmatic solution to the tool naming problem, but the underscore delimiter (P0-2) remains the biggest risk. The session changes for caller identity are clean. Handler changes for nested composition support are the most complex modification.

### After files 13-23 (Chunk B)
The DAG orchestrator is architecturally sound. Pipeline and scatter-gather executors are clear and well-tested. The critical gap is the lack of cycle/depth protection at runtime (P0-1). The parallel execution opportunity (P1-2) is documented but not implemented. Tracing integration is thorough.

### After files 24-26 (Chunk C)
Backend integrations are minimal and clean. The identity module is well-designed with clear extraction from three sources. The SBOM dependency extraction (P1-6) could use typed structs.

### After files 27-35 (Chunk D)
The type system is the heaviest area of the fork. The triple representation (proto/hand-written/compiled) is the biggest maintenance burden (P1-3). The `types_compat.rs` file is a liability without tests (P2-2). Validation is solid but has the cycle detection gap (P1-8).

### After files 36-43 (Chunk E)
Test coverage for the core executor logic is good (90 passing tests). Major gaps: types_compat roundtrips (P2-2), partial failure paths (P2-3), recursion limits (P2-1), and agent identity filtering (P2-7). The golden JSON tests are a good practice.

---

## Top 10 Highest-Impact Findings

| Rank | ID | Category | Chunk | Impact | Description |
|------|-----|----------|-------|--------|-------------|
| 1 | P0-1 | Correctness | B | **Critical** | No runtime cycle/depth protection for nested compositions - stack overflow possible |
| 2 | P0-2 | Correctness | A | **High** | Underscore delimiter for tool routing is ambiguous when names contain underscores |
| 3 | P0-5 | Correctness | B | **Medium** | `aggregate()` Merge op uses stale pre-transformation data |
| 4 | P1-3 | Architecture | D | **High** | Triple type system (proto/hand-written/compiled) creates ~2000 lines of mapping code |
| 5 | P2-2 | Test Gap | E | **High** | types_compat.rs has 993 lines and zero tests |
| 6 | P1-4 | Architecture | B | **Medium** | ExecutionError loses error context by collapsing to strings |
| 7 | P2-7 | Test Gap | E | **Medium** | Agent identity filtering has no passing integration test |
| 8 | P3-1 | Performance | B | **Medium** | Excessive Value cloning in pipeline (O(steps * payload_size)) |
| 9 | P1-8 | Architecture | D | **Medium** | Validation cycle detection misses implicit composition references |
| 10 | P3-3 | Performance | B | **Low-Med** | JSONPath re-parsed from string on every execution instead of pre-compiled |

## Estimated Test Coverage Gaps by Module

| Module | Fork Tests | Coverage Estimate | Gap |
|--------|-----------|-------------------|-----|
| `executor/mod.rs` | 7 tests | ~75% | Missing: cycles, depth limits, error propagation |
| `executor/pipeline.rs` | 5 tests | ~70% | Missing: step binding errors, empty pipeline, pattern steps |
| `executor/scatter_gather.rs` | 9 tests | ~65% | Missing: partial failures, all-failed, timeout+partial |
| `executor/timeout.rs` | ~3 tests | ~60% | Missing: nested timeout propagation |
| `executor/context.rs` | 0 direct | ~40% | Tested indirectly via pipeline/scatter tests |
| `compiled.rs` | ~15 tests | ~70% | Missing: error paths, malformed input, deep nesting |
| `types.rs` | ~10 tests | ~60% | Missing: serde edge cases, unknown fields |
| `types_compat.rs` | 0 tests | **~0%** | **Critical gap** - 993 lines with no tests |
| `validation.rs` | ~19 tests | ~80% | Missing: composition-graph cycles |
| `identity.rs` | 6 tests | ~90% | Good coverage, minor gap on empty strings |
| `handler.rs` (fork changes) | 1 test (failing) | ~30% | Low coverage on the critical integration path |
| `session.rs` (fork changes) | 1 test (failing) | ~20% | Low coverage on composition+tracing path |

## Suggested Refactoring Priorities

### Highest ROI (Most safety for least churn)

1. **Add recursion depth counter** (P0-1) - ~20 lines of code in `executor/mod.rs`. Add a `depth: usize` field to `ExecutionContext`, increment on each `execute_tool` call to a composition, and error if it exceeds MAX_REFERENCE_DEPTH (100). This eliminates the stack overflow risk.

2. **Fix `aggregate()` Merge bug** (P0-5) - ~5 lines. Change `Merge` to operate on the current `result` value instead of the original `values` parameter.

3. **Add types_compat roundtrip tests** (P2-2) - ~200 lines of test code. For each top-level type, verify `original == from_proto(to_proto(original))`. This catches silent data loss in the conversion layer.

4. **Pre-compile JSONPaths in aggregation ops** (P3-3) - ~50 lines. Parse JSONPaths during `CompiledRegistry::compile()` and store as `CompiledAggregationOp` variants, similar to how `CompiledFieldSource` already works.

5. **Replace `Value::clone()` with `Arc<Value>` for broadcast inputs** (P3-1, P3-2) - ~100 lines. Use `Arc<Value>` for the input passed to scatter targets and pipeline steps that don't modify it.

### Medium-term refactoring

6. **Unify tool naming** (P0-2) - The CLAUDE.md "Tool Algebra" design is sound. Replace underscore-based routing with `(server, name)` tuples internally. This is the highest-churn change but eliminates the most pervasive source of fragility.

7. **Consolidate type systems** (P1-3) - Either generate hand-written types from proto (with custom serde derives) or drop proto types. This eliminates types_compat.rs entirely.

8. **Add composition-graph cycle detection to validation** (P1-8) - Walk `ToolCall` references in pipeline steps and scatter targets during validation, not just the `depends` field.

## Architectural Concerns: Fork vs Upstream

### Merge risk
- The fork modifies `handler.rs`, `session.rs`, and `router.rs` - core proxy files that upstream actively develops. The handler.rs diff is ~400 lines of interleaved changes, making merge conflicts likely.
- The `DELIMITER` / `resource_name` / `parse_resource_name` functions are the highest-risk merge points since upstream may also evolve tool naming.

### Upstream divergence
- Upstream has added `llm/` module refactoring, embeddings support, tunnel protocols, and ext_authz since the fork. These don't conflict with the registry/composition work but increase drift.
- The fork's proto additions (`registry.proto`) are clean extensions that don't modify upstream proto files.

### Long-term sustainability
- The registry/composition system is a substantial feature (~5000 new lines of Rust). If not upstreamed, it will require ongoing merge maintenance for every upstream release.
- The `types_compat.rs` layer is the weakest link: it must track changes in BOTH proto definitions and hand-written types, and has no tests to catch regressions.
- The execution engine is well-isolated behind the `ToolInvoker` trait, which is a good abstraction boundary. If upstream changes the backend calling convention, only `RelayToolInvoker` needs updating.

### Recommendation
The fork adds genuine value (DAG composition, virtual tools, agent identity scoping) that the upstream doesn't have. The cleanest path forward is:
1. Fix P0-1 (recursion) and P0-5 (aggregate bug) immediately
2. Add types_compat tests (P2-2) to prevent silent regressions
3. Propose the registry.proto additions to upstream (they're backward-compatible extensions)
4. If upstreaming the executor, first implement the Tool Algebra refactoring (P0-2) to eliminate the underscore delimiter
