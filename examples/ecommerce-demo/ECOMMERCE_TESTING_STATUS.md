# eCommerce Demo Testing Status

**Last Updated**: 2026-01-19
**Branch**: feature/tool-algebra

This document captures the current state of the eCommerce demo to enable continuation of testing in a new Claude Code session.

## Architecture Overview

```
┌─────────────────┐     ┌─────────────────┐
│  Customer UI    │     │ Merchandiser UI │
│   (FastHTML)    │     │   (FastHTML)    │
│   Port 8080     │     │   Port 8081     │
└────────┬────────┘     └────────┬────────┘
         │ REST /chat            │ REST /chat
         ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│ Customer Agent  │     │Merchandiser Agent│
│  (Google ADK)   │     │  (LangGraph)    │
│   Port 9001     │     │   Port 9002     │
└────────┬────────┘     └────────┬────────┘
         │                       │
         └───────────┬───────────┘
                     │ MCP over HTTP
            ┌────────▼────────┐
            │  AgentGateway   │
            │   Port 3000     │
            │  (Admin: 15000) │
            └────────┬────────┘
                     │
     ┌───────┬───────┼───────┬───────┐
     ▼       ▼       ▼       ▼       ▼
┌────────┐┌────────┐┌────────┐┌────────┐┌────────┐
│Catalog ││ Cart   ││ Order  ││Inventory││Supplier│
│:8001   ││:8002   ││:8003   ││:8004    ││:8005   │
└────────┘└────────┘└────────┘└────────┘└────────┘
```

## Current Component Status

| Component | Port | Status | Notes |
|-----------|------|--------|-------|
| Catalog Service | 8001 | ✅ Working | MCP over HTTP with vector search |
| Cart Service | 8002 | ✅ Working | MCP over HTTP |
| Order Service | 8003 | ✅ Working | MCP over HTTP |
| Inventory Service | 8004 | ✅ Working | MCP over HTTP |
| Supplier Service | 8005 | ✅ Working | MCP over HTTP |
| AgentGateway | 3000 | ✅ Working | Exposes 38 tools |
| Gateway Admin UI | 15000 | ✅ Available | http://localhost:15000/ui |
| Customer Agent | 9001 | ⚠️ Requires GOOGLE_API_KEY | Google ADK + Gemini |
| Merchandiser Agent | 9002 | ✅ Working | LangGraph + Anthropic |
| Web Chat UI | 8080 | ✅ Working | Single UI for both agents |

## Quick Start Commands

### 1. Start MCP Backend Services
```bash
cd /Users/jake/src/open_src/agentgatewaygastown/examples/ecommerce-demo

# Start all 5 services (from repo root, or use start_services.sh)
.venv/bin/python -m mcp_tools.catalog_service.server > /tmp/catalog.log 2>&1 &
.venv/bin/python -m mcp_tools.cart_service.server > /tmp/cart.log 2>&1 &
.venv/bin/python -m mcp_tools.order_service.server > /tmp/order.log 2>&1 &
.venv/bin/python -m mcp_tools.inventory_service.server > /tmp/inventory.log 2>&1 &
.venv/bin/python -m mcp_tools.supplier_service.server > /tmp/supplier.log 2>&1 &
```

### 2. Start Gateway (from repo root)
```bash
cd /Users/jake/src/open_src/agentgatewaygastown
./target/release/agentgateway -f examples/ecommerce-demo/gateway-configs/config.yaml > /tmp/gateway.log 2>&1 &
```

### 3. Start Agents
```bash
cd /Users/jake/src/open_src/agentgatewaygastown/examples/ecommerce-demo
.venv/bin/python -m agents.customer_agent > /tmp/customer_agent.log 2>&1 &
.venv/bin/python -m agents.merchandiser_agent > /tmp/merchandiser_agent.log 2>&1 &
```

### 4. Check All Running
```bash
for port in 8001 8002 8003 8004 8005 3000 9001 9002; do
  pid=$(lsof -ti :$port 2>/dev/null)
  if [ -n "$pid" ]; then echo "Port $port: Running (PID $pid)"; else echo "Port $port: NOT RUNNING"; fi
done
```

### Stop Everything
```bash
cd /Users/jake/src/open_src/agentgatewaygastown/examples/ecommerce-demo
./stop_services.sh
# Or manually:
for port in 8001 8002 8003 8004 8005 3000 9001 9002 8080 8081 15000 15020 15021; do
  pid=$(lsof -ti :$port 2>/dev/null)
  [ -n "$pid" ] && kill $pid 2>/dev/null
done
```

## Testing Commands

### Test Gateway MCP Connection
```bash
# Initialize session (get session ID from mcp-session-id header)
curl -i -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'

# List tools (use session ID from above)
SESSION="<session-id-from-above>"
curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
```

### Test Agent Chat Endpoints
```bash
# Test Merchandiser Agent (use 60s timeout for LLM calls)
curl -s --max-time 60 -X POST http://localhost:9002/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "show inventory report", "user_id": "test-user", "session_id": "test-123"}'

# Test Customer Agent
curl -s --max-time 60 -X POST http://localhost:9001/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "search for headphones", "user_id": "test-user", "session_id": "test-123"}'
```

### Test Individual MCP Services Directly
```bash
# Catalog service
curl -s -X POST http://localhost:8001/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
```

## Issues Fixed This Session

### 1. Gateway Config - Field Naming (camelCase)
- **File**: `gateway-configs/config.yaml`
- **Fix**: Changed `refresh_interval` → `refreshInterval`

### 2. Gateway Config - MCP Target Format
- **File**: `gateway-configs/config.yaml`
- **Fix**: Changed `streamableHttp: { url: ... }` → `mcp: { host: ... }`
```yaml
# Wrong:
- name: catalog-service
  streamableHttp:
    url: http://localhost:8001/mcp

# Correct:
- name: catalog-service
  mcp:
    host: http://localhost:8001/mcp
```

### 3. Registry Path
- **File**: `gateway-configs/config.yaml`
- **Fix**: Changed `file://./ecommerce_registry.json` → `file://./examples/ecommerce-demo/gateway-configs/ecommerce_registry.json`

### 4. Registry JSON - Invalid Comment Objects
- **File**: `gateway-configs/ecommerce_registry.json`
- **Fix**: Removed standalone `{ "$comment": "..." }` objects that had no `name` field
- **Note**: Simplified registry to only passthrough tools (removed complex compositions that had schema errors)

### 5. Merchandiser Agent - Tool Invocation
- **File**: `agents/merchandiser_agent/__main__.py`
- **Fix**: LangChain `@tool` decorated functions need `.invoke({})` not direct call
```python
# Wrong:
result = get_inventory_report()

# Correct:
result = get_inventory_report.invoke({})
```

### 6. Gateway Client - MCP Session Management (NEW)
- **File**: `agents/shared/gateway_client.py`
- **Fix**: Added `_ensure_initialized()` to send `initialize` request before other MCP calls
- **Fix**: Added SSE response parsing (`data: {...}` format)
- **Fix**: Store and reuse `mcp-session-id` header

### 7. Gateway Client - Event Loop Handling (NEW)
- **Files**: `agents/merchandiser_agent/agent.py`, `agents/customer_agent/agent.py`
- **Fix**: Replace `asyncio.get_event_loop().run_until_complete()` with context-aware helper
```python
def _call_gateway_tool(name: str, args: dict) -> Any:
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        return asyncio.run(client.call_tool(name, args))
    # If loop exists, run in thread pool
    with concurrent.futures.ThreadPoolExecutor(max_workers=1) as executor:
        future = executor.submit(asyncio.run, client.call_tool(name, args))
        return future.result()
```

### 8. Merchandiser Agent - Missing Tools (NEW)
- **File**: `agents/merchandiser_agent/agent.py`
- **Fix**: Replaced non-existent `merchandiser_dashboard`, `restock_report`, `advance_deliveries` with aggregation functions
- These tools now call multiple existing tools and combine results

### 9. Customer Agent - Tool Name Corrections (NEW)
- **File**: `agents/customer_agent/agent.py`
- **Fix**: Changed `safe_checkout` → `checkout` (match registry)
- **Fix**: Changed `list_my_orders` → `list_orders` (match registry)

### 10. Catalog Service - Vector Search Query (NEW)
- **File**: `mcp_tools/catalog_service/database.py`
- **Fix**: sqlite-vec requires `k = ?` not `LIMIT ?` for KNN queries
```sql
-- Wrong:
WHERE e.embedding MATCH ? ORDER BY e.distance LIMIT ?

-- Correct:
WHERE e.embedding MATCH ? AND k = ? ORDER BY e.distance
```

### 11. Seed Script - Module Path (NEW)
- **File**: `data/seed_data.py`
- **Fix**: Changed import path from `mcp-tools` → `mcp_tools`
- **Fix**: Use `mcp_tools.catalog_service.database` instead of `catalog_service.database`

### 12. Web UI - FastHTML Response Handling (NEW)
- **File**: `web_ui/chat_app.py`
- **Fix**: Handle FastHTML `Html()` returning tuple instead of single object

## Known Issues / Next Steps

### 1. LLM-Powered Agent Testing
- Agents work with fallback handlers without LLM
- For full LLM-powered agent testing, set one of:
  - `ANTHROPIC_API_KEY` (default provider)
  - `OPENAI_API_KEY`
  - `GOOGLE_API_KEY`
- Note: Google ADK agent may need `session_service` param update

### 2. Registry Compositions (Deferred)
- Simplified registry removed complex compositions (pipeline, saga, scatter-gather)
- Original had schema errors with `schemaMap` operations
- Reference correct format in: `examples/pattern-demos/configs/registry.json`

### 3. Session State Management
- Gateway clients cache MCP session IDs
- If backend services restart, agent must also restart to clear stale sessions
- Future: Implement session reconnection or error-based reinitialize

## File Locations

```
examples/ecommerce-demo/
├── gateway-configs/
│   ├── config.yaml              # Gateway configuration
│   └── ecommerce_registry.json  # Virtual tools registry
├── mcp_tools/                   # MCP backend services
│   ├── catalog_service/
│   ├── cart_service/
│   ├── order_service/
│   ├── inventory_service/
│   └── supplier_service/
├── agents/
│   ├── customer_agent/          # Google ADK agent
│   ├── merchandiser_agent/      # LangGraph agent
│   └── shared/
│       ├── a2a_server.py        # A2A + REST /chat server
│       └── gateway_client.py    # MCP client for gateway
├── web_ui/
│   ├── customer_app.py          # FastHTML customer UI
│   └── merchandiser_app.py      # FastHTML merchandiser UI
├── data/
│   ├── seed_data.py             # Sample data seeder
│   └── generate_synthetic.py    # LLM-powered data generator
├── start_services.sh            # Start all services (tmux)
├── stop_services.sh             # Stop all services
└── pyproject.toml               # Python dependencies
```

## Log Files
- `/tmp/catalog.log`
- `/tmp/cart.log`
- `/tmp/order.log`
- `/tmp/inventory.log`
- `/tmp/supplier.log`
- `/tmp/gateway.log`
- `/tmp/customer_agent.log`
- `/tmp/merchandiser_agent.log`

## Environment Variables

```bash
# Gateway URL (for agents)
export GATEWAY_URL="http://localhost:3000"

# LLM Provider (optional - agents have fallback)
export LLM_PROVIDER="anthropic"  # or "openai" or "google"
export ANTHROPIC_API_KEY="..."
# export OPENAI_API_KEY="..."
# export GOOGLE_API_KEY="..."

# Agent ports (defaults shown)
export CUSTOMER_AGENT_PORT=9001
export MERCHANDISER_AGENT_PORT=9002
```

## Verification Checklist

- [x] All 5 MCP services start on ports 8001-8005
- [x] Gateway starts and loads config from ecommerce demo
- [x] Gateway tools/list returns 38 tools from all services
- [x] Database seeded with 20 products and 5 suppliers
- [x] Merchandiser agent /chat works with fallback
- [x] Customer agent /chat works with fallback
- [x] Web Chat UI works (port 8080)
- [ ] Agents work with LLM (requires API key)
- [x] End-to-end: Web UI → Agent → Gateway → MCP Services
