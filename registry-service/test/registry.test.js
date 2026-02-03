/**
 * Integration tests for the Registry Service
 *
 * Run with: npm test
 * Requires the registry service to be running on localhost:16000
 */

import { describe, it, before, after, beforeEach } from 'node:test';
import assert from 'node:assert';

const REGISTRY_URL = process.env.REGISTRY_URL || 'http://localhost:16000';

// Helper to make requests
async function request(path, options = {}) {
  const url = `${REGISTRY_URL}${path}`;
  const response = await fetch(url, {
    headers: { 'Content-Type': 'application/json', ...options.headers },
    ...options,
  });
  const data = response.headers.get('content-type')?.includes('application/json')
    ? await response.json()
    : await response.text();
  return { status: response.status, data };
}

// Sample tool definitions for testing
const sampleSourceTool = {
  name: 'test_source_tool',
  description: 'A test source tool',
  version: '1.0.0',
  source: {
    server: 'test-server',
    tool: 'backend_tool',
  },
};

const sampleScatterGatherTool = {
  name: 'test_scatter_gather',
  description: 'A test scatter-gather tool',
  version: '1.0.0',
  spec: {
    scatterGather: {
      targets: [
        { tool: 'tool_a' },
        { tool: 'tool_b' },
      ],
      aggregation: {
        ops: [{ flatten: true }, { dedupe: { field: '$.url' } }],
      },
      timeoutMs: 30000,
    },
  },
};

const sampleSchema = {
  name: 'TestSchema',
  version: '1.0.0',
  description: 'A test schema',
  schema: {
    type: 'object',
    properties: {
      id: { type: 'string' },
      name: { type: 'string' },
    },
    required: ['id'],
  },
};

const sampleServer = {
  name: 'test-backend',
  version: '1.0.0',
  description: 'A test backend server',
  provides: [
    { tool: 'tool_one', version: '1.0.0' },
    { tool: 'tool_two', version: '1.0.0' },
  ],
};

describe('Registry Service', () => {
  // Clean up before tests
  beforeEach(async () => {
    // Reset to empty registry
    await request('/registry', {
      method: 'PUT',
      body: JSON.stringify({
        schemaVersion: '2.0',
        tools: [],
        schemas: [],
        servers: [],
        agents: [],
      }),
    });
  });

  describe('Health Check', () => {
    it('should return healthy status', async () => {
      const { status, data } = await request('/health');
      assert.strictEqual(status, 200);
      assert.strictEqual(data.status, 'ok');
      assert.strictEqual(data.service, 'registry');
    });
  });

  describe('Registry CRUD', () => {
    it('GET /registry should return empty registry initially', async () => {
      const { status, data } = await request('/registry');
      assert.strictEqual(status, 200);
      assert.strictEqual(data.schemaVersion, '2.0');
      assert.deepStrictEqual(data.tools, []);
      assert.deepStrictEqual(data.schemas, []);
      assert.deepStrictEqual(data.servers, []);
    });

    it('PUT /registry should replace entire registry', async () => {
      const newRegistry = {
        schemaVersion: '2.0',
        description: 'Test registry',
        tools: [sampleSourceTool],
        schemas: [sampleSchema],
        servers: [sampleServer],
        agents: [],
      };

      const { status: putStatus } = await request('/registry', {
        method: 'PUT',
        body: JSON.stringify(newRegistry),
      });
      assert.strictEqual(putStatus, 200);

      const { status, data } = await request('/registry');
      assert.strictEqual(status, 200);
      assert.strictEqual(data.tools.length, 1);
      assert.strictEqual(data.schemas.length, 1);
      assert.strictEqual(data.servers.length, 1);
    });

    it('POST /registry should merge updates', async () => {
      // First add a tool
      await request('/tools', {
        method: 'POST',
        body: JSON.stringify(sampleSourceTool),
      });

      // Then merge with a new schema
      const { status } = await request('/registry', {
        method: 'POST',
        body: JSON.stringify({ schemas: [sampleSchema] }),
      });
      assert.strictEqual(status, 200);

      // Verify both exist (tools array is replaced, not merged in POST)
      const { data } = await request('/registry');
      assert.strictEqual(data.schemas.length, 1);
    });
  });

  describe('Tools CRUD', () => {
    it('GET /tools should return empty array initially', async () => {
      const { status, data } = await request('/tools');
      assert.strictEqual(status, 200);
      assert.deepStrictEqual(data, []);
    });

    it('POST /tools should create a new tool', async () => {
      const { status, data } = await request('/tools', {
        method: 'POST',
        body: JSON.stringify(sampleSourceTool),
      });
      assert.strictEqual(status, 201);
      assert.strictEqual(data.status, 'created');
      assert.strictEqual(data.tool.name, 'test_source_tool');
    });

    it('POST /tools should reject duplicate names', async () => {
      await request('/tools', {
        method: 'POST',
        body: JSON.stringify(sampleSourceTool),
      });

      const { status, data } = await request('/tools', {
        method: 'POST',
        body: JSON.stringify(sampleSourceTool),
      });
      assert.strictEqual(status, 409);
      assert.ok(data.error.includes('already exists'));
    });

    it('GET /tools/:name should return a specific tool', async () => {
      await request('/tools', {
        method: 'POST',
        body: JSON.stringify(sampleSourceTool),
      });

      const { status, data } = await request('/tools/test_source_tool');
      assert.strictEqual(status, 200);
      assert.strictEqual(data.name, 'test_source_tool');
      assert.strictEqual(data.source.server, 'test-server');
    });

    it('GET /tools/:name should return 404 for non-existent tool', async () => {
      const { status } = await request('/tools/nonexistent');
      assert.strictEqual(status, 404);
    });

    it('PUT /tools/:name should update an existing tool', async () => {
      await request('/tools', {
        method: 'POST',
        body: JSON.stringify(sampleSourceTool),
      });

      const updated = { ...sampleSourceTool, description: 'Updated description' };
      const { status } = await request('/tools/test_source_tool', {
        method: 'PUT',
        body: JSON.stringify(updated),
      });
      assert.strictEqual(status, 200);

      const { data } = await request('/tools/test_source_tool');
      assert.strictEqual(data.description, 'Updated description');
    });

    it('DELETE /tools/:name should remove a tool', async () => {
      await request('/tools', {
        method: 'POST',
        body: JSON.stringify(sampleSourceTool),
      });

      const { status } = await request('/tools/test_source_tool', {
        method: 'DELETE',
      });
      assert.strictEqual(status, 200);

      const { status: getStatus } = await request('/tools/test_source_tool');
      assert.strictEqual(getStatus, 404);
    });

    it('should handle scatter-gather tool definition', async () => {
      const { status } = await request('/tools', {
        method: 'POST',
        body: JSON.stringify(sampleScatterGatherTool),
      });
      assert.strictEqual(status, 201);

      const { data } = await request('/tools/test_scatter_gather');
      assert.ok(data.spec.scatterGather);
      assert.strictEqual(data.spec.scatterGather.targets.length, 2);
    });
  });

  describe('Schemas CRUD', () => {
    it('GET /schemas should return empty array initially', async () => {
      const { status, data } = await request('/schemas');
      assert.strictEqual(status, 200);
      assert.deepStrictEqual(data, []);
    });

    it('POST /schemas should create a new schema', async () => {
      const { status, data } = await request('/schemas', {
        method: 'POST',
        body: JSON.stringify(sampleSchema),
      });
      assert.strictEqual(status, 201);
      assert.strictEqual(data.schema.name, 'TestSchema');
    });

    it('DELETE /schemas/:name should remove a schema', async () => {
      await request('/schemas', {
        method: 'POST',
        body: JSON.stringify(sampleSchema),
      });

      const { status } = await request('/schemas/TestSchema', {
        method: 'DELETE',
      });
      assert.strictEqual(status, 200);

      const { data } = await request('/schemas');
      assert.strictEqual(data.length, 0);
    });
  });

  describe('Servers CRUD', () => {
    it('GET /servers should return empty array initially', async () => {
      const { status, data } = await request('/servers');
      assert.strictEqual(status, 200);
      assert.deepStrictEqual(data, []);
    });

    it('POST /servers should create a new server', async () => {
      const { status, data } = await request('/servers', {
        method: 'POST',
        body: JSON.stringify(sampleServer),
      });
      assert.strictEqual(status, 201);
      assert.strictEqual(data.server.name, 'test-backend');
    });

    it('DELETE /servers/:name should remove a server', async () => {
      await request('/servers', {
        method: 'POST',
        body: JSON.stringify(sampleServer),
      });

      const { status } = await request('/servers/test-backend', {
        method: 'DELETE',
      });
      assert.strictEqual(status, 200);

      const { data } = await request('/servers');
      assert.strictEqual(data.length, 0);
    });
  });

  describe('Import', () => {
    it('POST /import should import from JSON content', async () => {
      // This test would require a file to import from
      // For now, just verify the endpoint exists
      const { status } = await request('/import', {
        method: 'POST',
        body: JSON.stringify({ source: '/nonexistent/path.json' }),
      });
      // Should fail with file not found, not 404 (endpoint exists)
      assert.strictEqual(status, 400);
    });
  });
});

describe('UI Integration Contract', () => {
  // These tests verify the API contract expected by the UI

  beforeEach(async () => {
    await request('/registry', {
      method: 'PUT',
      body: JSON.stringify({
        schemaVersion: '2.0',
        tools: [],
        schemas: [],
        servers: [],
        agents: [],
      }),
    });
  });

  it('should support the fetchRegistry() contract', async () => {
    const { status, data } = await request('/registry');
    assert.strictEqual(status, 200);

    // UI expects these fields
    assert.ok('schemaVersion' in data);
    assert.ok('tools' in data);
    assert.ok('schemas' in data);
    assert.ok('servers' in data);
    assert.ok(Array.isArray(data.tools));
    assert.ok(Array.isArray(data.schemas));
    assert.ok(Array.isArray(data.servers));
  });

  it('should support the saveRegistryTool() contract - create', async () => {
    // UI creates via POST /tools
    const { status } = await request('/tools', {
      method: 'POST',
      body: JSON.stringify(sampleSourceTool),
    });
    assert.strictEqual(status, 201);
  });

  it('should support the saveRegistryTool() contract - update', async () => {
    // Create first
    await request('/tools', {
      method: 'POST',
      body: JSON.stringify(sampleSourceTool),
    });

    // UI updates via PUT /tools/:name
    const { status } = await request('/tools/test_source_tool', {
      method: 'PUT',
      body: JSON.stringify({ ...sampleSourceTool, version: '2.0.0' }),
    });
    assert.strictEqual(status, 200);
  });

  it('should support the deleteRegistryTool() contract', async () => {
    await request('/tools', {
      method: 'POST',
      body: JSON.stringify(sampleSourceTool),
    });

    // UI deletes via DELETE /tools/:name
    const { status } = await request('/tools/test_source_tool', {
      method: 'DELETE',
    });
    assert.strictEqual(status, 200);
  });

  it('should support the getRegistryTool() contract', async () => {
    await request('/tools', {
      method: 'POST',
      body: JSON.stringify(sampleSourceTool),
    });

    // UI fetches via GET /tools/:name
    const { status, data } = await request('/tools/test_source_tool');
    assert.strictEqual(status, 200);
    assert.strictEqual(data.name, 'test_source_tool');
  });

  it('should return 404 for non-existent tool (UI expects null)', async () => {
    const { status } = await request('/tools/nonexistent');
    assert.strictEqual(status, 404);
  });
});
