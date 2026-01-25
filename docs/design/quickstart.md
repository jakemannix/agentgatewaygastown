# Quickstart: Virtual Tools & Compositions

Get the eCommerce demo running with virtual tools and compositions.

## Prerequisites

- Rust 1.86+
- Node.js 18+ / npm 10+
- Python 3.11+ (for demo agents)

## 1. Build the Gateway

```bash
# Clone and enter the repo
cd agentgatewaygastown

# Build UI (required for gateway)
cd ui && npm install && npm run build && cd ..

# Build gateway (debug mode for faster iteration)
cargo build -p agentgateway-app

# Verify build
./target/debug/agentgateway --version
```

## 2. Run the eCommerce Demo

### Terminal 1: Start Backend Services

```bash
cd examples/ecommerce-demo
./start_services.sh
```

This starts 5 MCP servers on ports 8001-8005:
- catalog-service (8001)
- cart-service (8002)
- order-service (8003)
- inventory-service (8004)
- supplier-service (8005)

### Terminal 2: Start the Gateway

```bash
RUST_LOG=info ./target/debug/agentgateway -f examples/ecommerce-demo/gateway-configs/config.yaml
```

Gateway starts on:
- **MCP endpoint**: http://localhost:3000/mcp
- **UI**: http://localhost:15000/ui

### Terminal 3: Start Demo Agents

```bash
cd examples/ecommerce-demo

# Install Python dependencies
pip install -r requirements.txt

# Set your LLM API key (pick one)
export ANTHROPIC_API_KEY=your-key  # Claude
# OR
export OPENAI_API_KEY=your-key     # GPT-4
# OR
export GOOGLE_API_KEY=your-key     # Gemini

# Start the agent orchestrator
python main.py
```

Agent orchestrator runs on http://localhost:9000 with:
- Customer agent on /customer
- Merchandiser agent on /merchandiser

## 3. Test the System

### Via UI

1. Open http://localhost:15000/ui
2. Go to the Chat tab
3. Select "customer-agent"
4. Try: "Search for coffee makers"

### Via curl

```bash
# Direct MCP call to gateway
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "X-Agent-Name: customer-agent" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list"
  }'

# Call a composition
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "X-Agent-Name: customer-agent" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "virtual_personalized_search",
      "arguments": {
        "query": "coffee makers",
        "user_id": "web-user"
      }
    }
  }'
```

## 4. Explore the Registry

The registry defines virtual tools and compositions:

```bash
# View the registry
cat examples/ecommerce-demo/gateway-configs/ecommerce_registry_v2.json | jq '.tools | length'
# Output: 30+ tools

# See composition definitions
cat examples/ecommerce-demo/gateway-configs/ecommerce_registry_v2.json | jq '.tools[] | select(.spec != null) | .name'
# Output: personalized_search, product_with_availability, top_restock_quote, etc.
```

### Key Compositions

| Name | Pattern | Description |
|------|---------|-------------|
| `personalized_search` | 3-step pipeline | search → hydrate → personalize |
| `product_with_availability` | 2-step pipeline | get_product + check_stock |
| `top_restock_quote` | 2-step pipeline | low_stock_alerts → get_quotes |
| `product_intelligence` | 3-step pipeline | product + stock + quotes |

## 5. Create Your Own Composition

### Option A: TypeScript DSL

```typescript
// my-tools.ts
import { tool, pipeline } from '@agentgateway/vmcp-dsl';

export const myTool = tool("my_composition")
  .description("My custom composition")
  .pipeline(p => p
    .step("first", t => t.tool("backend_tool_1", "my-service")
      .input({ input: { path: "$" } }))
    .step("second", t => t.tool("backend_tool_2", "my-service")
      .input({ step: { stepId: "first", path: "$.result" } }))
  )
  .build();

export const registry = {
  schemaVersion: "2.0",
  tools: [myTool],
  schemas: [],
  servers: [],
  agents: []
};
```

Compile:
```bash
cd packages/vmcp-dsl
npx ts-node bin/vmcp-compile.ts my-tools.ts -o my-registry.json
```

### Option B: Visual Builder

1. Open `packages/vmcp-dsl/tool-builder/index.html` in a browser
2. Add steps visually
3. Configure data bindings
4. Export JSON

### Option C: Hand-write JSON

```json
{
  "schemaVersion": "2.0",
  "tools": [
    {
      "name": "my_composition",
      "description": "My custom composition",
      "spec": {
        "pipeline": {
          "steps": [
            {
              "id": "first",
              "operation": { "tool": { "name": "backend_tool_1", "server": "my-service" } },
              "input": { "input": { "path": "$" } }
            },
            {
              "id": "second",
              "operation": { "tool": { "name": "backend_tool_2", "server": "my-service" } },
              "input": { "step": { "stepId": "first", "path": "$.result" } }
            }
          ]
        }
      }
    }
  ]
}
```

## 6. Run Tests

```bash
# Rust tests (all)
make test

# Just registry tests
cargo test -p agentgateway registry

# TypeScript tests
cd packages/vmcp-dsl && npm test
```

## Troubleshooting

### Gateway won't start

```bash
# Check config syntax
./target/debug/agentgateway -f config.yaml --validate

# Enable debug logging
RUST_LOG=debug ./target/debug/agentgateway -f config.yaml
```

### Composition returns error

```bash
# Check gateway logs for execution trace
RUST_LOG=info,composition=debug ./target/debug/agentgateway -f config.yaml
```

### Agent returns 500

Common causes:
1. **Missing structuredContent**: Tool has `outputSchema` but gateway didn't populate `structuredContent`
2. **Wrong tool name**: Agent calling `tool_name` instead of `virtual_tool_name`
3. **Missing dependency**: Agent's `depends` doesn't include the tool

### Tools not showing in tools/list

1. Check `X-Agent-Name` header matches registry agent name
2. Verify agent's `depends` includes the tool
3. Check gateway logs for filtering messages

## Next Steps

- Read [virtual-tools-vision.md](./virtual-tools-vision.md) for architecture overview
- Read [code-walkthrough.md](./code-walkthrough.md) for implementation details
- Explore more patterns in `examples/pattern-demos/`
