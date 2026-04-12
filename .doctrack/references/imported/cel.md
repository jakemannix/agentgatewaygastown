---
type: reference
original_path: architecture/cel.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# CEL (Common Expression Language) in AgentGateway (imported)

Source: `architecture/cel.md`

## Overview
AgentGateway extensively uses CEL for runtime policy evaluation. CEL is an expression language that evaluates user-defined expressions based on incoming requests.

Example: `jwt.sub == "test-user" && mcp.tool.name == "add"`

CEL is chosen over Lua or WASM for its speed and sufficiency for most use cases.

## Current Uses
- Defining attributes for logs/traces (e.g., `user_agent: 'request.headers["user-agent"]'`)
- Modifying HTTP headers and bodies
- Authorization policies
- Rate limiting key selection

## Architecture: ContextBuilder

### Parse-Time Analysis
During configuration parsing (not per-request), agentgateway extracts which variables each CEL expression references. This determines what data to capture at runtime.

### Runtime Efficiency ("Pay for what you use")
`ContextBuilder.with_xxx()` conditionally adds fields to the CEL context only if an expression needs them. Critical for expensive fields like `request.body` (requires storing a copy) and useful for `headers` which can also be costly.

### Variable Availability
CEL expressions run at different pipeline stages. Some variables aren't available yet (e.g., `response` during request header transformation) and some are no longer available (e.g., `request` during logging). AgentGateway dynamically retains data based on whether any expression references it.

### Limitations
Handles most cases but has false negatives: `request.body` works but `request["body"]` does not (dynamic key access not detected during parse-time analysis).

## Schema Documentation
- Variable schema: auto-generated to `schema/cel.json` and `schema/cel.md`
- Function documentation: manually maintained in `schema/cel-functions.md`

## Related Notes
- [[references/imported/configuration|Configuration Architecture]] — where CEL policies are configured
- [[references/imported/virtual-tools|Virtual Tools]] — CEL used in filter predicates and routing
