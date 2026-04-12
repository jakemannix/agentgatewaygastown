---
type: reference
original_path: docs/design/registry-v2.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Registry v2 Design Document (imported)

Source: `docs/design/registry-v2.md`

**Status**: Draft | **Target Release**: 0.9.0

## Purpose
Introduces a versioned, SBOM-like system for managing tools, agents, and their dependencies. Enables reproducible deployments, contract enforcement, and safe upgrades.

## Top-Level Schema
```
Registry
  schemaVersion: "2.0"
  schemas: Schema[]       — reusable JSON Schema definitions
  servers: Server[]       — backend service metadata
  tools: Tool[]           — virtual tools and compositions
  agents: Agent[]         — agent metadata with dependencies
```

## Key Architecture Decisions

### Registry as Local Cache
The JSON file is a local cache of what a future centralized Registry Service would serve. The schema represents the RPC response format.

### Governance Split
- **Registration-time validation** (Registry Service): reject circular deps, schema conflicts, invalid versions
- **Runtime validation** (AgentGateway): validate I/O, enforce caller dependencies, policy violations

### Gateway Ownership
AgentGateway is **infrastructure middleware** (like Istio sidecars) — owned by platform/infra teams, not agent application teams. Configured centrally via Helm/GitOps.

### Phase 1 Focus: Agent-to-MCP-Tool
Agents stored in registry now for dependency-scoped discovery and SBOM export. Phase 2 adds A2A agent multiplexing and skill-based routing.

## Design Goals
- **Reproducibility**: deterministic deployments from registry file
- **Contract Enforcement**: schema mismatches detected at startup or runtime
- **Composability**: agents and tools interchangeable in pipelines
- **Backwards Compatible**: v1 registries continue to work (with warnings)

## Entity Identification
All entities use `(name, version)` as unique identifier with semantic versioning. Dependencies reference specific versions.

## Related Notes
- [[references/imported/agent-identity|Agent Identity]] — how agents identify themselves
- [[references/imported/virtual-tools|Virtual Tools]] — tool definitions
- [[references/imported/proto-codegen-migration|Proto Codegen]] — how this schema became proto
