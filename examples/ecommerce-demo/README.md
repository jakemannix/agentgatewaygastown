# eCommerce Demo

A comprehensive eCommerce demo showcasing **agentgateway's virtual tools** with **A2A (Agent-to-Agent)** communication and composition patterns like pipelines, sagas, and scatter-gather.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        Chat Web UI (FastHTML)                     │
│                         http://localhost:8080                     │
└────────────────────────────┬─────────────────────────────────────┘
                             │ REST /chat endpoint
              ┌──────────────┴──────────────┐
              ▼                              ▼
┌─────────────────────────┐    ┌─────────────────────────┐
│   Customer Agent        │    │   Merchandiser Agent    │
│   (Google ADK)          │    │   (LangGraph)           │
│   http://localhost:9001 │    │   http://localhost:9002 │
│   - /chat (REST)        │◄──►│   - /chat (REST)        │
│   - / (A2A)             │    │   - / (A2A)             │
└────────────┬────────────┘    └────────────┬────────────┘
             │                              │
             │         MCP Protocol         │
             └──────────────┬───────────────┘
                            ▼
              ┌─────────────────────────┐
              │     AgentGateway        │
              │   (Virtual Tools)       │
              │   http://localhost:3000 │
              └────────────┬────────────┘
                           │ MCP (Streamable HTTP)
     ┌───────┬─────────────┼─────────────┬───────┐
     ▼       ▼             ▼             ▼       ▼
┌────────┐┌────────┐┌───────────┐┌──────────┐┌────────┐
│Catalog ││ Cart   ││  Order    ││ Inventory││Supplier│
│ :8001  ││ :8002  ││  :8003    ││  :8004   ││ :8005  │
└───┬────┘└───┬────┘└─────┬─────┘└────┬─────┘└───┬────┘
    │         │           │           │          │
    ▼         ▼           ▼           ▼          ▼
 [SQLite databases - one per service]
```

## Key Features

- **A2A Protocol**: Agents expose standard A2A endpoints for inter-agent communication
- **REST Chat API**: Simple `/chat` endpoint for user-facing web applications
- **MCP Gateway**: Virtual tools with composition patterns (pipeline, saga, scatter-gather)
- **HTTP MCP Services**: All backend services use streamable-http transport (not stdio)
- **Multi-Framework**: Customer agent uses Google ADK, Merchandiser uses LangGraph
- **Configurable LLMs**: Works with Anthropic, OpenAI, or Google models
- **Semantic Search**: Product search using sentence-transformers and sqlite-vec

## Quick Start

### Prerequisites

- Rust 1.86+ (for agentgateway)
- Python 3.10+
- [uv](https://docs.astral.sh/uv/) (Python package manager) - **required**
- An LLM API key (Anthropic, OpenAI, or Google)

### 1. Install uv (if needed)

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

### 2. Install Dependencies

```bash
cd examples/ecommerce-demo
uv sync
```

> Note: `start_services.sh` will automatically run `uv sync` if needed.

### 3. Set LLM API Key

```bash
# Choose one:
export ANTHROPIC_API_KEY="sk-ant-..."  # Recommended
export OPENAI_API_KEY="sk-..."
export GOOGLE_API_KEY="..."
```

### 4. Build AgentGateway

```bash
cd ../..  # project root
make build
```

### 5. Start All Services

```bash
cd examples/ecommerce-demo
./start_services.sh
```

This starts:
- MCP Backend Services (HTTP):
  - Catalog Service: http://localhost:8001
  - Cart Service: http://localhost:8002
  - Order Service: http://localhost:8003
  - Inventory Service: http://localhost:8004
  - Supplier Service: http://localhost:8005
- Gateway (MCP): http://localhost:3000
- Customer Agent: http://localhost:9001 (REST /chat + A2A)
- Merchandiser Agent: http://localhost:9002 (REST /chat + A2A)
- Chat Web UI: http://localhost:8080

### 6. Open the Chat UI

Open http://localhost:8080 and start chatting!

### 7. Stop Services

```bash
./stop_services.sh
```

Or if using tmux directly: `tmux kill-session -t ecommerce-demo`

## Agents

### Customer Agent (Google ADK)

Shopping assistant that helps customers:
- Search for products using natural language
- Browse by category
- Manage shopping cart
- Complete checkout with order tracking

**Endpoints**:
- REST Chat: `POST http://localhost:9001/chat` - For web UI interaction
- A2A: `POST http://localhost:9001/` - For agent-to-agent communication

**Skills**:
- Product Search
- Cart Management
- Checkout
- Order Tracking

### Merchandiser Agent (LangGraph)

Inventory management assistant that helps merchandisers:
- Monitor stock levels and low stock alerts
- Create and manage purchase orders
- Track supplier deliveries
- Analyze sales data

**Endpoints**:
- REST Chat: `POST http://localhost:9002/chat` - For web UI interaction
- A2A: `POST http://localhost:9002/` - For agent-to-agent communication

**Skills**:
- Inventory Monitoring
- Purchase Order Management
- Supplier Management
- Sales Analytics

## Virtual Tools (Gateway)

The gateway exposes virtual tools that compose the underlying MCP services:

| Pattern | Tool | Description |
|---------|------|-------------|
| **Alias** | `browse_products` | Direct mapping to catalog service |
| **Transform** | `get_product_details` | Hides internal fields from customers |
| **Pipeline** | `restock_report` | Aggregates low stock + suppliers + POs |
| **Saga** | `safe_checkout` | Transaction with inventory reservation |
| **Scatter-Gather** | `merchandiser_dashboard` | Parallel fetch of all dashboard data |
| **Cache** | `cached_categories` | 5-minute TTL for frequent reads |
| **Timeout** | `quick_search` | 3-second deadline with fallback |

## Directory Structure

```
examples/ecommerce-demo/
├── agents/
│   ├── customer_agent/     # Google ADK agent
│   │   ├── agent.py        # Agent implementation
│   │   └── __main__.py     # A2A server entry point
│   ├── merchandiser_agent/ # LangGraph agent
│   │   ├── agent.py        # Agent implementation
│   │   └── __main__.py     # A2A server entry point
│   └── shared/             # Shared utilities
│       ├── gateway_client.py   # MCP gateway client
│       └── a2a_server.py       # A2A server base class
├── mcp_tools/              # MCP backend services
│   ├── catalog_service/
│   ├── cart_service/
│   ├── order_service/
│   ├── inventory_service/
│   └── supplier_service/
├── web_ui/
│   ├── chat_app.py         # FastHTML chat interface
│   └── static/
├── gateway-configs/
│   ├── config.yaml         # Gateway configuration
│   └── ecommerce_registry.json  # Virtual tools
├── data/
│   ├── seed_data.py        # Sample data (20 products, 5 suppliers)
│   └── generate_synthetic.py  # LLM-powered synthetic data generator
├── start_services.sh       # Start all services (requires uv, tmux)
└── stop_services.sh        # Stop all services
```

## Configuration

### LLM Provider Selection

Set environment variables to configure the LLM:

```bash
# Use a specific provider
export LLM_PROVIDER=anthropic  # or openai, google

# Or use a specific model
export LLM_MODEL=claude-sonnet-4-20250514
```

### Agent Ports

```bash
export CUSTOMER_AGENT_PORT=9001
export MERCHANDISER_AGENT_PORT=9002
export GATEWAY_URL=http://localhost:3000
export WEB_PORT=8080
```

## A2A Protocol

Both agents expose standard A2A endpoints:

- `GET /` or `GET /.well-known/agent.json` - Agent card
- `POST /` with `message/send` - Send message
- `POST /` with `tasks/get` - Get task status

Example A2A request:
```json
{
  "jsonrpc": "2.0",
  "id": "123",
  "method": "message/send",
  "params": {
    "message": {
      "role": "user",
      "content": [{"kind": "Text", "text": "Search for headphones"}]
    }
  }
}
```

## Synthetic Data Generation

Generate realistic product and supplier data using LLMs:

```bash
cd examples/ecommerce-demo

# Generate 50 products (auto-detects LLM from API key)
uv run python data/generate_synthetic.py products --count 50

# Generate products in a specific niche
uv run python data/generate_synthetic.py products --count 20 --prompt "luxury watches and jewelry"

# Generate 10 suppliers
uv run python data/generate_synthetic.py suppliers --count 10

# Use a specific provider
uv run python data/generate_synthetic.py products --count 10 --provider openai

# Output to file instead of seeding database
uv run python data/generate_synthetic.py products --count 10 --output my_products.json

# Dry run (preview without saving)
uv run python data/generate_synthetic.py products --count 5 --dry-run
```

The generator uses your configured LLM API key (Anthropic, OpenAI, or Google) to create varied, realistic data including:
- Products with realistic names, descriptions, prices, and stock levels
- Suppliers with varied lead times and reliability scores
- Proper category distribution and pricing relationships

## Troubleshooting

### No LLM API key
Agents will fall back to direct tool calls without LLM reasoning. Set `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, or `GOOGLE_API_KEY`.

### Gateway not connecting
Ensure the gateway is built and running: `make build && make run-gateway`

### Agents not responding
Check that the gateway is running first - agents need it for tool calls.

### Reset databases
```bash
cd examples/ecommerce-demo

# Reset and re-seed with default data
uv run python data/seed_data.py --reset

# Or reset and generate fresh synthetic data
uv run python data/seed_data.py --reset
uv run python data/generate_synthetic.py products --count 100
```

## License

MIT - Same as parent agentgateway project.
