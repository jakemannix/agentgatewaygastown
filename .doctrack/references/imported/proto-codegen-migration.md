---
type: reference
original_path: docs/design/proto-codegen-migration.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Proto Codegen Migration Plan (imported)

Source: `docs/design/proto-codegen-migration.md`

## Goal
Migrate from hand-written types to proto-generated types in both Rust and TypeScript, using JSON serialization for human-readable registry files.

```
registry.proto (Single Source of Truth)
    ├── prost + pbjson (Rust)
    ├── ts-proto via buf (TypeScript)
    └── JSON Schema (validation)
```

## Status: ALL PHASES COMPLETE

### Phase 0: Golden Test Fixtures -- COMPLETE
10 fixture files, 21 golden tests ensuring JSON serialization stability before and after migration.

### Phase 1: Infrastructure -- COMPLETE
- `pbjson-build` generates proto3 JSON-compatible serde for Rust (solves prost's oneof+serde problem)
- `buf.gen.ts.yaml` with remote ts-proto plugin for TypeScript

### Phase 2: Oneof+Serde Challenge -- COMPLETE
Key finding: prost's oneof enums don't support serde derives, but pbjson-build generates proper proto3 JSON serialization that handles oneofs correctly.

### Phase 3-4: Generate & Compatibility -- COMPLETE
Proto types compile, serialize correctly. 6 proto type tests + 21 golden tests passing. Documented field naming differences: `source.target` (v1) vs `source.server` (v2).

### Phase 5: Rust Migration -- COMPLETE (Compatibility Layer)
Architecture decision: Keep both type systems with conversion at parse boundary.
- Proto types for canonical JSON parsing (single source of truth)
- Hand-written types for runtime (with methods, builders, etc.)
- `From<proto::*>` implementations in `types_compat.rs`
- 278 Rust tests passing

### Phase 6: TypeScript Migration -- COMPLETE
- Generated 161KB `registry.ts` with `fromJSON`/`toJSON` methods
- 88 TypeScript tests pass (5 new proto tests)
- Builder API unchanged (backwards compatible)

### Phase 7-8: Visual Builder & Demos -- COMPLETE
All demos pass. Total: 278 Rust + 88 TypeScript tests.

## Key Files
- `crates/agentgateway/proto/registry.proto` — 900+ line proto definition
- `crates/agentgateway/src/types/proto.rs` — generated types
- `crates/agentgateway/src/mcp/registry/types_compat.rs` — conversion layer
- `packages/vmcp-dsl/src/generated/registry.ts` — TypeScript generated types

## Related Notes
- [[references/imported/ARCHITECTURE_DISCONNECTS|Architecture Disconnects]] — the gaps this migration resolved
- [[references/imported/registry-v2|Registry v2]] — the schema being formalized
- [[references/imported/code-walkthrough|Code Walkthrough]] — where these files live
