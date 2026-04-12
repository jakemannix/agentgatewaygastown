---
type: reference
original_path: docs/ARCHITECTURE_DISCONNECTS.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Architecture Disconnects: Current State vs. Target (imported)

Source: `docs/ARCHITECTURE_DISCONNECTS.md`

## Target Vision
```
TypeScript (constrained subset) → compile → Schematized JSON IR → deploy → Go/Rust runtime (gateway)
```

## Five Key Disconnects Identified

### 1. No TypeScript-to-JSON Compiler Path
- vMCP shows pseudo-code, agentgateway accepts hand-written JSON
- No defined workflow from authoring to deployment
- **Status**: Partially addressed by vmcp-dsl TypeScript package and vmcp-compile CLI

### 2. No Schematized JSON IR Specification
- Agentgateway had ad-hoc JSON format (registry.json)
- No formal IR definition or standard composition representation
- **Status**: Addressed by registry.proto as single source of truth

### 3. Limited Pattern Support
- Only Tool Adapter (1:1 transforms) was initially implemented
- 15+ patterns were missing (pipeline, router, scatter-gather, etc.)
- **Status**: Core patterns (Pipeline, ScatterGather, Filter, SchemaMap, MapEach) now implemented

### 4. Type System Mismatch
- vMCP uses functional type signatures: `(A -> B) x [Tool] -> Tool`
- Agentgateway uses JSON Schema + JSONPath (practical but no formal type inference)
- No type checker to verify composition safety
- **Status**: Still a gap; JSON Schema validation at build time is the workaround

### 5. Control Plane vs. Data Plane Confusion
- Unclear boundary between what's data plane vs. control plane responsibility
- Should composition execution happen in gateway or external service?
- **Status**: Resolved — compositions execute in the gateway (data plane)

## Recommended Fix Phases
1. Define Schematized JSON IR Spec (done — registry.proto)
2. TypeScript DSL to JSON Compiler (done — vmcp-dsl)
3. Extend Agentgateway to Execute IR (done — composition executor)
4. Integration (done — end-to-end working)

## Related Notes
- [[references/imported/ALIGNMENT_WITH_AGENTGATEWAY|Alignment Analysis]] — complementary analysis
- [[references/imported/proto-codegen-migration|Proto Codegen Migration]] — how the IR was formalized
- [[references/imported/DESIGN_TS_TO_RUNTIME|TS to Runtime Design]] — the three-layer implementation
