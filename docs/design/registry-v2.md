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
2. [Architecture Intent](#architecture-intent)
3. [Design Goals](#design-goals)
4. [Schema Overview](#schema-overview)
5. [Entity Definitions](#entity-definitions)
6. [Dependency Management](#dependency-management)
7. [Validation Rules](#validation-rules)
8. [Runtime Architecture](#runtime-architecture)
9. [Phased Implementation](#phased-implementation)
10. [Migration Path](#migration-path)
11. [Work Packages](#work-packages)

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

## Architecture Intent

### The Registry as a Local Cache

The registry JSON file (e.g., `registry-v2.json`) is a **local cache** of data that would be served by a future **centralized Registry Service**. The JSON schema represents the RPC response format you'd receive from that service.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   FUTURE: Centralized Registry Service                       │
│                                                                             │
│   ┌───────────────────────────────────────────────────────────────────┐    │
│   │  Registry Service (gRPC/HTTP API)                                  │    │
│   │                                                                    │    │
│   │  • Source of truth for all entities                               │    │
│   │  • Registration-time validation (reject invalid entries)          │    │
│   │  • Version history and audit logs                                 │    │
│   │  • Search, discovery, deprecation workflows                       │    │
│   │  • Push notifications to gateways on changes                      │    │
│   └──────────────────────────────────────────────────────────────────┘    │
│                                    │                                        │
│                                    │ sync / cache                           │
│                                    ▼                                        │
│   ┌──────────────────────────────────────────────────────────────────┐     │
│   │  AgentGateway Instances (with local cache)                        │     │
│   │                                                                   │     │
│   │  • Cache subset relevant to this gateway's clients/backends      │     │
│   │  • Runtime validation (enforce what was registered)              │     │
│   │  • Periodic refresh from Registry Service                        │     │
│   │  • Fallback to cached data if service unavailable                │     │
│   └──────────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘

CURRENT: For development, the JSON file serves as a mock of the Registry Service.
         Gateways load it directly from disk or HTTP endpoint.
```

### Governance Split: Registration vs Runtime

Governance is enforced at two levels:

| Level | When | Who Enforces | Examples |
|-------|------|--------------|----------|
| **Registration-time** | When entity is added to registry | Registry Service | Reject circular deps, schema conflicts, invalid versions |
| **Runtime** | When entity is used | AgentGateway | Validate I/O, enforce caller dependencies, policy violations |

Registration-time validation catches errors early (before deployment). Runtime validation enforces the contracts established at registration.

### Gateway Ownership: Infrastructure, Not Applications

AgentGateway is **infrastructure middleware**, owned by platform/infra teams - similar to Istio sidecars:

- **Not** owned by agent application teams
- **Not** owned by MCP server teams
- **Transparent** when working correctly
- **Configured centrally** via Helm charts, GitOps, etc.

Deployment topology is flexible:
- Sidecar next to each agent
- Centralized gateway cluster
- Regional mesh gateways
- Hybrid approaches

The registry and virtual tools enable **policy-driven tool abstraction** without requiring agent or server teams to coordinate directly.

### Phase 1 Focus: Agent-to-MCP-Tool

This design supports both MCP tools and A2A agents, but **Phase 1 focuses on MCP tool access**:

```
Phase 1 (MVP):
  Agent ──▶ AgentGateway ──▶ MCP Servers (tools)
            │
            └── Registry provides:
                • Tool versioning
                • Virtual tools (aliases, compositions)
                • Dependency-scoped discovery
                • SBOM tracking

Phase 2 (Future):
  Agent ──▶ AgentGateway ──▶ Other A2A Agents (agent-as-tool)
            │
            └── Registry adds:
                • A2A agent multiplexing
                • Agent skill invocation in compositions
                • Strongly-typed A2A calls via skill schemas
```

Agents are stored in the registry now (with `depends` clauses) to enable:
- Dependency-scoped tool discovery
- SBOM export
- Future A2A features (Phase 2)

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

## Runtime Architecture

This section describes how the gateway actually enforces the registry at runtime. This is the **critical path** from design to working demo.

### Current State vs Target State

| Capability | MCP (Current) | A2A (Current) | Phase 1 (v2) | Phase 2 (Future) |
|------------|---------------|---------------|--------------|------------------|
| Backend multiplexing | ✅ Multiple | ❌ Single | ✅ MCP only | ✅ Both |
| Virtual tools | ✅ Yes | ❌ N/A | ✅ Yes | ✅ Yes |
| Caller identity | ❌ None | ❌ None | ✅ Required | ✅ Required |
| Dependency scoping | ❌ No | ❌ No | ✅ Yes (MCP) | ✅ Yes (both) |
| Agent-as-tool | ❌ No | ❌ N/A | ❌ No | ✅ Yes |

### Caller Identity

Before we can enforce dependencies, we need to know WHO is calling.

**MCP Callers:**
```
┌─────────────────────────────────────────────────────────────────┐
│  How does the gateway identify an MCP client?                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Option 1: Header-based identity                                │
│    Client sends: X-Agent-Name: research-agent                   │
│                  X-Agent-Version: 2.1.0                         │
│                                                                 │
│  Option 2: OAuth/JWT claims                                     │
│    JWT contains: { "agent_name": "...", "agent_version": "..." }│
│                                                                 │
│  Option 3: Client registration (OAuth Dynamic Client Reg)      │
│    Client registers and gets credentials tied to identity       │
│                                                                 │
│  Option 4: MCP ClientInfo                                       │
│    Initialize params: { "clientInfo": { "name": "...", ... } }  │
│    ⚠️ MCP spec has clientInfo but no version field!            │
│                                                                 │
│  RECOMMENDATION: Use headers + fallback to clientInfo.name      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**A2A Callers (Agent-to-Agent):**
```
┌─────────────────────────────────────────────────────────────────┐
│  How does the gateway identify an A2A caller?                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Option 1: Require caller to send their AgentCard URL           │
│    Header: X-Caller-Agent: http://research-agent:9000           │
│    Gateway fetches card to verify identity                      │
│                                                                 │
│  Option 2: Mutual TLS with agent certificates                   │
│    Cert CN contains agent identity                              │
│                                                                 │
│  Option 3: OAuth with agent-specific credentials                │
│    Token introspection reveals agent identity                   │
│                                                                 │
│  RECOMMENDATION: X-Caller-Agent header + optional mTLS          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Dependency-Scoped Tool Discovery

When a registered agent calls `tools/list`, return only tools it depends on:

```
┌─────────────────────────────────────────────────────────────────┐
│                    tools/list Flow (v2)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Client sends: tools/list                                    │
│     Headers: X-Agent-Name: research-agent                       │
│              X-Agent-Version: 2.1.0                             │
│                                                                 │
│  2. Gateway looks up agent in registry:                         │
│     agents["research-agent"]["2.1.0"].depends = [               │
│       { type: "tool", name: "search_documents", version: "1.0.0" },
│       { type: "tool", name: "fetch", version: "1.2.3" }         │
│     ]                                                           │
│                                                                 │
│  3. Gateway filters tools to only those in depends              │
│                                                                 │
│  4. Response: { tools: [search_documents, fetch] }              │
│                                                                 │
│  FALLBACK: Unknown caller → return all tools (configurable)     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Code Location:** `crates/agentgateway/src/mcp/handler.rs` → `merge_tools()`

Current code does RBAC filtering. We need to add dependency filtering:

```rust
// Current (simplified):
let tools = transformed_tools
    .filter(|(server, tool)| policies.validate(...))  // RBAC
    .collect();

// Target (v2):
let tools = transformed_tools
    .filter(|(server, tool)| policies.validate(...))  // RBAC
    .filter(|(_, tool)| {
        // NEW: Dependency filter
        match caller_identity {
            Some(agent) => registry.agent_depends_on(&agent, &tool.name, &tool.version),
            None => config.allow_unknown_caller,
        }
    })
    .collect();
```

### Agent Multiplexing (A2A) — *Phase 2*

> **Note**: This section describes Phase 2 functionality. Phase 1 focuses on MCP tool access only.

Currently A2A routes to a single backend. Phase 2 would add multiplexing like MCP:

```
┌─────────────────────────────────────────────────────────────────┐
│                    A2A Routing (Phase 2)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Current:                                                       │
│    /a2a/* → Single backend agent                                │
│                                                                 │
│  Phase 2:                                                       │
│    /a2a/research-agent/* → research-agent backend               │
│    /a2a/summarizer/* → summarizer backend                       │
│                                                                 │
│  OR (discovery-based):                                          │
│    /.well-known/agents → List all registered agents             │
│    /a2a → Routes based on message content (task.agent_name?)    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Implementation Options (Phase 2):**

1. **Path-based routing**: `/a2a/{agent-name}/...`
   - Simple, explicit
   - Requires clients to know agent names

2. **Discovery endpoint**: `/.well-known/agents` returns aggregated agent list
   - Client discovers available agents
   - Then calls specific agent by name

3. **Content-based routing**: Parse A2A message, route by recipient
   - More complex but allows multi-agent orchestration

### Agent-as-Tool Invocation — *Phase 2*

> **Note**: This section describes Phase 2 functionality.

Compositions could call agents like tools:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Agent-as-Tool (v2)                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Pipeline step can specify:                                     │
│    { "operation": { "tool": { "name": "fetch" } } }             │
│                    OR                                           │
│    { "operation": { "agent": { "name": "summarizer",            │
│                                "skill": "summarize" } } }       │
│                                                                 │
│  When executing agent step:                                     │
│    1. Look up agent in registry → get URL                       │
│    2. Map pipeline input → A2A DataPart (using skill schema)    │
│    3. Send A2A message/send to agent                            │
│    4. Extract result from A2A response                          │
│    5. Map to pipeline output                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Proto Update Needed:**

```protobuf
message StepOperation {
  oneof op {
    ToolCall tool = 1;
    AgentCall agent = 2;  // NEW
    PatternSpec pattern = 3;
  }
}

message AgentCall {
  string name = 1;     // Agent name
  string skill = 2;    // Skill ID
  // Version resolved from depends declaration
}
```

### End-to-End Demo Flow (Phase 1)

Here's how the Phase 1 demo works, focused on MCP tool access:

```
┌─────────────────────────────────────────────────────────────────┐
│                    E2E Demo Flow (Phase 1)                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. STARTUP                                                     │
│     - Gateway loads registry-v2.json (local cache)              │
│     - Validates all dependencies                                │
│     - Connects to MCP backends (document-service, etc.)         │
│                                                                 │
│  2. TOOL DISCOVERY (scoped by caller identity)                  │
│     Client: POST /mcp { method: "tools/list" }                  │
│     Headers: X-Agent-Name: research-agent, X-Agent-Version: 2.1.0
│     Response: Only tools research-agent depends on              │
│                                                                 │
│  3. TOOL CALL (with validation)                                 │
│     Client: POST /mcp { method: "tools/call",                   │
│                         params: { name: "search_documents" } }  │
│     Gateway: Validates agent has this dependency                │
│     Gateway: Routes to document-service (versioned)             │
│                                                                 │
│  4. VIRTUAL TOOL CALL                                           │
│     Client: POST /mcp { method: "tools/call",                   │
│                         params: { name: "search_all" } }        │
│     Gateway: Resolves virtual tool to scatter-gather            │
│     Gateway: Calls multiple backends in parallel                │
│     Gateway: Aggregates and returns results                     │
│                                                                 │
│  5. COMPOSITION CALL (MCP tools only)                           │
│     Client: POST /mcp { method: "tools/call",                   │
│                         params: { name: "order_saga" } }        │
│     Gateway: Executes saga pattern with MCP tools               │
│     Gateway: Handles compensation on failure                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Phase 2 Demo (Future)

Phase 2 adds A2A agent invocation:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Additional in Phase 2                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  AGENT DISCOVERY                                                │
│     Client: GET /.well-known/agents                             │
│     Response: List of registered agents                         │
│                                                                 │
│  AGENT-AS-TOOL COMPOSITION                                      │
│     Pipeline steps can invoke A2A agents as tools               │
│     - Input mapped to A2A DataPart                              │
│     - Output extracted from A2A response                        │
│     - Schema validation via skill inputSchema/outputSchema      │
│                                                                 │
│  DIRECT A2A (multiplexed)                                       │
│     Client: POST /a2a/research-agent { method: "message/send" } │
│     Gateway: Routes to correct agent backend                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Runtime Work Packages Summary

**Phase 1 (MVP) - Agent-to-MCP-Tool:**

**WP10: Caller Identity**
- Extract caller identity from headers/JWT/MCP clientInfo
- Add to request context for downstream use
- Files: `src/mcp/identity.rs`

**WP11: Dependency-Scoped Discovery**
- Modify `merge_tools()` to filter by caller's depends
- Add configuration for unknown caller policy
- Files: `src/mcp/handler.rs`

**WP14: Tool Discovery Endpoint** (MCP only)
- Scoped `tools/list` already exists
- Add versioned tool metadata

**Phase 2 (Future) - Agent-as-Tool:**

**WP12: Agent Multiplexing** — *Phase 2*
- Implement agent registry lookup
- Add path-based or discovery-based routing
- Files: `src/a2a/router.rs` (new), `src/a2a/mod.rs`

**WP13: Agent-as-Tool Executor** — *Phase 2*
- Implement `AgentCall` in composition executor
- Map tool I/O to A2A DataPart
- Files: `src/mcp/registry/executor/agent.rs` (new)

**WP15: Agent Discovery Endpoints** — *Phase 2*
- `/.well-known/agents` - list registered agents
- Aggregated agent cards with gateway URLs
- Files: `src/a2a/discovery.rs` (new)

## Phased Implementation

### Phase 1: Agent-to-MCP-Tool (MVP)

Focus on enabling agents to access MCP tools through the gateway with versioning and dependency tracking.

**Scope:**
- ✅ Full registry data model (schemas, servers, tools, agents)
- ✅ Versioned tools with source references
- ✅ Virtual tools (aliases, projections, compositions)
- ✅ Agent definitions with SBOM `depends` clauses
- ✅ Caller identity extraction (headers, MCP clientInfo)
- ✅ Dependency-scoped `tools/list`
- ✅ Startup validation (dependencies, schemas)
- ✅ Runtime validation (optional, configurable)
- ✅ SBOM export

**Out of Scope for Phase 1:**
- ❌ A2A agent multiplexing
- ❌ Agent-as-tool invocation in compositions
- ❌ `/.well-known/agents` discovery endpoint
- ❌ Strongly-typed A2A calls via skill schemas

### Phase 2: Agent-as-Tool (Future)

Extend the gateway to invoke A2A agents as steps in compositions.

**Scope:**
- A2A agent multiplexing (path-based routing)
- Agent skill invocation with schema validation
- Compositions that mix MCP tools and A2A agents
- Agent discovery endpoints

This will be designed once Phase 1 is validated and working.

---

## Implementation Roadmap

### TDD Approach

1. **Update IR (Protobuf)** - Define the data structures
2. **Update Interfaces** - TypeScript DSL and Rust types
3. **Write Failing Tests** - Tests that compile but fail
4. **Implement Parsing** - JSON → IR conversion
5. **Implement Validation** - Startup checks
6. **Implement Runtime** - Caller identity, scoped discovery

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

### Agent-as-Tool — *Phase 2*

> **Note**: This pattern is Phase 2 functionality.

Using an agent skill as a step in a pipeline (Phase 2):

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

## Appendix B: A2A DataPart Validation — *Phase 2*

> **Note**: This section describes Phase 2 functionality.

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
