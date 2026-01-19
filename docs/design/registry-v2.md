# Registry v2 Design Document

**Status**: Draft  
**Authors**: Platform Team  
**Created**: 2026-01-18  
**Target Release**: 0.9.0

## Executive Summary

Registry v2 introduces a versioned, SBOM-like system for managing tools, agents, and their dependencies. This enables reproducible deployments, contract enforcement, and safe upgrades across the AgentGateway ecosystem.

---

## Table of Contents

1. [Motivation](#motivation)
2. [Design Goals](#design-goals)
3. [Schema Overview](#schema-overview)
4. [Entity Definitions](#entity-definitions)
5. [Dependency Management](#dependency-management)
6. [Validation Rules](#validation-rules)
7. [Migration Path](#migration-path)
8. [Implementation Roadmap](#implementation-roadmap)
9. [Work Packages](#work-packages)

---

## Motivation

### Current State (v1)

The v1 registry supports only tools with no versioning:

```json
{
  "schemaVersion": "1.0",
  "tools": [
    { "name": "search", "source": { "target": "backend", "tool": "search" } }
  ]
}
```

**Limitations:**
- No tool versioning → breaking changes silently propagate
- No agent registration → no dependency tracking for A2A
- No schema reuse → duplicated definitions across tools
- No SBOM → can't audit what versions are deployed

### Desired State (v2)

```json
{
  "schemaVersion": "2.0",
  "schemas": [...],
  "servers": [...],
  "tools": [...],
  "agents": [...]
}
```

**Capabilities:**
- All entities versioned with semantic versions
- Explicit dependency declarations (tools → tools, agents → tools, agents → agents)
- Reusable, versioned schemas
- Startup and runtime validation
- SBOM export for compliance

---

## Design Goals

| Goal | Description |
|------|-------------|
| **Reproducibility** | Given a registry file, deployments are deterministic |
| **Contract Enforcement** | Schema mismatches detected at startup or runtime |
| **Composability** | Agents and tools interchangeable in pipelines |
| **Backwards Compatible** | v1 registries continue to work (with warnings) |
| **Parallel Development** | Modular design for concurrent implementation |

---

## Schema Overview

### Top-Level Structure

```
Registry
├── schemaVersion: "2.0"
├── schemas: Schema[]
├── servers: Server[]
├── tools: Tool[]
└── agents: Agent[]
```

### Entity Identification

All entities use `(name, version)` as their unique identifier:

```json
{ "name": "search_documents", "version": "1.0.0" }
```

### Reference Format

References to entities use typed objects:

```json
{ "type": "tool", "name": "search_documents", "version": "1.0.0" }
{ "type": "agent", "name": "research-agent", "version": "2.1.0", "skill": "research_topic" }
{ "type": "schema", "name": "SearchQuery", "version": "1.0.0" }
```

Or shorthand in JSON Schema refs:

```json
{ "$ref": "#SearchQuery:1.0.0" }
```

---

## Entity Definitions

### Schema

Reusable JSON Schema definitions with versioning.

```typescript
interface Schema {
  name: string;
  version: string;  // semver
  description?: string;
  schema: JSONSchema;  // The actual JSON Schema
  metadata?: Record<string, unknown>;
}
```

**Example:**
```json
{
  "name": "SearchQuery",
  "version": "1.0.0",
  "description": "Standard search query input",
  "schema": {
    "type": "object",
    "properties": {
      "query": { "type": "string" },
      "limit": { "type": "integer", "default": 10 }
    },
    "required": ["query"]
  }
}
```

### Server

MCP server registration declaring provided tools.

```typescript
interface Server {
  name: string;
  version: string;  // semver
  description?: string;
  provides: ToolProvision[];  // Tools this server provides
  deprecated?: boolean;
  deprecationMessage?: string;
  metadata?: Record<string, unknown>;
}

interface ToolProvision {
  tool: string;     // Tool name
  version: string;  // Tool version this server provides
}
```

**Example:**
```json
{
  "name": "document-service",
  "version": "1.2.0",
  "description": "SQLite-backed document service",
  "provides": [
    { "tool": "search_documents", "version": "1.0.0" },
    { "tool": "create_document", "version": "1.1.3" }
  ]
}
```

### Tool

Tool definition with optional source (passthrough) or spec (composition).

```typescript
interface Tool {
  name: string;
  version: string;  // semver
  description?: string;
  
  // Implementation: exactly one of source or spec
  source?: ToolSource;  // Passthrough to backend
  spec?: PatternSpec;   // Composition
  
  // Dependencies (for compositions)
  depends?: Dependency[];
  
  // Schemas (inline or ref)
  inputSchema?: JSONSchema | SchemaRef;
  outputSchema?: JSONSchema | SchemaRef;
  
  // Transformation (optional)
  outputTransform?: OutputTransform;
  
  metadata?: Record<string, unknown>;
}

interface ToolSource {
  server: string;         // Server name
  serverVersion: string;  // Server version
  tool: string;           // Original tool name on server
  defaults?: Record<string, unknown>;  // Injected defaults
  hideFields?: string[];  // Fields to hide from schema
}

interface SchemaRef {
  $ref: string;  // Format: "#SchemaName:Version"
}
```

### Agent

A2A agent registration extending the standard AgentCard.

```typescript
interface Agent {
  // Standard A2A AgentCard fields
  name: string;
  version: string;
  description: string;
  url: string;
  protocolVersion: string;
  
  defaultInputModes: string[];
  defaultOutputModes: string[];
  
  skills: AgentSkill[];
  capabilities: AgentCapabilities;
  
  provider?: AgentProvider;
  security?: SecurityRequirement[];
  securitySchemes?: Record<string, SecurityScheme>;
  
  // Extended fields (via capabilities.extensions)
  // Dependencies declared in urn:agentgateway:sbom extension
}

interface AgentSkill {
  id: string;
  name: string;
  description: string;
  tags: string[];
  examples?: string[];
  
  inputModes: string[];
  outputModes: string[];
  
  // NEW: Schemas for DataPart validation
  inputSchema?: JSONSchema | SchemaRef;
  outputSchema?: JSONSchema | SchemaRef;
}

interface AgentCapabilities {
  streaming?: boolean;
  pushNotifications?: boolean;
  stateTransitionHistory?: boolean;
  extensions: AgentExtension[];
}

interface AgentExtension {
  uri: string;  // e.g., "urn:agentgateway:sbom"
  description?: string;
  required?: boolean;
  params: Record<string, unknown>;
}
```

### Dependency

Typed reference to a tool or agent.

```typescript
interface Dependency {
  type: "tool" | "agent";
  name: string;
  version: string;
  skill?: string;  // Required when type is "agent"
}
```

---

## Dependency Management

### Declaration

Tools and agents declare dependencies explicitly:

```json
{
  "name": "research_pipeline",
  "version": "1.0.0",
  "depends": [
    { "type": "tool", "name": "fetch", "version": "1.2.3" },
    { "type": "agent", "name": "summarizer", "version": "2.0.0", "skill": "summarize" }
  ],
  "spec": { "pipeline": { ... } }
}
```

For agents, dependencies go in the SBOM extension:

```json
{
  "capabilities": {
    "extensions": [
      {
        "uri": "urn:agentgateway:sbom",
        "params": {
          "depends": [
            { "type": "tool", "name": "search_documents", "version": "1.0.0" }
          ]
        }
      }
    ]
  }
}
```

### Resolution

At startup, the gateway builds a dependency graph and validates:

1. All referenced entities exist in the registry
2. No circular dependencies
3. All servers provide their declared tools
4. All schemas resolve

### No Wildcards

Version constraints like `*` or `>=1.0.0` are **not supported**. All dependencies must specify exact versions for reproducibility.

---

## Validation Rules

### Startup Validation

| Check | Severity | Description |
|-------|----------|-------------|
| Schema resolution | Error | All `$ref` must resolve to existing schemas |
| Server provisions | Error | Each server's `provides` must match registered tools |
| Tool sources | Error | Each tool's `source.server` must exist |
| Dependency resolution | Error | All `depends` entries must exist |
| Circular dependencies | Error | Dependency graph must be acyclic |
| Deprecated entities | Warning | Warn if using deprecated servers/tools |
| Unused schemas | Warning | Schemas not referenced anywhere |

### Runtime Validation

| Check | Configurable | Description |
|-------|--------------|-------------|
| Input schema validation | Yes | Validate tool/skill input against schema |
| Output schema validation | Yes | Validate tool/skill output against schema |
| Unknown caller | Yes | Allow/warn/deny unregistered agents |
| Undeclared tool use | Yes | Agent calls tool not in its depends |

### Configuration

```yaml
registry:
  source: file://./registry.json
  validation:
    startup:
      missingEntity: error
      deprecatedEntity: warn
      unusedSchema: ignore
    runtime:
      inputValidation: warn
      outputValidation: ignore
      unknownCaller: allow
      undeclaredDependency: warn
```

---

## Migration Path

### Phase 1: Schema Support

Add `schemas` section, allow `$ref` in tool schemas.

```json
{
  "schemaVersion": "2.0",
  "schemas": [...],
  "tools": [
    { "name": "search", "inputSchema": { "$ref": "#SearchQuery:1.0.0" } }
  ]
}
```

### Phase 2: Server Registration

Add `servers` section with `provides`.

```json
{
  "servers": [
    { "name": "doc-svc", "version": "1.0.0", "provides": [...] }
  ],
  "tools": [
    { "name": "search", "version": "1.0.0", "source": { "server": "doc-svc", "serverVersion": "1.0.0", ... } }
  ]
}
```

### Phase 3: Agent Registration

Add `agents` section with skills and SBOM extension.

### Phase 4: Validation Enforcement

Enable startup and runtime validation.

---

## Implementation Roadmap

### TDD Approach

1. **Update IR (Protobuf)** - Define the data structures
2. **Update Interfaces** - TypeScript DSL and Rust types
3. **Write Failing Tests** - Tests that compile but fail
4. **Implement Parsing** - JSON → IR conversion
5. **Implement Validation** - Startup checks
6. **Implement Runtime** - Runtime validation hooks

### File Changes

```
Proto changes:
  crates/agentgateway/proto/registry.proto  (major changes)

Rust changes:
  crates/agentgateway/src/mcp/registry/
    types.rs      (Schema, Server, Agent types)
    store.rs      (Loading, resolution)
    validate.rs   (NEW: validation logic)
    sbom.rs       (NEW: SBOM export)

TypeScript changes:
  packages/registry-dsl/
    src/types.ts      (Type definitions)
    src/builder.ts    (Fluent builder API)
    src/validate.ts   (Validation logic)

Test files:
  crates/agentgateway/tests/registry_v2/
    parsing.rs
    validation.rs
    resolution.rs
  packages/registry-dsl/tests/
    builder.test.ts
    validate.test.ts
```

---

## Work Packages

The implementation is divided into independent work packages that can be developed in parallel.

### WP1: Proto Schema Update

**Owner**: TBD  
**Dependencies**: None  
**Deliverables**:
- Updated `registry.proto` with Schema, Server, Agent messages
- Generated Rust and Go bindings

**Files**:
- `crates/agentgateway/proto/registry.proto`

### WP2: Rust Types and Parsing

**Owner**: TBD  
**Dependencies**: WP1  
**Deliverables**:
- Rust type definitions matching proto
- JSON deserialization with serde
- Schema reference resolution

**Files**:
- `crates/agentgateway/src/mcp/registry/types.rs`
- `crates/agentgateway/src/mcp/registry/schema_resolver.rs` (NEW)

### WP3: Rust Validation

**Owner**: TBD  
**Dependencies**: WP2  
**Deliverables**:
- Startup validation logic
- Dependency graph builder
- Cycle detection

**Files**:
- `crates/agentgateway/src/mcp/registry/validate.rs` (NEW)
- `crates/agentgateway/src/mcp/registry/dependency_graph.rs` (NEW)

### WP4: Rust Runtime Hooks

**Owner**: TBD  
**Dependencies**: WP3  
**Deliverables**:
- Input/output schema validation at call time
- Caller identity tracking
- Undeclared dependency detection

**Files**:
- `crates/agentgateway/src/mcp/registry/runtime.rs` (NEW)
- `crates/agentgateway/src/mcp/router.rs` (modifications)

### WP5: TypeScript DSL Types

**Owner**: TBD  
**Dependencies**: WP1  
**Deliverables**:
- TypeScript type definitions
- Fluent builder API for registry construction

**Files**:
- `packages/registry-dsl/src/types.ts`
- `packages/registry-dsl/src/builder.ts`

### WP6: TypeScript DSL Validation

**Owner**: TBD  
**Dependencies**: WP5  
**Deliverables**:
- Client-side validation matching Rust
- Helpful error messages

**Files**:
- `packages/registry-dsl/src/validate.ts`

### WP7: SBOM Export

**Owner**: TBD  
**Dependencies**: WP2, WP3  
**Deliverables**:
- Export registry as CycloneDX or SPDX format
- CLI command: `agentgateway sbom export`

**Files**:
- `crates/agentgateway/src/mcp/registry/sbom.rs` (NEW)
- `crates/agentgateway/src/commands/sbom.rs` (NEW)

### WP8: Test Suite

**Owner**: TBD  
**Dependencies**: WP2, WP5  
**Deliverables**:
- Comprehensive test fixtures
- Rust integration tests
- TypeScript unit tests

**Files**:
- `crates/agentgateway/tests/registry_v2/*.rs`
- `packages/registry-dsl/tests/*.test.ts`
- `examples/pattern-demos/configs/registry-v2-example.json`

### WP9: Documentation

**Owner**: TBD  
**Dependencies**: All  
**Deliverables**:
- Updated user documentation
- Migration guide
- API reference

**Files**:
- `docs/user-guide/registry-v2.md`
- `docs/migration/v1-to-v2.md`

---

## Dependency Graph (Work Packages)

```
WP1 (Proto)
 ├── WP2 (Rust Types) ──┬── WP3 (Rust Validation) ── WP4 (Runtime)
 │                      └── WP7 (SBOM)
 └── WP5 (TS Types) ──── WP6 (TS Validation)

WP8 (Tests) depends on: WP2, WP5
WP9 (Docs) depends on: All
```

---

## Example Registry

See: [`examples/pattern-demos/configs/registry-v2-example.json`](../../../examples/pattern-demos/configs/registry-v2-example.json)

This file demonstrates all v2 features including:
- Versioned schemas with `$ref`
- Server registration with `provides`
- Tools with source and composition patterns
- Agents with skill schemas and SBOM dependencies
- Advanced patterns: Pipeline, Scatter-Gather, Saga

---

## Open Questions

1. **Schema inheritance**: Should schemas support `allOf` composition?
2. **Multi-file registries**: Support for `$include` to split large registries?
3. **Remote schemas**: Allow `$ref` to external URLs?
4. **Version aliasing**: Support `latest` as alias to highest version?

---

## Appendix A: Pattern Catalog

### Pipeline

Sequential execution with data flow between steps.

```json
{
  "spec": {
    "pipeline": {
      "steps": [
        { "id": "step1", "operation": { "tool": { "name": "fetch" } } },
        { "id": "step2", "operation": { "tool": { "name": "parse" } },
          "input": { "reference": { "step": "step1", "path": "$.content" } } }
      ]
    }
  }
}
```

### Scatter-Gather

Parallel execution with result aggregation.

```json
{
  "spec": {
    "scatterGather": {
      "targets": [
        { "tool": "search_web" },
        { "tool": "search_arxiv" },
        { "agent": { "name": "internal-search", "skill": "search" } }
      ],
      "aggregation": { "ops": [{ "flatten": true }, { "dedupe": { "field": "$.url" } }] }
    }
  }
}
```

### Saga

Distributed transaction with compensation on failure.

```json
{
  "spec": {
    "saga": {
      "steps": [
        {
          "id": "reserve_inventory",
          "name": "Reserve Inventory",
          "action": { "tool": { "name": "inventory_reserve" } },
          "compensate": { "tool": { "name": "inventory_release" } }
        },
        {
          "id": "charge_payment",
          "name": "Charge Payment",
          "action": { "tool": { "name": "payment_charge" } },
          "compensate": { "tool": { "name": "payment_refund" } }
        },
        {
          "id": "ship_order",
          "name": "Ship Order",
          "action": { "tool": { "name": "shipping_create" } },
          "compensate": { "tool": { "name": "shipping_cancel" } }
        }
      ]
    }
  }
}
```

### Agent-as-Tool

Using an agent skill as a step in a pipeline.

```json
{
  "spec": {
    "pipeline": {
      "steps": [
        {
          "id": "research",
          "operation": {
            "agent": {
              "name": "research-agent",
              "skill": "research_topic"
            }
          }
        },
        {
          "id": "summarize",
          "operation": {
            "agent": {
              "name": "summarizer-agent",
              "skill": "summarize"
            }
          },
          "input": {
            "construct": {
              "fields": {
                "content": { "reference": { "step": "research", "path": "$.findings" } }
              }
            }
          }
        }
      ]
    }
  }
}
```

---

## Appendix B: A2A DataPart Validation

A2A agents receive messages containing `Part[]`:

```typescript
type Part = TextPart | FilePart | DataPart;

interface TextPart { text: string; }
interface FilePart { file: FileContent; }
interface DataPart { data: Record<string, unknown>; }  // ← Validatable
```

When a skill declares `inputSchema`, the gateway validates `DataPart.data` against it:

1. Client sends `message/send` with `parts: [{ "data": {...} }]`
2. Gateway extracts `DataPart` from parts
3. Gateway validates against skill's `inputSchema`
4. On success: forward to agent
5. On failure: reject with validation error (or warn, based on config)

This enables type-safe agent composition.
