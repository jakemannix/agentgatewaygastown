# Registry Service (Prototype)

A standalone registry service for AgentGateway virtual tool definitions.

> **Note:** This is a prototype/throwaway implementation for development. In production, this would be a proper service with database backend, authentication, versioning, and notification capabilities.

## Quick Start

```bash
cd registry-service
npm install
npm start
```

The service runs on `http://localhost:16000` by default.

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `REGISTRY_PORT` | `16000` | Port to listen on |
| `REGISTRY_FILE` | `./data/registry.json` | Path to registry JSON file |

## API Endpoints

### Health & Registry

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/registry` | Get full registry (for gateway polling) |
| PUT | `/registry` | Replace entire registry |
| POST | `/registry` | Partial update (merge) |

### Tools CRUD

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/tools` | List all tools |
| GET | `/tools/:name` | Get single tool |
| POST | `/tools` | Create tool |
| PUT | `/tools/:name` | Update tool |
| DELETE | `/tools/:name` | Delete tool |

### Schemas CRUD

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/schemas` | List all schemas |
| POST | `/schemas` | Create schema |
| DELETE | `/schemas/:name` | Delete schema |

### Servers CRUD

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/servers` | List all servers |
| POST | `/servers` | Create server |
| DELETE | `/servers/:name` | Delete server |

### Import

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/import` | Import registry from file path or URL |

## Usage with AgentGateway

### 1. Start the Registry Service

```bash
cd registry-service
npm start
```

### 2. Import Existing Registry

```bash
# From a local file
curl -X POST http://localhost:16000/import \
  -H "Content-Type: application/json" \
  -d '{"source": "/path/to/registry.json"}'

# From a URL
curl -X POST http://localhost:16000/import \
  -H "Content-Type: application/json" \
  -d '{"source": "http://example.com/registry.json"}'
```

### 3. Configure Gateway to Poll Registry

In your gateway config, set the registry source to the service URL:

```yaml
registry:
  source: http://localhost:16000/registry
  refresh_interval: 30s
```

### 4. Access the UI

The AgentGateway UI will automatically connect to the registry service at `http://localhost:16000`.

To use a different URL, set the environment variable when building/running the UI:

```bash
NEXT_PUBLIC_REGISTRY_URL=http://my-registry:16000 npm run build
```

## Architecture Notes

**Current (Prototype):**
- Single JSON file storage
- No authentication
- No notifications (gateways must poll)
- No versioning/history

**Future (Production):**
- Database backend (PostgreSQL, etc.)
- Authentication/authorization (OAuth, API keys)
- Webhook notifications to gateways on changes
- Version history and rollback
- Multi-tenancy support
- Schema validation on write
- Dependency graph analysis

## Development

```bash
# Watch mode (auto-restart on changes)
npm run dev

# With custom port
REGISTRY_PORT=17000 npm start

# With custom registry file
REGISTRY_FILE=/tmp/my-registry.json npm start
```
