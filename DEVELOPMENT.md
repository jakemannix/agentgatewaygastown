# Quickstart (GitHub, no local install)

1. Click **Code → Create codespace on main**.
2. In the terminal:
   cargo fmt --all
   cargo clippy --all -- -D warnings
   cargo test --all
3. If you touched the UI:
   cd ui
   npm ci
   npm test

# Local Development

This page contains instructions on how to run everything locally.

## Build from Source

Requirements:
- Rust 1.86+
- npm 10+

Build the agentgateway UI:

```bash
cd ui
npm install
npm run build
```

Build the agentgateway binary:

```bash
cd ..
export CARGO_NET_GIT_FETCH_WITH_CLI=true
make build
```

Run the agentgateway binary:

```bash
./target/release/agentgateway
```
Open your browser and navigate to `http://localhost:15000/ui` to see the agentgateway UI.

## Performance Benchmarks

The project uses [divan](https://github.com/nvzqz/divan) for performance benchmarking.

### Running Benchmarks

Run all benchmarks:
```bash
make bench
```

This runs benchmarks for:
- **CEL expression evaluation**: compile, build, and execute phases for various expression types
- **HTTP route matching**: performance across different route table sizes
- **Authorization**: policy evaluation overhead

### Establishing Baselines

Save a baseline measurement (e.g., before making changes):
```bash
make bench-baseline
```

Compare against baseline after making changes:
```bash
make bench-compare
```

### Interpreting Results

Benchmark output shows timing statistics:
- **fastest/slowest**: Min/max observed times
- **median**: The middle value (most representative)
- **mean**: Average time (can be skewed by outliers)
- **samples/iters**: Number of samples and iterations per sample

Example output:
```
agentgateway                  fastest       │ slowest       │ median        │ mean
├─ cel
│  ╰─ benches
│     ├─ bench_execute
│     │  ├─ header            126 ns        │ 165 ns        │ 131 ns        │ 133 ns
│     │  ╰─ simple_access     67 ns         │ 122 ns        │ 68 ns         │ 69 ns
```

Focus on the **median** for stable comparisons. Large differences between median and mean suggest outliers.

### Adding New Benchmarks

Benchmarks are defined inline in source files using the `#[divan::bench]` attribute. They require the `internal_benches` feature flag to be enabled.

Example benchmark:
```rust
#[divan::bench]
fn bench_my_function(b: Bencher) {
    // Setup code (not measured)
    let input = setup_data();

    b.bench(|| {
        divan::black_box(my_function(&input))
    });
}
```

See existing benchmarks in:
- `crates/agentgateway/src/cel/benches.rs` - CEL expression benchmarks
- `crates/agentgateway/src/http/route_test.rs` - Route matching benchmarks
- `crates/agentgateway/src/http/authorization_tests.rs` - Authorization benchmarks

