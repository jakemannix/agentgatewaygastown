# Testing Output Transformation in AgentGateway

This document describes how to test the virtual tool output transformation functionality directly in the agentgateway repo without external dependencies.

## Overview

Output transformation allows virtual tools to:
1. Extract JSON from text responses
2. Apply JSONPath projections to reshape output
3. Rename/filter output fields

## Test Setup

### 1. Create a test registry file

Create `test-registry.json` in the repo root:

```json
{
  "schemaVersion": "1.0",
  "servers": [
    {
      "name": "time-server",
      "stdio": {
        "command": "uvx",
        "args": ["mcp-server-time"]
      }
    }
  ],
  "tools": [
    {
      "name": "get_current_time",
      "server": "time-server",
      "description": "Get current time in a timezone"
    },
    {
      "name": "get_time_simple",
      "source": "get_current_time",
      "description": "Get just the datetime string",
      "outputSchema": {
        "type": "object",
        "properties": {
          "time": {
            "type": "string",
            "source_field": "$.datetime"
          },
          "day": {
            "type": "string",
            "source_field": "$.day_of_week"
          }
        }
      }
    }
  ]
}
```

### 2. Create a test config file

Create `test-config.yaml` in the repo root:

```yaml
binds:
- port: 3000
  listeners:
  - routes:
    - policies:
        cors:
          allowOrigins:
          - "*"
          allowHeaders:
          - mcp-protocol-version
          - content-type
          - cache-control
          - mcp-session-id
      backends:
      - mcp:
          targets:
          - name: time-server
            stdio:
              cmd: uvx
              args: ["mcp-server-time"]

registry:
  source: file://./test-registry.json
  refreshInterval: 30s
```

### 3. Build and run agentgateway

```bash
cargo build --release
./target/release/agentgateway -f test-config.yaml
```

You should see logs indicating:
- Registry loaded from `test-registry.json`
- MCP target `time-server` being added
- Listening on port 3000

## Testing with curl

### Test 1: Direct tool call (no transformation)

Call the base tool `time-server/get_current_time`:

```bash
# Initialize session
INIT_RESP=$(curl -s -X POST "http://localhost:3000/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}')

echo "Init response: $INIT_RESP"

# Extract session ID from response headers (will be in the SSE data)
SESSION_ID=$(echo "$INIT_RESP" | grep -o 'mcp-session-id: [^"]*' | head -1 | cut -d' ' -f2)

# If that doesn't work, make another request to get session
curl -s -X POST "http://localhost:3000/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: test-session" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"time-server/get_current_time","arguments":{"timezone":"UTC"}}}'
```

Expected output (raw, no transformation):
```json
{
  "timezone": "UTC",
  "datetime": "2026-01-10T14:30:00+00:00",
  "day_of_week": "Saturday",
  "is_dst": false
}
```

### Test 2: Virtual tool with output transformation

Call the virtual tool `get_time_simple`:

```bash
curl -s -X POST "http://localhost:3000/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}'

# Then call the virtual tool
curl -s -X POST "http://localhost:3000/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: test-session" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_time_simple","arguments":{"timezone":"America/New_York"}}}'
```

Expected output (transformed):
```json
{
  "time": "2026-01-10T09:30:00-05:00",
  "day": "Saturday"
}
```

## Testing with the Rust test suite

Add this test to `crates/agentgateway/src/mcp/registry/compiled.rs`:

```rust
#[cfg(test)]
mod output_transform_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_output_transformation() {
        let registry_json = json!({
            "schemaVersion": "1.0",
            "servers": [
                {"name": "test-server", "url": "http://test", "transport": "streamablehttp"}
            ],
            "tools": [
                {
                    "name": "base_tool",
                    "server": "test-server"
                },
                {
                    "name": "virtual_tool",
                    "source": "base_tool",
                    "outputSchema": {
                        "type": "object",
                        "properties": {
                            "extracted": {
                                "type": "string",
                                "source_field": "$.nested.value"
                            }
                        }
                    }
                }
            ]
        });

        let registry: Registry = serde_json::from_value(registry_json).unwrap();
        let compiled = CompiledRegistry::compile(registry).unwrap();

        let input = json!({
            "nested": {
                "value": "hello world"
            },
            "other": "ignored"
        });

        let result = compiled.transform_output("virtual_tool", input).unwrap();

        assert_eq!(result, json!({
            "extracted": "hello world"
        }));
    }
}
```

Run the test:
```bash
cargo test output_transform_tests --release
```

## Automated Integration Test Script

Save this as `test-output-transform.sh`:

```bash
#!/bin/bash
set -e

echo "=== Output Transformation Integration Test ==="

# Build
echo "Building agentgateway..."
cargo build --release

# Create test files
cat > /tmp/test-registry.json << 'EOF'
{
  "schemaVersion": "1.0",
  "servers": [
    {"name": "time-server", "stdio": {"command": "uvx", "args": ["mcp-server-time"]}}
  ],
  "tools": [
    {"name": "get_current_time", "server": "time-server"},
    {
      "name": "get_time_simple",
      "source": "get_current_time",
      "outputSchema": {
        "type": "object",
        "properties": {
          "time": {"type": "string", "source_field": "$.datetime"},
          "day": {"type": "string", "source_field": "$.day_of_week"}
        }
      }
    }
  ]
}
EOF

cat > /tmp/test-config.yaml << 'EOF'
binds:
- port: 3000
  listeners:
  - routes:
    - backends:
      - mcp:
          targets:
          - name: time-server
            stdio:
              cmd: uvx
              args: ["mcp-server-time"]
registry:
  source: file:///tmp/test-registry.json
  refreshInterval: 30s
EOF

# Start server in background
echo "Starting agentgateway..."
./target/release/agentgateway -f /tmp/test-config.yaml &
AG_PID=$!
sleep 3

cleanup() {
    echo "Cleaning up..."
    kill $AG_PID 2>/dev/null || true
}
trap cleanup EXIT

# Test virtual tool
echo "Testing virtual tool get_time_simple..."
RESULT=$(curl -s -X POST "http://localhost:3000/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' && \
curl -s -X POST "http://localhost:3000/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_time_simple","arguments":{"timezone":"UTC"}}}')

echo "Result: $RESULT"

# Check if transformation was applied
if echo "$RESULT" | grep -q '"time"'; then
    echo "SUCCESS: Output transformation working!"
else
    echo "FAILURE: Output transformation not applied"
    exit 1
fi

echo "=== Test Complete ==="
```

Make it executable and run:
```bash
chmod +x test-output-transform.sh
./test-output-transform.sh
```

## Verifying the Implementation

The key files involved in output transformation:

1. **`src/mcp/registry/compiled.rs`**: `CompiledVirtualTool::transform_output()` - applies JSONPath projections
2. **`src/mcp/handler.rs`**: `transform_server_message()` and `transform_call_tool_result()` - intercepts responses
3. **`src/mcp/session.rs`**: `CallToolRequest` handler uses `send_single_with_output_transform()`

To verify the wiring:
```bash
# Check that transform methods are being used (no "unused" warnings)
cargo build --release 2>&1 | grep -i "transform"
```

## Troubleshooting

### "Expecting value" JSON parse error
The response is in SSE format (`data: {...}`). The client needs to strip the `data: ` prefix.

### Output not transformed
1. Check that the virtual tool has `outputSchema` with `source_field` properties
2. Check that the response content is valid JSON (not plain text)
3. Check agentgateway logs for transformation errors

### Virtual tool not found
Ensure the registry is being loaded:
```bash
curl http://localhost:3000/registry
```
