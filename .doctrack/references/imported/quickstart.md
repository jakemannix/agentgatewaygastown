---
type: reference
original_path: docs/design/quickstart.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Quickstart: Virtual Tools & Compositions (imported)

Source: `docs/design/quickstart.md`

Get the eCommerce demo running with virtual tools and compositions.

## Prerequisites
- Rust 1.86+, Node.js 18+ / npm 10+, Python 3.11+

## Build & Run

### Build Gateway
```bash
cd ui && npm install && npm run build && cd ..
cargo build -p agentgateway-app  # Debug build (fast, use for testing)
```

### Start eCommerce Demo
1. **Terminal 1**: `cd examples/ecommerce-demo && ./start_services.sh` (5 MCP servers on ports 8001-8005)
2. **Terminal 2**: `RUST_LOG=info ./target/debug/agentgateway -f examples/ecommerce-demo/gateway-configs/config.yaml` (MCP on :3000, UI on :15000)
3. **Terminal 3**: Set API key (`ANTHROPIC_API_KEY` or `OPENAI_API_KEY` or `GOOGLE_API_KEY`), then `python main.py` (agent on :9000)

## Key Compositions in Demo
| Name | Pattern | Description |
|------|---------|-------------|
| `personalized_search` | 3-step pipeline | search -> hydrate -> personalize |
| `product_with_availability` | 2-step pipeline | get_product + check_stock |
| `top_restock_quote` | 2-step pipeline | low_stock_alerts -> get_quotes |

## Creating Your Own Composition
Three options:
1. **TypeScript DSL**: Use `@agentgateway/vmcp-dsl` builder API, compile with `vmcp-compile`
2. **Visual Builder**: Open `packages/vmcp-dsl/tool-builder/index.html`
3. **Hand-write JSON**: Directly author registry JSON with `spec` field

## Troubleshooting
- Gateway won't start: `./target/debug/agentgateway -f config.yaml --validate`
- Composition errors: `RUST_LOG=info,composition=debug`
- Missing structuredContent: tool has `outputSchema` but gateway didn't populate structured response
- Tools not in tools/list: check `X-Agent-Name` matches registry, verify agent `depends`

## Related Notes
- [[references/imported/virtual-tools-vision|Virtual Tools Vision]] — architecture overview
- [[references/imported/code-walkthrough|Code Walkthrough]] — implementation details
