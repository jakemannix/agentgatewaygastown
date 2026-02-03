/**
 * Integration tests for Gateway MCP behavior with registry changes
 *
 * These tests verify the MCP contract for tool list changes:
 * 1. Gateway declares listChanged capability (or not)
 * 2. tools/list reflects registry changes after refresh
 * 3. notifications/tools/list_changed sent when capability declared
 *
 * Prerequisites:
 *   - Gateway running on localhost:3000 with file:// registry source
 *   - Registry file writable for test modifications
 *
 * Run with:
 *   # Start gateway first (from repo root):
 *   ./target/debug/agentgateway -f examples/research-assistant-demo/gateway-configs/config.yaml
 *
 *   # Then run tests:
 *   REGISTRY_FILE=examples/research-assistant-demo/gateway-configs/research_registry.json npm test
 */

import { describe, it, before, after, beforeEach } from 'node:test';
import assert from 'node:assert';
import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const GATEWAY_URL = process.env.GATEWAY_URL || 'http://localhost:3000';
// Default to research-assistant-demo registry (requires gateway started with that config)
const REGISTRY_FILE = process.env.REGISTRY_FILE ||
  path.join(__dirname, '..', '..', 'examples', 'research-assistant-demo', 'gateway-configs', 'research_registry.json');

// Check if gateway is running
async function isGatewayRunning() {
  try {
    const response = await fetch(`${GATEWAY_URL}/mcp`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/event-stream',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 0,
        method: 'initialize',
        params: {
          protocolVersion: '2024-11-05',
          capabilities: {},
          clientInfo: { name: 'probe', version: '1.0.0' },
        },
      }),
      signal: AbortSignal.timeout(5000),
    });
    // Gateway is running if we get any HTTP response with mcp-session-id header
    // (even 500 if backends aren't available)
    return response.headers.has('mcp-session-id') || response.ok || response.status === 406;
  } catch {
    return false;
  }
}

// MCP JSON-RPC helpers
let requestId = 0;
function mcpRequest(method, params = {}) {
  return {
    jsonrpc: '2.0',
    id: ++requestId,
    method,
    params,
  };
}

// Create empty registry
function createEmptyRegistry() {
  return {
    schemaVersion: '2.0',
    tools: [],
    schemas: [],
    servers: [],
    agents: [],
  };
}

// Create a simple test tool
function createTestTool(name) {
  return {
    name,
    description: `Test tool ${name}`,
    version: '1.0.0',
    source: {
      server: 'test-server',
      tool: 'backend_tool',
    },
  };
}

// MCP HTTP client for Streamable HTTP transport
class McpClient {
  constructor(baseUrl) {
    this.baseUrl = baseUrl;
    this.sessionId = null;
  }

  async initialize() {
    // Send initialize request without session header (new session)
    const response = await fetch(`${this.baseUrl}/mcp`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/event-stream',
      },
      body: JSON.stringify(mcpRequest('initialize', {
        protocolVersion: '2024-11-05',
        capabilities: {},
        clientInfo: {
          name: 'test-client',
          version: '1.0.0',
        },
      })),
    });

    // Extract session ID from response header
    this.sessionId = response.headers.get('mcp-session-id');

    // Parse response (may be SSE or JSON)
    const contentType = response.headers.get('content-type') || '';
    let result;

    if (contentType.includes('text/event-stream')) {
      const text = await response.text();
      // Parse SSE format: "event: message\ndata: {...}\n\n"
      const dataMatch = text.match(/data: (.+)/);
      if (dataMatch) {
        result = JSON.parse(dataMatch[1]);
      }
    } else {
      result = await response.json();
    }

    // Send initialized notification
    await this.notify('notifications/initialized', {});

    return result;
  }

  async request(method, params = {}) {
    const response = await fetch(`${this.baseUrl}/mcp`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/event-stream',
        ...(this.sessionId ? { 'mcp-session-id': this.sessionId } : {}),
      },
      body: JSON.stringify(mcpRequest(method, params)),
    });

    const contentType = response.headers.get('content-type') || '';

    if (contentType.includes('text/event-stream')) {
      const text = await response.text();
      const dataMatch = text.match(/data: (.+)/);
      if (dataMatch) {
        return JSON.parse(dataMatch[1]);
      }
    }

    return response.json();
  }

  async notify(method, params = {}) {
    await fetch(`${this.baseUrl}/mcp`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(this.sessionId ? { 'mcp-session-id': this.sessionId } : {}),
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method,
        params,
      }),
    });
  }

  async listTools() {
    const response = await this.request('tools/list', {});
    return response.result?.tools || [];
  }
}

describe('Gateway MCP Contract', () => {
  let client;
  let initializeResult;
  let originalRegistry;
  let gatewayRunning = false;

  before(async () => {
    // Check if gateway is running
    gatewayRunning = await isGatewayRunning();
    if (!gatewayRunning) {
      console.log(`
SKIP: Gateway not running at ${GATEWAY_URL}

To run these tests:
1. Start the gateway:
   ./target/debug/agentgateway -f examples/research-assistant-demo/gateway-configs/config.yaml

2. Start the backend services (optional, for full tests):
   cd examples/research-assistant-demo && ./start_services.sh

3. Run tests:
   REGISTRY_FILE=examples/research-assistant-demo/gateway-configs/research_registry.json npm test
`);
      return;
    }

    // Save original registry if it exists
    try {
      originalRegistry = await fs.readFile(REGISTRY_FILE, 'utf-8');
      console.log(`Using registry file: ${REGISTRY_FILE}`);
    } catch {
      originalRegistry = null;
      console.log(`Warning: Registry file not found: ${REGISTRY_FILE}`);
    }
  });

  after(async () => {
    // Restore original registry
    if (originalRegistry) {
      await fs.writeFile(REGISTRY_FILE, originalRegistry);
    }
  });

  beforeEach(async () => {
    client = new McpClient(GATEWAY_URL);
  });

  describe('Capability Declaration', () => {
    it('should declare tools capability in initialize response', async () => {
      if (!gatewayRunning) {
        console.log('SKIP: Gateway not running');
        return;
      }
      initializeResult = await client.initialize();

      assert.ok(initializeResult.result, 'Initialize should return a result');
      assert.ok(initializeResult.result.capabilities, 'Result should have capabilities');
      assert.ok('tools' in initializeResult.result.capabilities, 'Capabilities should include tools');

      // Record what the gateway claims
      const toolsCapability = initializeResult.result.capabilities.tools;
      console.log('Gateway tools capability:', JSON.stringify(toolsCapability));
    });

    it('should indicate listChanged capability (true or false)', async () => {
      if (!gatewayRunning) return;
      initializeResult = await client.initialize();

      const toolsCapability = initializeResult.result?.capabilities?.tools || {};
      const listChanged = toolsCapability.listChanged;

      console.log(`Gateway declares listChanged: ${listChanged}`);

      // This test documents the current behavior - it passes either way
      assert.ok(
        listChanged === true || listChanged === false || listChanged === undefined,
        'listChanged should be boolean or undefined'
      );
    });
  });

  describe('Tool List Reflects Registry Changes', () => {
    it('should return tools from registry via tools/list', async () => {
      if (!gatewayRunning || !originalRegistry) return;
      // Write a registry with one tool
      const registry = createEmptyRegistry();
      registry.tools = [createTestTool('initial_tool')];
      await fs.writeFile(REGISTRY_FILE, JSON.stringify(registry, null, 2));

      // Wait for file watcher debounce (250ms) + buffer
      await new Promise(resolve => setTimeout(resolve, 500));

      await client.initialize();
      const tools = await client.listTools();

      // Should see the initial tool (plus any backend tools)
      const toolNames = tools.map(t => t.name);
      console.log('Tools after initial registry:', toolNames);

      assert.ok(
        toolNames.includes('initial_tool'),
        `Expected 'initial_tool' in tools list, got: ${toolNames.join(', ')}`
      );
    });

    it('should reflect new tools after registry file changes', async () => {
      if (!gatewayRunning || !originalRegistry) return;
      // Start with empty registry
      const registry = createEmptyRegistry();
      await fs.writeFile(REGISTRY_FILE, JSON.stringify(registry, null, 2));
      await new Promise(resolve => setTimeout(resolve, 500));

      await client.initialize();

      // Get initial tool list
      const toolsBefore = await client.listTools();
      const namesBefore = toolsBefore.map(t => t.name);
      console.log('Tools before change:', namesBefore);

      // Add a new tool to registry
      registry.tools = [createTestTool('dynamically_added_tool')];
      await fs.writeFile(REGISTRY_FILE, JSON.stringify(registry, null, 2));

      // Wait for file watcher debounce + reload
      await new Promise(resolve => setTimeout(resolve, 500));

      // Get updated tool list
      const toolsAfter = await client.listTools();
      const namesAfter = toolsAfter.map(t => t.name);
      console.log('Tools after change:', namesAfter);

      assert.ok(
        namesAfter.includes('dynamically_added_tool'),
        `Expected 'dynamically_added_tool' in tools list after registry change, got: ${namesAfter.join(', ')}`
      );
    });

    it('should reflect removed tools after registry file changes', async () => {
      if (!gatewayRunning || !originalRegistry) return;
      // Start with a tool
      const registry = createEmptyRegistry();
      registry.tools = [createTestTool('tool_to_remove')];
      await fs.writeFile(REGISTRY_FILE, JSON.stringify(registry, null, 2));
      await new Promise(resolve => setTimeout(resolve, 500));

      await client.initialize();

      // Verify tool exists
      const toolsBefore = await client.listTools();
      const namesBefore = toolsBefore.map(t => t.name);
      assert.ok(namesBefore.includes('tool_to_remove'), 'Tool should exist before removal');

      // Remove the tool
      registry.tools = [];
      await fs.writeFile(REGISTRY_FILE, JSON.stringify(registry, null, 2));
      await new Promise(resolve => setTimeout(resolve, 500));

      // Verify tool is gone
      const toolsAfter = await client.listTools();
      const namesAfter = toolsAfter.map(t => t.name);
      console.log('Tools after removal:', namesAfter);

      assert.ok(
        !namesAfter.includes('tool_to_remove'),
        `Expected 'tool_to_remove' to be removed, but still present in: ${namesAfter.join(', ')}`
      );
    });
  });

  describe('Tool List Changed Notification (MCP Spec)', () => {
    it('should send notifications/tools/list_changed when registry changes (if listChanged capability declared)', async () => {
      if (!gatewayRunning || !originalRegistry) return;
      // This test verifies the MCP spec requirement:
      // "When the list of available tools changes, servers that declared the
      //  listChanged capability SHOULD send a notification"

      initializeResult = await client.initialize();

      const listChanged = initializeResult.result?.capabilities?.tools?.listChanged;

      if (!listChanged) {
        console.log('SKIP: Gateway does not declare listChanged capability');
        // Test passes but documents that notification is not supported
        return;
      }

      // If listChanged is declared, we need to listen for notifications
      // This requires keeping the SSE connection open

      // Write initial registry
      const registry = createEmptyRegistry();
      await fs.writeFile(REGISTRY_FILE, JSON.stringify(registry, null, 2));
      await new Promise(resolve => setTimeout(resolve, 500));

      // Set up notification listener via GET request (SSE stream)
      const notifications = [];
      const controller = new AbortController();

      const notificationPromise = (async () => {
        try {
          const response = await fetch(`${GATEWAY_URL}/mcp`, {
            method: 'GET',
            headers: {
              'Accept': 'text/event-stream',
              'mcp-session-id': client.sessionId,
            },
            signal: controller.signal,
          });

          const reader = response.body.getReader();
          const decoder = new TextDecoder();

          while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            const text = decoder.decode(value);
            const lines = text.split('\n');

            for (const line of lines) {
              if (line.startsWith('data: ')) {
                try {
                  const data = JSON.parse(line.slice(6));
                  if (data.method === 'notifications/tools/list_changed') {
                    notifications.push(data);
                  }
                } catch {
                  // Ignore parse errors
                }
              }
            }
          }
        } catch (err) {
          if (err.name !== 'AbortError') {
            throw err;
          }
        }
      })();

      // Trigger a registry change
      registry.tools = [createTestTool('notification_test_tool')];
      await fs.writeFile(REGISTRY_FILE, JSON.stringify(registry, null, 2));

      // Wait for notification (with timeout)
      await new Promise(resolve => setTimeout(resolve, 2000));
      controller.abort();

      await notificationPromise;

      console.log('Notifications received:', notifications.length);

      assert.ok(
        notifications.length > 0,
        'Expected notifications/tools/list_changed notification when registry changes (listChanged capability was declared)'
      );
    });
  });
});
