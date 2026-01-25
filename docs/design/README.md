# Design Documents

This directory contains design documents for AgentGateway's virtual tools and composition system.

## Quick Links

| Document | Description |
|----------|-------------|
| [**quickstart.md**](./quickstart.md) | Get the eCommerce demo running in 5 minutes |
| [**virtual-tools-vision.md**](./virtual-tools-vision.md) | What works, what's not implemented |
| [**code-walkthrough.md**](./code-walkthrough.md) | Where the code lives |

## Reference Documents

| Document | Description |
|----------|-------------|
| [registry-v2.md](./registry-v2.md) | Full registry v2 specification |
| [proto-codegen-migration.md](./proto-codegen-migration.md) | Proto codegen implementation (complete) |
| [composition-test-plan.md](./composition-test-plan.md) | Test plan for compositions |
| [fastmcp-transforms-comparison.md](./fastmcp-transforms-comparison.md) | Comparison with FastMCP transforms |

## Current Status

### Working Features

- **Virtual tools** (1:1): Rename, defaults, field hiding, output transforms
- **Compositions** (N:1): Pipeline, Scatter-Gather, Filter, SchemaMap, MapEach
- **Data binding**: input, step, constant, construct
- **Registry v2**: schemas, servers, agents, dependency-scoped discovery
- **Proto codegen**: TypeScript DSL → Proto → Rust runtime

### Not Yet Implemented

- **Stateful patterns**: Saga, Retry, Timeout, Cache, Circuit Breaker
- **Parallel DAG execution**: Pipeline steps run sequentially
- **Agent-as-tool**: A2A agents as composition steps (Phase 2)

## Demo

```bash
# Quick test
cd examples/ecommerce-demo
./start_services.sh  # Terminal 1
RUST_LOG=info ./target/debug/agentgateway -f gateway-configs/config.yaml  # Terminal 2
python main.py  # Terminal 3
# Open http://localhost:15000/ui
```

## Archive

Historical design documents are in [archive/](./archive/):

- `dag-executor.md` - Parallel execution design (not implemented)
- `saga-pattern.md` - Saga pattern design (not implemented)
- `registry-integration.md` - Original v1 registry design
- `registry-v2-work-packages.md` - Work packages (completed)
- `ecommerce-service-separation.md` - Service separation design (done)
