---
type: reference
original_path: docs/design/README.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Design Documents Index (imported)

Source: `docs/design/README.md`

## Quick Links
- [[references/imported/quickstart|Quickstart]] — get the eCommerce demo running in 5 minutes
- [[references/imported/virtual-tools-vision|Virtual Tools Vision]] — what works, what's not implemented
- [[references/imported/code-walkthrough|Code Walkthrough]] — where the code lives

## Reference Documents
- [[references/imported/registry-v2|Registry v2]] — full registry v2 specification
- [[references/imported/proto-codegen-migration|Proto Codegen Migration]] — proto codegen implementation (complete)
- [[references/imported/composition-test-plan|Composition Test Plan]] — test plan for compositions
- [[references/imported/fastmcp-transforms-comparison|FastMCP Comparison]] — comparison with FastMCP transforms

## Current Working Features
- Virtual tools (1:1): Rename, defaults, field hiding, output transforms
- Compositions (N:1): Pipeline, Scatter-Gather, Filter, SchemaMap, MapEach
- Data binding: input, step, constant, construct
- Registry v2: schemas, servers, agents, dependency-scoped discovery
- Proto codegen: TypeScript DSL to Proto to Rust runtime

## Not Yet Implemented
- Stateful patterns: Saga, Retry, Timeout, Cache, Circuit Breaker
- Parallel DAG execution
- Agent-as-tool (A2A agents as composition steps)

## Archive
Historical design documents in `docs/design/archive/`:
- `dag-executor.md`, `saga-pattern.md`, `registry-integration.md`, `registry-v2-work-packages.md`, `ecommerce-service-separation.md`
