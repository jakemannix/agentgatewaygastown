# Design: MCP `notifications/tools/list_changed` Support

## Summary

Implement support for the MCP `notifications/tools/list_changed` notification, which informs connected clients when the tool list changes due to registry updates.

## MCP Specification Requirements

From [MCP Spec - Tools](https://modelcontextprotocol.io/specification/2025-03-26/server/tools):

### Capability Declaration

Servers that support tool list change notifications MUST declare it:

```json
{
  "capabilities": {
    "tools": {
      "listChanged": true
    }
  }
}
```

### Notification Format

When the list of available tools changes, servers that declared `listChanged: true` **SHOULD** send:

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/tools/list_changed"
}
```

### Expected Client Behavior

After receiving the notification, clients should call `tools/list` to get the updated tool list.

## Current State

| Component | Status |
|-----------|--------|
| `listChanged` capability declaration | **Not implemented** - returns `undefined` |
| Registry file watcher | **Implemented** - detects file changes |
| Registry HTTP polling | **Implemented** - polls at `refreshInterval` |
| Internal registry update | **Implemented** - `ArcSwap` updates atomically |
| Session tracking | **Partial** - sessions exist but not tracked for notifications |
| SSE notification channel | **Not implemented** |

### Current Initialize Response

```json
{
  "capabilities": {
    "tools": {}
  }
}
```

### Desired Initialize Response

```json
{
  "capabilities": {
    "tools": {
      "listChanged": true
    }
  }
}
```

## Architecture

### Components Involved

```
┌─────────────────┐     file change      ┌──────────────────┐
│  Registry File  │ ──────────────────▶  │  RegistryStore   │
└─────────────────┘                      │  (file watcher)  │
                                         └────────┬─────────┘
                                                  │
                                                  │ update()
                                                  ▼
┌─────────────────┐     poll interval    ┌──────────────────┐
│ Registry HTTP   │ ──────────────────▶  │  RegistryStore   │
│ Source          │                      │  (ArcSwap)       │
└─────────────────┘                      └────────┬─────────┘
                                                  │
                                                  │ notify subscribers
                                                  ▼
                                         ┌──────────────────┐
                                         │ SessionRegistry  │
                                         │ (new component)  │
                                         └────────┬─────────┘
                                                  │
                                                  │ broadcast to all
                                                  ▼
                                         ┌──────────────────┐
                                         │  MCP Sessions    │
                                         │  (SSE channels)  │
                                         └──────────────────┘
```

### New Components

#### 1. SessionRegistry

Tracks active MCP sessions that need notifications:

```rust
pub struct SessionRegistry {
    // Map of session_id -> notification sender
    sessions: DashMap<String, mpsc::Sender<ServerNotification>>,
}

impl SessionRegistry {
    pub fn register(&self, session_id: String, tx: mpsc::Sender<ServerNotification>);
    pub fn unregister(&self, session_id: &str);
    pub async fn broadcast(&self, notification: ServerNotification);
}
```

#### 2. RegistryStore Changes

Add notification callback when registry updates:

```rust
impl RegistryStore {
    pub fn update(&self, registry: Registry) -> Result<(), RegistryError> {
        let compiled = CompiledRegistry::compile(registry)?;
        self.current.store(Arc::new(Some(Arc::new(compiled))));

        // NEW: Notify subscribers
        if let Some(ref notifier) = self.change_notifier {
            notifier.notify_tools_changed();
        }

        Ok(())
    }
}
```

#### 3. Capability Declaration

In `mcp/session.rs` or handler initialization:

```rust
ServerCapabilities {
    tools: Some(ToolsCapability {
        list_changed: Some(true),  // NEW
    }),
    // ...
}
```

## TDD Test Plan

Tests are already written in `registry-service/test/gateway-mcp.test.js`.

### Test 1: Capability Declaration (currently failing)

```javascript
it('should declare tools capability in initialize response', async () => {
  const result = await client.initialize();
  assert.ok(result.result.capabilities.tools, 'Should have tools capability');
});

it('should indicate listChanged capability', async () => {
  const result = await client.initialize();
  const listChanged = result.result?.capabilities?.tools?.listChanged;
  // Currently: undefined
  // Expected: true
  assert.strictEqual(listChanged, true, 'Should declare listChanged: true');
});
```

**To make pass:**
- Add `listChanged: true` to tools capability in initialize response

### Test 2: Tools List Reflects Changes (currently failing)

```javascript
it('should reflect new tools after registry file changes', async () => {
  await client.initialize();
  const toolsBefore = await client.listTools();

  // Add tool to registry file
  registry.tools = [createTestTool('new_tool')];
  await fs.writeFile(REGISTRY_FILE, JSON.stringify(registry));
  await sleep(500); // Wait for file watcher

  const toolsAfter = await client.listTools();
  assert.ok(toolsAfter.some(t => t.name === 'new_tool'));
});
```

**Current failure:** Returns empty tools list (backends required)

**To make pass:**
- Allow `tools/list` to return virtual tools even when backends unavailable
- OR ensure test runs with backends available

### Test 3: Notification Sent (currently skipped)

```javascript
it('should send notifications/tools/list_changed', async () => {
  const result = await client.initialize();

  // Skip if not declared
  if (!result.result?.capabilities?.tools?.listChanged) {
    return; // Currently skips here
  }

  // Listen for notifications via SSE
  const notifications = [];
  const sseStream = await client.openNotificationStream();
  sseStream.on('notification', n => notifications.push(n));

  // Trigger registry change
  await fs.writeFile(REGISTRY_FILE, newRegistry);
  await sleep(1000);

  assert.ok(
    notifications.some(n => n.method === 'notifications/tools/list_changed'),
    'Should receive tools/list_changed notification'
  );
});
```

**To make pass:**
1. Declare `listChanged: true` (enables this test)
2. Track sessions with open SSE connections
3. Broadcast notification when registry updates

## Implementation Steps

### Phase 1: Declare Capability

1. Modify `ServerCapabilities` to include `listChanged: true`
2. Run tests - Test 1 should pass

### Phase 2: Fix tools/list Without Backends

1. Modify `tools/list` handler to return virtual tools even if backends unavailable
2. Run tests - Test 2 should pass

### Phase 3: Implement Notifications

1. Create `SessionRegistry` to track active sessions
2. Register sessions when SSE connection established
3. Unregister sessions when connection closed
4. Hook `RegistryStore::update()` to broadcast notifications
5. Run tests - Test 3 should pass

## Configuration

No new configuration required. The feature is automatically enabled when:
- Registry is configured with `file://` or `http://` source
- `refreshInterval` is set (for HTTP sources)

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Memory leak from orphaned sessions | Heartbeat/timeout to clean up stale sessions |
| Notification storm on rapid changes | Debounce notifications (max 1 per second) |
| Client doesn't support notifications | Graceful - client simply won't receive them |

## Testing Commands

```bash
# Build gateway
cargo build -p agentgateway-app

# Start gateway with test config
./target/debug/agentgateway -f registry-service/test/test-gateway-config.yaml

# Run MCP contract tests
cd registry-service
REGISTRY_FILE=data/test-registry.json npm test
```

## Success Criteria

All tests in `registry-service/test/gateway-mcp.test.js` pass:

- [ ] `should declare tools capability in initialize response`
- [ ] `should indicate listChanged capability (true)`
- [ ] `should return tools from registry via tools/list`
- [ ] `should reflect new tools after registry file changes`
- [ ] `should reflect removed tools after registry file changes`
- [ ] `should send notifications/tools/list_changed when registry changes`

## References

- [MCP Specification - Tools](https://modelcontextprotocol.io/specification/2025-03-26/server/tools)
- [MCP Specification - Notifications](https://modelcontextprotocol.io/specification/2025-03-26/basic/notifications)
- Test file: `registry-service/test/gateway-mcp.test.js`
- Registry store: `crates/agentgateway/src/mcp/registry/store.rs`
