---
type: reference
original_path: DEVELOPMENT.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Development Guide (imported)

Source: `DEVELOPMENT.md`

## Build Requirements
- Rust 1.86+ (workspace edition 2024, rust-version 1.90)
- npm 10+

## Build Commands
```bash
# UI
cd ui && npm install && npm run build && cd ..

# Binary (release)
export CARGO_NET_GIT_FETCH_WITH_CLI=true
make build

# Debug (fast, for development)
cargo build -p agentgateway-app
```

## Key Makefile Targets
- `make build` — release build with UI
- `make lint` — formatting + clippy
- `make fix-lint` — auto-fix lint
- `make test` — all tests
- `make gen` — regenerate APIs and schema
- `make validate` — validate example configs
- `make bench` — performance benchmarks (divan)

## Benchmark Areas
- CEL expression evaluation (compile, build, execute)
- HTTP route matching (various route table sizes)
- Authorization (policy evaluation overhead)

## Run
```bash
./target/release/agentgateway -f examples/basic/config.yaml
# UI at http://localhost:15000/ui
```
