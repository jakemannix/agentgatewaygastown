/**
 * Prototype Registry Service for AgentGateway
 *
 * This is a throwaway prototype to demonstrate the registry service architecture.
 * In production, this would be a proper service with:
 * - Database backend (PostgreSQL, etc.)
 * - Authentication/authorization
 * - Versioning and history
 * - Webhook notifications to gateways
 * - Multi-tenancy support
 *
 * For now, it just reads/writes to a JSON file.
 */

import express from 'express';
import cors from 'cors';
import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const app = express();
const PORT = process.env.REGISTRY_PORT || 16000;

// Registry file path - can be overridden via environment variable
const REGISTRY_FILE = process.env.REGISTRY_FILE || path.join(__dirname, 'data', 'registry.json');

// Ensure data directory exists
await fs.mkdir(path.dirname(REGISTRY_FILE), { recursive: true });

// Initialize empty registry if file doesn't exist
try {
  await fs.access(REGISTRY_FILE);
} catch {
  const emptyRegistry = {
    schemaVersion: "2.0",
    description: "AgentGateway Tool Registry",
    schemas: [],
    servers: [],
    agents: [],
    tools: [],
    metadata: {}
  };
  await fs.writeFile(REGISTRY_FILE, JSON.stringify(emptyRegistry, null, 2));
  console.log(`Created empty registry at ${REGISTRY_FILE}`);
}

// Middleware
app.use(cors({
  origin: [
    'http://localhost:3000',
    'http://localhost:15000',
    'http://localhost:19000',
    'http://127.0.0.1:3000',
    'http://127.0.0.1:15000',
    'http://127.0.0.1:19000'
  ],
  credentials: true
}));
app.use(express.json({ limit: '10mb' }));

// Helper: Read registry
async function readRegistry() {
  const content = await fs.readFile(REGISTRY_FILE, 'utf-8');
  return JSON.parse(content);
}

// Helper: Write registry
async function writeRegistry(registry) {
  await fs.writeFile(REGISTRY_FILE, JSON.stringify(registry, null, 2));
}

// Helper: Validate registry structure
function validateRegistry(registry) {
  if (!registry || typeof registry !== 'object') {
    throw new Error('Registry must be an object');
  }
  if (!registry.schemaVersion) {
    registry.schemaVersion = "2.0";
  }
  if (!Array.isArray(registry.tools)) {
    registry.tools = [];
  }
  if (!Array.isArray(registry.schemas)) {
    registry.schemas = [];
  }
  if (!Array.isArray(registry.servers)) {
    registry.servers = [];
  }
  if (!Array.isArray(registry.agents)) {
    registry.agents = [];
  }
  return registry;
}

// =============================================================================
// API Routes
// =============================================================================

// Health check
app.get('/health', (req, res) => {
  res.json({ status: 'ok', service: 'registry' });
});

// GET /registry - Full registry (for gateway polling)
app.get('/registry', async (req, res) => {
  try {
    const registry = await readRegistry();
    res.json(registry);
  } catch (err) {
    console.error('Error reading registry:', err);
    res.status(500).json({ error: err.message });
  }
});

// PUT /registry - Replace entire registry
app.put('/registry', async (req, res) => {
  try {
    const registry = validateRegistry(req.body);
    await writeRegistry(registry);
    res.json({ status: 'success', message: 'Registry updated' });
  } catch (err) {
    console.error('Error writing registry:', err);
    res.status(400).json({ error: err.message });
  }
});

// POST /registry - Merge/update registry (partial update)
app.post('/registry', async (req, res) => {
  try {
    const current = await readRegistry();
    const updates = req.body;

    // Merge updates
    const merged = {
      ...current,
      ...updates,
      // Arrays are replaced, not merged
      tools: updates.tools ?? current.tools,
      schemas: updates.schemas ?? current.schemas,
      servers: updates.servers ?? current.servers,
      agents: updates.agents ?? current.agents,
    };

    await writeRegistry(validateRegistry(merged));
    res.json({ status: 'success', message: 'Registry updated' });
  } catch (err) {
    console.error('Error updating registry:', err);
    res.status(400).json({ error: err.message });
  }
});

// =============================================================================
// Tool CRUD
// =============================================================================

// GET /tools - List all tools
app.get('/tools', async (req, res) => {
  try {
    const registry = await readRegistry();
    res.json(registry.tools || []);
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

// GET /tools/:name - Get single tool
app.get('/tools/:name', async (req, res) => {
  try {
    const registry = await readRegistry();
    const tool = registry.tools?.find(t => t.name === req.params.name);
    if (!tool) {
      return res.status(404).json({ error: 'Tool not found' });
    }
    res.json(tool);
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

// POST /tools - Create tool
app.post('/tools', async (req, res) => {
  try {
    const registry = await readRegistry();
    const tool = req.body;

    if (!tool.name) {
      return res.status(400).json({ error: 'Tool name is required' });
    }

    // Check for duplicate
    if (registry.tools?.some(t => t.name === tool.name)) {
      return res.status(409).json({ error: `Tool "${tool.name}" already exists` });
    }

    registry.tools = [...(registry.tools || []), tool];
    await writeRegistry(registry);

    res.status(201).json({ status: 'created', tool });
  } catch (err) {
    res.status(400).json({ error: err.message });
  }
});

// PUT /tools/:name - Update tool
app.put('/tools/:name', async (req, res) => {
  try {
    const registry = await readRegistry();
    const tool = req.body;
    const index = registry.tools?.findIndex(t => t.name === req.params.name);

    if (index === -1 || index === undefined) {
      return res.status(404).json({ error: 'Tool not found' });
    }

    // Preserve name from URL
    tool.name = req.params.name;
    registry.tools[index] = tool;
    await writeRegistry(registry);

    res.json({ status: 'updated', tool });
  } catch (err) {
    res.status(400).json({ error: err.message });
  }
});

// DELETE /tools/:name - Delete tool
app.delete('/tools/:name', async (req, res) => {
  try {
    const registry = await readRegistry();
    const index = registry.tools?.findIndex(t => t.name === req.params.name);

    if (index === -1 || index === undefined) {
      return res.status(404).json({ error: 'Tool not found' });
    }

    registry.tools.splice(index, 1);
    await writeRegistry(registry);

    res.json({ status: 'deleted' });
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

// =============================================================================
// Schema CRUD
// =============================================================================

// GET /schemas - List all schemas
app.get('/schemas', async (req, res) => {
  try {
    const registry = await readRegistry();
    res.json(registry.schemas || []);
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

// POST /schemas - Create schema
app.post('/schemas', async (req, res) => {
  try {
    const registry = await readRegistry();
    const schema = req.body;

    if (!schema.name) {
      return res.status(400).json({ error: 'Schema name is required' });
    }

    if (registry.schemas?.some(s => s.name === schema.name)) {
      return res.status(409).json({ error: `Schema "${schema.name}" already exists` });
    }

    registry.schemas = [...(registry.schemas || []), schema];
    await writeRegistry(registry);

    res.status(201).json({ status: 'created', schema });
  } catch (err) {
    res.status(400).json({ error: err.message });
  }
});

// DELETE /schemas/:name - Delete schema
app.delete('/schemas/:name', async (req, res) => {
  try {
    const registry = await readRegistry();
    const index = registry.schemas?.findIndex(s => s.name === req.params.name);

    if (index === -1 || index === undefined) {
      return res.status(404).json({ error: 'Schema not found' });
    }

    registry.schemas.splice(index, 1);
    await writeRegistry(registry);

    res.json({ status: 'deleted' });
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

// =============================================================================
// Server CRUD
// =============================================================================

// GET /servers - List all servers
app.get('/servers', async (req, res) => {
  try {
    const registry = await readRegistry();
    res.json(registry.servers || []);
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

// POST /servers - Create server
app.post('/servers', async (req, res) => {
  try {
    const registry = await readRegistry();
    const server = req.body;

    if (!server.name) {
      return res.status(400).json({ error: 'Server name is required' });
    }

    if (registry.servers?.some(s => s.name === server.name)) {
      return res.status(409).json({ error: `Server "${server.name}" already exists` });
    }

    registry.servers = [...(registry.servers || []), server];
    await writeRegistry(registry);

    res.status(201).json({ status: 'created', server });
  } catch (err) {
    res.status(400).json({ error: err.message });
  }
});

// DELETE /servers/:name - Delete server
app.delete('/servers/:name', async (req, res) => {
  try {
    const registry = await readRegistry();
    const index = registry.servers?.findIndex(s => s.name === req.params.name);

    if (index === -1 || index === undefined) {
      return res.status(404).json({ error: 'Server not found' });
    }

    registry.servers.splice(index, 1);
    await writeRegistry(registry);

    res.json({ status: 'deleted' });
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

// =============================================================================
// Import/Export
// =============================================================================

// POST /import - Import registry from file path or URL
app.post('/import', async (req, res) => {
  try {
    const { source } = req.body;

    if (!source) {
      return res.status(400).json({ error: 'Source is required' });
    }

    let content;
    if (source.startsWith('http://') || source.startsWith('https://')) {
      const response = await fetch(source);
      if (!response.ok) {
        throw new Error(`Failed to fetch: ${response.status}`);
      }
      content = await response.text();
    } else if (source.startsWith('file://')) {
      const filePath = source.replace('file://', '');
      content = await fs.readFile(filePath, 'utf-8');
    } else {
      // Assume it's a file path
      content = await fs.readFile(source, 'utf-8');
    }

    const imported = JSON.parse(content);
    const registry = validateRegistry(imported);
    await writeRegistry(registry);

    res.json({
      status: 'imported',
      tools: registry.tools?.length || 0,
      schemas: registry.schemas?.length || 0,
      servers: registry.servers?.length || 0
    });
  } catch (err) {
    res.status(400).json({ error: err.message });
  }
});

// =============================================================================
// Start Server
// =============================================================================

app.listen(PORT, () => {
  console.log(`Registry service running on http://localhost:${PORT}`);
  console.log(`Registry file: ${REGISTRY_FILE}`);
  console.log(`\nEndpoints:`);
  console.log(`  GET  /health          - Health check`);
  console.log(`  GET  /registry        - Get full registry (for gateway polling)`);
  console.log(`  PUT  /registry        - Replace registry`);
  console.log(`  POST /registry        - Update registry (partial)`);
  console.log(`  GET  /tools           - List tools`);
  console.log(`  GET  /tools/:name     - Get tool`);
  console.log(`  POST /tools           - Create tool`);
  console.log(`  PUT  /tools/:name     - Update tool`);
  console.log(`  DELETE /tools/:name   - Delete tool`);
  console.log(`  GET  /schemas         - List schemas`);
  console.log(`  POST /schemas         - Create schema`);
  console.log(`  DELETE /schemas/:name - Delete schema`);
  console.log(`  GET  /servers         - List servers`);
  console.log(`  POST /servers         - Create server`);
  console.log(`  DELETE /servers/:name - Delete server`);
  console.log(`  POST /import          - Import from file/URL`);
});
