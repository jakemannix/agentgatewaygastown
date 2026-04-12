# Virtual Tools: Requirements Specification

**Status**: Draft v0.1
**Date**: 2026-03-21
**Author**: Jake Mannix

This document captures the functional and non-functional requirements for the
virtual tools system. It is intended to be precise enough that an implementor
could write TDD tests against it, but it does not prescribe implementation
details. Where a design choice is noted as "implementation detail," any approach
that satisfies the stated requirements is valid.

---

## 1. Purpose

Software is increasingly programmed in natural language. MCP introduced a useful
factorization: tool authors and agent authors are separate roles, turning an
N×M problem into N+M. But today, wiring tools into agents means copying tool
definitions into the agent's context window — polluting it with terminology,
field names, and schema shapes that don't fit the agent's domain which degrades
agent quality.

Virtual tools are an abstraction layer that preserves the N+M factorization
while letting agent authors customize how tools appear — via configuration: natural
language "code". The customization happens along three axes:

1. **Naming**: Tool name, description, and input field names should use the
   agent's domain language (e.g., `story_id` instead of `issue_key`), so the
   LLM interprets them correctly.

2. **Simplification**: Input schemas are projected to only the fields the agent
   needs. Fields that should be set deterministically (not left to LLM
   hallucination) get configurable defaults injected (but not shown to the LLM).

3. **Composition**: Common multi-tool workflows that an agent would otherwise
   orchestrate at runtime (burning tokens) are precompiled into a single tool
   call that the data plane executes with zero LLM involvement.

Virtual tools also serve governance teams: tool and agent registrations live in
a central registry with versioned lineage, environment tags, and lifecycle
enforcement. This is co-equal with the tool abstraction itself — the registry
is the system of record for what tools exist, what agents use them, and what's
allowed in production.

---

## 2. Personas

### Agent Author
Builds AI agents in natural language. May not be a programmer. Wants tools that
match their agent's domain model. Uses the visual builder, hand-crafted (but 
software validated) JSON, or TypeScript DSL to define virtual tools. Cares about 
eval quality — the right tool names, the right input fields, no extraneous noise.
Note: the existence of virtual tool validation and a constrained DSL means that
LLMs are able to easily generate these virtual tools as well - both offline, with 
the agent author, but potentially also online, via a CodeAct-like paradigm.

### Tool Author
Builds MCP tool servers. Publishes tools with native schemas. Doesn't know (or
care) which agents will consume them. Expects that their tool's contract is
respected — callers send valid inputs, and the tool's output schema is the
source of truth. [TODO: this needs a better paragraph]

### Platform / Governance Team
Operates the central registry control plane and gateway data plane. Manages which 
tools are deployed in which environments. Tracks which agents depend on which tool
versions. Enforces lifecycle rules (you can't delete a prod tool that has active 
dependents, you can't require patch-level version dependencies on base tools which
declare they only keep the most recent 2 patch-levels running at a time).  Cares
about lineage, auditability, and data classification.

---

## 3. Core Concepts

### Virtual Tool
A versioned tool definition in the registry that is served to agents as a normal
MCP tool.  It wraps one or more backend tools, optionally overriding the tool 
description string, customizing inputs (projection, defaults, field mapping) and 
outputs (schema mapping). From the agent's perspective, it's just a tool.

### Composition
A virtual tool whose implementation orchestrates multiple tool calls — in
parallel (scatter-gather), sequentially (pipeline), or as an arbitrary DAG
where independent branches run concurrently. A composition is a single
`tools/call` from the agent's perspective; the data plane handles the fan-out.

### Registry
A control plane service that stores versioned definitions of tools, agents,
schemas, and servers. The data plane consumes compiled snapshots from the
registry and caches them. The registry is not in the hot path.

The current file-based registry in the agentgateway prototype
(github.com/jakemannix/agentgatewaygastown, a temporary fork) is a proof of 
concept. The spec defines the API contract; the file is one dummy backing store.

### Data Plane
The runtime that serves MCP requests. It loads a registry snapshot, compiles
it, and handles `tools/list` and `tools/call` by resolving virtual tools,
executing compositions, applying transforms, and routing to backends. Must be
high-performance and agent-SDK-independent.

---

## 4. Functional Requirements

### 4.1 Registry Control Plane API

The registry exposes a CRUD API for managing tools, agents, schemas, and servers.
Format: OpenAPI + protobuf definitions.

#### 4.1.1 Tool Registration

**Create a tool**: Register a physical tool or virtual tool definition. At 
registration time:
- If the tool is a base physical tool backed by a deployed MCP Server, the 
  registry fetches the backend's `tools/list` and stores a
  **versioned schema snapshot**.
- Virtual tools also get stored, but require validation:  
- Static validation runs against this snapshot:
  - Projected required fields MUST have a default. Reject otherwise.
  - Default values MUST match the field's declared type.
  - Mapped input field names MUST reference fields that exist in the snapshot.
  - Output transform field paths MUST be syntactically valid.
- The tool is assigned a version (explicit or auto-incremented).

**Read/List tools**: Query by name, version, tags, server, environment.

**Update a tool**: New version created. Dependent agents are checked:
- If any `prod` agent's dependency range no longer matches, warn.

**Delete a tool version**: Lifecycle enforcement:
- If any `prod` agent depends on this exact version (or a range that only this
  version satisfies): **reject** with details (agent name, team contact).
- If only `stage` agents depend on it: **warn** with same details.
- If no dependents: succeed.

#### 4.1.2 Agent Registration

Agents are registered with an A2A AgentCard plus:
- **Versioned tool dependencies**: List of `(tool_name, version_range)` pairs.
  Version ranges follow SemVer (e.g., `fetch:1.2.*`, `search:>=2.0.0`).
- **Environment tag**: `stage`, `prod`, or other deployment-specific values.

#### 4.1.3 Schema Registration

Named, versioned JSON Schema definitions. Tools reference them via
`$ref: "#/schemas/Name"`. The registry resolves all `$ref` chains at
compilation time and rejects circular references.

#### 4.1.4 Server Registration

Backend MCP server declarations with version and tool provisions. Used for
SBOM tracking and lifecycle validation, not routing (routing is a data plane
concern configured separately).

#### 4.1.5 Lineage Queries

The registry MUST support:
- **Forward**: Given agent X, return all `(tool, version)` tuples it depends on.
- **Reverse**: Given tool Y version Z, return all agents that depend on it.
- **Filtered**: Constrain queries by environment tag.

#### 4.1.6 Environment Tags

Both tools and agents carry an `environment` field. The registry enforces
rules based on environment:
- `prod` entities have stricter lifecycle enforcement (see 4.1.1).
- Queries can filter by environment.
- The data plane can be configured to only load tools matching a given
  environment.

---

### 4.2 Input Customization

A virtual tool can customize the input schema that agents see, relative to the
backend tool's actual schema. Three operations, all validated at registration
time against the stored schema snapshot:

#### 4.2.1 Projection

Remove fields from the input schema. Only optional fields may be projected
away without a default. Removing a required field without providing a default
is a registration error.

#### 4.2.2 Defaults

Set field values deterministically. The agent never sees these fields (they
are projected away), and the values are injected at call time before forwarding
to the backend.

**Variable substitution sources (v1)**:
- `${ENV.VAR_NAME}` — environment variable on the data plane
- `${REQUEST.header.X-Foo}` — value from the incoming MCP request header
- `${FILE.filename.key}` — value from a side-loaded per-agent properties file

Substitution failures (missing env var, missing header, missing file) are
errors at call time, not registration time (since the values may only exist
at runtime).

#### 4.2.3 Field Mapping (Renaming)

Rename input fields to match the agent's domain language. The virtual tool
accepts the mapped name from the agent and translates it to the backend's
field name before forwarding.

Example: Agent sends `{story_id: "PROJ-123"}`, virtual tool maps to
`{issue_key: "PROJ-123"}` for the Jira backend.

Field mapping applies to the input schema: the agent sees the mapped names in
`tools/list`, and sends the mapped names in `tools/call`.

---

### 4.3 Output Transforms

When a backend tool returns a result, the virtual tool can transform it into
a different schema before returning it to the agent. This is required because:

1. Most MCP tools today don't declare `outputSchema` even when they return
   structured data (embedded as a string in the text content field).
2. Chaining tools in compositions requires structured, schema-conforming data
   to flow between steps.

#### 4.3.1 Core Model

An output transform is a **mapping from output field names to extraction
expressions**, where each expression pulls a value from the backend's response.

The simplest viable model (to be validated with tests against demo scenarios):

> Each output field maps to a **JSONPath expression** with an **optional
> default value**. If the source is an array, the mapping is applied
> **per-element**.

This collapses the following field source types (path, literal, coalesce, 
template, concat, nested, arrayMap) into a uniform concept. The spec should 
define the minimum set of extraction operations that can reproduce the 
functionality of the reference demos (a research-assistant multi-source 
search and an ecommerce product-search agent). If fewer types suffice, prefer 
fewer.

#### 4.3.2 Constant Injection

Output transforms can inject constant values (e.g., `"source": "arxiv"`).
This is a degenerate case of the mapping: a field with no extraction path and
only a default.

#### 4.3.3 Null Safety

If the output schema declares a field as required (non-null), and the
extraction expression could produce null (e.g., the backend omits the field),
the transform MUST have a default. This is validated statically at
registration time against the output schema.

#### 4.3.4 Output Schema and structuredContent

When a virtual tool has an `outputSchema`, the data plane populates the MCP
response's `structuredContent` field with the transformed, schema-conforming
result. The text content field carries the same data as a serialized string
(for backward compatibility with clients that don't read `structuredContent`).

#### 4.3.5 Validation

- **Static (at registration/compilation)**: Check that the transform can
  satisfy the output schema — flag fields that could produce null where the
  schema requires non-null.
- **Runtime (debug mode only)**: Validate actual output against the compiled
  JSON Schema validator. Log violations but don't fail the request.

---

### 4.4 Composition Patterns

Compositions orchestrate multiple tool calls from a single `tools/call`
invocation. The data plane executes the composition with zero LLM involvement.

#### 4.4.1 Pipeline

Sequential execution of steps. Each step can reference the output of previous
steps. Independent steps (no data dependency) SHOULD execute in parallel when
the dependency graph allows it — the implementation should treat the pipeline
as a DAG, not a linear chain.

Each step specifies:
- **id**: Unique identifier for data binding.
- **operation**: A tool call, or a nested composition pattern.
- **input binding**: Where this step gets its input. Options:
  - From the composition's original input (with JSONPath extraction).
  - From a previous step's output (by step ID + JSONPath).
  - A constant value.
  - A constructed object with fields drawn from multiple sources.

#### 4.4.2 Scatter-Gather

Fan out the same input to N targets in parallel, then aggregate results.

Each target is either a tool reference or a nested pattern (allowing
nested scatter-gather within a pipeline step, for example).

**Target optionality**: Individual targets may be marked as optional. If an
optional target fails, the composition continues. The aggregated result SHOULD
include an `errors` array describing which targets failed and why.

**Minimum success**: The composition as a whole fails if zero targets succeed.
At least one must return a result.

**Aggregation operations** (applied in sequence to the collected results):
- **Extract**: Pull a field from each result (JSONPath).
- **Flatten**: Flatten one level of nested arrays.
- **Dedupe**: Remove duplicate objects by a key field.
- **Sort**: Order by a field, ascending or descending.
- **Limit**: Take the first N items.
- **Wrap**: Wrap the result array in an object with a named field.
- **Merge**: Merge multiple objects into one (for object results).

#### 4.4.3 Extensibility

Each pattern carries a `type` field that defaults to `"stateless"` for
Pipeline and ScatterGather. This is a placeholder for future stateful patterns
(saga, retry with persistence, etc.) where a `persistence` or `stateStore`
configuration would be needed. The v1 spec defines only stateless execution.

The spec SHOULD be structured so that a future implementation could plug in a
workflow engine (e.g., Temporal, BPMN/Camunda) for stateful patterns without changing the
tool definition format.

#### 4.4.4 Streaming

Compositions SHOULD stream progress to the client as MCP content chunks during
execution. For example, a scatter-gather over 4 search backends should emit
progress text as each backend returns ("Searching arXiv... done. Searching
GitHub..."), with the final structured result delivered at completion.

This is important for UX: a 30-second silent block while a composition runs
is unacceptable.

---

### 4.5 Agent Identity and Tool Filtering

#### 4.5.1 Caller Identity

The data plane identifies the calling agent via (in priority order):
1. MCP `InitializeRequest` `clientInfo` (persisted in session).
2. HTTP headers (`X-Agent-Name`, `X-Agent-Version`).
3. JWT claims (`agent_name`, `agent_version`).

#### 4.5.2 Tool Filtering

When an agent calls `tools/list`, the returned tools are filtered:
1. If the agent is registered in the registry and has declared dependencies,
   only tools matching those dependencies are returned.
2. RBAC policies (CEL expressions) are evaluated per-tool.
3. The `unknownCallerPolicy` governs behavior for unidentified callers:
   - `allowAll` (default): all tools visible.
   - `denyAll`: no tools for unknown callers.
   - `allowUnregistered`: registered agents get filtered by SBOM; unknown
     callers get all tools.

#### 4.5.3 Tool Naming

Virtual tools are served as normal MCP tools. The MCP protocol has no concept
of "virtual" — this is an implementation detail of the data plane. The naming
convention for exposing virtual tools alongside backend tools is an
implementation choice.

One viable approach: the data plane acts as a "virtual" MCP server, and all
virtual tools are served under that server's namespace with their natural names.

---

### 4.6 MCP Protocol Integration

#### 4.6.1 Transparency

Virtual tools are invisible to the MCP protocol. Clients see normal tools in
`tools/list` and call them with normal `tools/call`. All composition,
transformation, and routing happens server-side.

#### 4.6.2 Output Schema and Structured Content

When a virtual tool declares an `outputSchema`:
- `tools/list` includes the schema in the tool definition.
- `tools/call` responses populate `structuredContent` with the transformed
  result.
- Text content carries the same data as a serialized string.

#### 4.6.3 Annotations (Future)

The spec should leave room for custom annotations on virtual tools, including:
- `outputDataClassification`: Labels on output data (e.g., "PII",
  "HighlyConfidential") for policy-based downstream handling.
- `inputDataClassification`: Labels on what a tool expects/disallows, enabling
  the data plane to block confidential data from flowing into tools that aren't
  cleared for it.

These are not v1 requirements but the annotation extension point must exist.

---

## 5. Non-Functional Requirements

### 5.1 Performance

- The data plane MUST add minimal latency beyond backend call time. The dominant
  costs are network I/O (backend calls) and LLM inference — gateway overhead
  for transform execution, schema validation, and aggregation should be
  negligible by comparison.
- Parallel execution in scatter-gather and DAG pipelines is critical — this is
  where the data plane saves the most agent tokens.
- A performance test suite MUST be part of the deliverable, measuring gateway
  overhead per-operation.

### 5.2 Streaming

Composition execution MUST support streaming partial results to the client.
Silent multi-second waits are a UX failure.

### 5.3 Scale

- The registry may contain thousands of tools across many teams.
- Any given data plane instance loads a subset: the tools and servers needed
  by the agents it serves.
- Data plane instances cache registry snapshots for minutes, not seconds. The
  registry is not in the hot path.

### 5.4 Deployment Independence

The virtual tools system MUST be independent of any agent SDK. It runs in the
data plane, which sits between agents and backend tools. Agents interact with
it via standard MCP — no special client library required.

If implemented within an existing data plane (e.g., agentgateway), the
implementation should be loosely coupled — ideally as a plugin/extension rather
than baked into the core. A standalone service is an alternative, but adds a
network hop for users already running a gateway.

---

## 6. Spec Deliverables

### 6.1 Registry API Specification
- OpenAPI definition for the CRUD + lineage query API.
- Protobuf definitions for the registry data model.

### 6.2 Composition Schema
- JSON Schema for the composition pattern definitions (pipeline, scatter-gather,
  aggregation ops, data bindings).
- A TypeScript validation library as the reference implementation — validates
  that a virtual tool definition is well-formed, dependencies exist, transforms
  match schemas, etc.

### 6.3 Reference Scenarios
The reference demos (a research-assistant multi-source search and an ecommerce
product-search agent, both in the agentgateway prototype repo) serve as the
product requirements. The spec MUST include reference scenarios derived from
these demos, expressed as test fixtures:

> Given this registry, these backend tool schemas, and this `tools/call`
> request — the expected response is X.

These scenarios are the acceptance tests for any implementation.

### 6.4 Visual Authoring Tool
A web-based WYSIWYG builder for virtual tools. Queries the registry for
existing tools (and their schemas), and provides autocomplete / drag-and-drop
for building compositions and transforms. Outputs valid registry JSON.

Uses the TypeScript validation library (6.2) for real-time feedback. This is
part of the reference implementation, not the spec itself, but the spec must
define the contracts it operates against.

---

## 7. Out of Scope (v1)

- **Stateful workflow patterns**: Saga, retry-with-persistence, circuit breaker.
  The pattern format has an extensibility hook (`type: "stateless"`) but v1
  only implements stateless execution.
- **Async / task-based compositions**: Where `tools/call` returns a task ID and
  results come later. This is better suited for A2A than MCP, and introduces
  persistence concerns.
- **Data classification enforcement**: The annotation extension point exists,
  but enforcement logic is future work.
- **MCP resources and prompts**: Virtual tools are strictly about MCP tools.
  Prompts and resources may feed into virtual tool configuration (e.g., as
  sources for defaults or templates) but are not themselves "virtualized."
- **Workflow engine integration**: The spec should be structured so a BPMN
  engine like Camunda could implement the composition patterns, but building
  that integration is not v1 work.

---

## 8. Open Questions

1. **Output transform simplification**: Can the 7 field source types in the
   agentgateway prototype (path, literal, coalesce, template, concat, nested,
   arrayMap) be collapsed into a simpler model ("field → JSONPath + default,
   applied per-element for arrays")? Needs validation via unit tests against
   demo scenarios with mocked backend tools returning varied optional field
   combinations.

2. **Tool naming convention**: The `virtual_` prefix is a known hack. A cleaner
   approach (e.g., virtual MCP server namespace) needs design work. The spec
   should define the contract for tool naming without coupling to a specific
   prefix.

3. **Plugin architecture**: If building within an existing data plane (e.g.,
   agentgateway), what does a generic plugin mechanism look like? This affects
   whether virtual tools live as a plugin or a standalone service.

4. **Streaming protocol**: How does composition progress map to MCP's streaming
   response format? Needs protocol-level design (likely SSE content chunks with
   progress text followed by final structured result).

---

## Appendix A: Existing Implementation References

A working prototype exists in the agentgateway fork at
github.com/agentgateway/agentgateway (branch: feature/virtual-tools).
Key locations within that codebase:

| Component | Location | Notes |
|-----------|----------|-------|
| Registry types (Rust) | `crates/agentgateway/src/mcp/registry/types.rs` | Canonical type definitions |
| Compilation | `crates/agentgateway/src/mcp/registry/compiled.rs` | 4-pass compiler with static analysis |
| Pattern execution | `crates/agentgateway/src/mcp/registry/executor/` | Pipeline, scatter-gather, schema-map |
| Tool resolution | `crates/agentgateway/src/mcp/handler.rs` | MCP request → backend routing |
| TypeScript DSL | `packages/vmcp-dsl/` | Builder API + CLI compiler |
| TypeScript types | `packages/registry-dsl/` | Registry type definitions |
| Visual builder | `packages/vmcp-dsl/tool-builder/` | Standalone HTML drag-drop UI |
| Research demo | `examples/research-assistant-demo/` | Working end-to-end with 10 virtual tools |
| Ecommerce demo | `examples/ecommerce-demo/` | Partially working, needs testing |

## Appendix B: Demo-Derived Reference Scenarios

These scenarios should be formalized as test fixtures in the spec.

### B.1 Schema Normalization (Source Tool + Output Transform)

**Given**: A backend `arxiv_search` tool returning:
```json
{"papers": [{"title": "T", "pdf_url": "U", "abstract": "A", "arxiv_id": "123"}]}
```

**And**: A virtual tool `normalized_arxiv` with output transform mapping
`papers[*]` to `results[*]` with fields `title`, `url` (from pdf_url), `snippet`
(from abstract), `source` (constant "arxiv").

**When**: Agent calls `normalized_arxiv` with `{query: "ColBERT"}`.

**Then**: Response contains:
```json
{"results": [{"title": "T", "url": "U", "snippet": "A", "source": "arxiv"}]}
```

### B.2 Scatter-Gather with Partial Failure

**Given**: 4 normalized search tools (exa, arxiv, github, huggingface).
GitHub target is optional and returns an error.

**When**: Agent calls `multi_source_search` with `{query: "transformers"}`.

**Then**: Response contains results from 3 successful sources, flattened and
deduped. An `errors` array describes the GitHub failure.

### B.3 Pipeline with Cross-Service Data Flow

**Given**: A pipeline `store_research_finding` with 3 steps:
1. `create_entity` on entity-service → returns `{entity: {id: "E1"}}`.
2. `register_content` on tag-service → receives entity ID from step 1.
3. `tag_content` on tag-service → receives content ID from step 2.

**When**: Agent calls `store_research_finding` with title, description, URL, tags.

**Then**: All 3 backend calls execute in sequence. Final result contains IDs
from all steps.

### B.4 Input Projection + Defaults + Field Mapping

**Given**: A backend tool with schema
`{query: string (required), api_key: string (required), region: string, format: string}`.

**And**: A virtual tool that:
- Projects away `region` and `format` (optional, no default needed).
- Sets default `api_key` = `${ENV.SEARCH_API_KEY}`.
- Maps `search_term` → `query`.

**When**: Agent calls with `{search_term: "ColBERT"}`.

**Then**: Backend receives `{query: "ColBERT", api_key: "<resolved>"}`.

### B.5 Lineage Query

**Given**: Registry contains:
- Tool `search:1.0.0`, `search:2.0.0`
- Agent `research-agent` (prod) depends on `search:1.*`
- Agent `test-agent` (stage) depends on `search:2.0.0`

**When**: Query "which agents depend on `search:1.0.0`?"

**Then**: Returns `research-agent` (prod).

**When**: Attempt to DELETE `search:1.0.0`.

**Then**: Rejected — `research-agent` (prod) depends on `search:1.*` and no
other `1.x` version exists.

**When**: Register `search:1.1.0`, then DELETE `search:1.0.0`.

**Then**: Succeeds — `research-agent`'s `search:1.*` dependency is satisfied
by `1.1.0`.
