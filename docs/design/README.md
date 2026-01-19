# Design Documents

This directory contains design documents for major AgentGateway features.

## Registry v2 (In Progress)

A versioned, SBOM-like registry system for managing tools, agents, and their dependencies.

| Document | Description |
|----------|-------------|
| [registry-v2.md](./registry-v2.md) | Main design document |
| [registry-v2-work-packages.md](./registry-v2-work-packages.md) | Implementation roadmap with TDD test stubs |

**Example Registry**: [`examples/pattern-demos/configs/registry-v2-example.json`](../../examples/pattern-demos/configs/registry-v2-example.json)

### Key Features

- **Versioned Schemas**: Reusable JSON Schemas with semantic versioning
- **Server Registration**: MCP servers declare which tool versions they provide
- **Agent Registration**: A2A agents with skill schemas and dependency declarations
- **Dependency Tracking**: Tools and agents declare exact version dependencies
- **Validation**: Startup and runtime contract enforcement

### Work Package Status

| WP | Name | Status | Owner |
|----|------|--------|-------|
| 1 | Proto Schema | Not Started | - |
| 2 | Rust Types | Not Started | - |
| 3 | Rust Validation | Not Started | - |
| 4 | Runtime Hooks | Not Started | - |
| 5 | TypeScript Types | Not Started | - |
| 6 | TypeScript Validation | Not Started | - |
| 7 | SBOM Export | Not Started | - |
| 8 | Test Suite | Not Started | - |
| 9 | Documentation | Not Started | - |

## Other Design Documents

| Document | Description |
|----------|-------------|
| [composition-test-plan.md](./composition-test-plan.md) | Test plan for tool composition patterns |
| [registry-integration.md](./registry-integration.md) | Original registry integration design |
| [saga-pattern.md](./saga-pattern.md) | Saga pattern for distributed transactions |
