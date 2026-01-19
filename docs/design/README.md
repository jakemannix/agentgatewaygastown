# Design Documents

This directory contains design documents for major AgentGateway features.

## Registry v2 (In Progress)

A versioned, SBOM-like registry system for managing tools, agents, and their dependencies.

| Document | Description |
|----------|-------------|
| [registry-v2.md](./registry-v2.md) | Main design document |
| [registry-v2-work-packages.md](./registry-v2-work-packages.md) | Implementation roadmap with TDD test stubs |

**Example Registry**: [`examples/pattern-demos/configs/registry-v2-example.json`](../../examples/pattern-demos/configs/registry-v2-example.json)

### Architecture Intent

The registry JSON file is a **local cache** of what would be served by a future centralized Registry Service. The gateway enforces at runtime what was validated at registration time.

See [registry-v2.md § Architecture Intent](./registry-v2.md#architecture-intent) for details.

### Key Features

- **Versioned Schemas**: Reusable JSON Schemas with semantic versioning
- **Server Registration**: MCP servers declare which tool versions they provide
- **Agent Registration**: Agents with dependency declarations (for tool scoping)
- **Dependency Tracking**: Tools and agents declare exact version dependencies
- **Validation**: Startup and runtime contract enforcement

### Phased Implementation

**Phase 1 (MVP)**: Agent-to-MCP-Tool
- Agents access versioned MCP tools through gateway
- Dependency-scoped `tools/list`
- Virtual tools (aliases, compositions)
- SBOM tracking

**Phase 2 (Future)**: Agent-as-Tool
- A2A agent multiplexing
- Agent skill invocation in compositions
- Strongly-typed A2A calls

### Work Package Status (Phase 1)

**Foundation (can start immediately):**

| WP | Name | Status | Owner | Est. |
|----|------|--------|-------|------|
| 1 | Proto Schema | Not Started | - | 2-3d |
| 5 | TypeScript Types | Not Started | - | 2-3d |
| 8 | Test Suite | Not Started | - | 3-4d |

**Data Layer (depends on WP1):**

| WP | Name | Status | Owner | Est. |
|----|------|--------|-------|------|
| 2 | Rust Types | Not Started | - | 3-4d |
| 3 | Rust Validation | Not Started | - | 4-5d |
| 6 | TypeScript Validation | Not Started | - | 2-3d |
| 7 | SBOM Export | Not Started | - | 2-3d |

**Runtime (critical path for Phase 1 demo):**

| WP | Name | Status | Owner | Est. |
|----|------|--------|-------|------|
| 10 | Caller Identity | Not Started | - | 2-3d |
| 11 | Dependency-Scoped Discovery | Not Started | - | 3-4d |
| 14 | Discovery Endpoints (MCP) | Not Started | - | 2-3d |
| 4 | Runtime Hooks | Not Started | - | 3-4d |

**Final:**

| WP | Name | Status | Owner | Est. |
|----|------|--------|-------|------|
| 9 | Documentation | Not Started | - | 2-3d |

### Work Packages (Phase 2 - Deferred)

| WP | Name | Status | Est. |
|----|------|--------|------|
| 12 | A2A Multiplexing | Deferred | 4-5d |
| 13 | Agent-as-Tool Executor | Deferred | 4-5d |
| 15 | Agent Discovery Endpoints | Deferred | 2-3d |

## Other Design Documents

| Document | Description |
|----------|-------------|
| [composition-test-plan.md](./composition-test-plan.md) | Test plan for tool composition patterns |
| [registry-integration.md](./registry-integration.md) | Original registry integration design |
| [saga-pattern.md](./saga-pattern.md) | Saga pattern for distributed transactions |
