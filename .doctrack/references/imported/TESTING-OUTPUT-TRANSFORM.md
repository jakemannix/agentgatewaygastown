---
type: reference
original_path: docs/TESTING-OUTPUT-TRANSFORM.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Testing Output Transformation (imported)

Source: `docs/TESTING-OUTPUT-TRANSFORM.md`

Practical guide for testing virtual tool output transformation in agentgateway.

## What Output Transformation Does
1. Extract JSON from text responses
2. Apply JSONPath projections to reshape output
3. Rename/filter output fields

## Testing Approach

### Registry Setup
Define a base tool and a virtual tool with `outputSchema` containing `source_field` (JSONPath) properties. Example: wrapping `mcp-server-time`'s `get_current_time` into `get_time_simple` that extracts only `$.datetime` and `$.day_of_week`.

### Test Methods
- **curl**: Initialize MCP session, call virtual tool, verify transformed output
- **Rust unit tests**: Test `CompiledRegistry::transform_output()` directly with mock data
- **Integration script**: Automated bash script that builds, starts gateway, runs curl tests

## Key Files Involved
- `src/mcp/registry/compiled.rs` — `CompiledVirtualTool::transform_output()`
- `src/mcp/handler.rs` — `transform_server_message()` and `transform_call_tool_result()`
- `src/mcp/session.rs` — `send_single_with_output_transform()`

## Troubleshooting
- SSE format requires stripping `data: ` prefix from responses
- Virtual tool must have `outputSchema` with `source_field` properties
- Response content must be valid JSON (not plain text)

## Related Notes
- [[references/imported/virtual-tools|Virtual Tools]] — the feature being tested
- [[references/imported/composition-test-plan|Composition Test Plan]] — broader test strategy
