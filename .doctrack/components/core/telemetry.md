---
feature: core
type: component
files:
  - crates/core/src/telemetry.rs
  - crates/core/src/telemetry/date.rs
  - crates/core/src/telemetry/msg.rs
  - crates/core/src/telemetry/nonblocking.rs
  - crates/core/src/telemetry/worker.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Telemetry (Structured Logging)

## Responsibility

High-performance structured logging subsystem with Istio-compatible output formats (plain and JSON), non-blocking writer thread, dynamic log level reload, and a test logger for assertions.

## Internal Logic

### Log Pipeline

```
tracing macros --> tracing-subscriber layer --> NonBlocking writer --> Worker thread --> stdout
                                                    |
                                              crossbeam channel
                                              (bounded, 10K lines)
```

### Key Components

**`setup_logging(default_level, json)`** - Entry point. Creates the non-blocking writer, configures the tracing subscriber with either `IstioFormat` (plain) or `IstioJsonFormat` (JSON), stores a `reload::Handle` for dynamic level changes.

**`IstioFormat`** (plain text) - Format: `{timestamp}\t{level}\t{target}\t{message}\t{key=value pairs}`. Target strips `agentgateway::` prefix for brevity. Span fields shown as `:{span_name}{field=value}`.

**`IstioJsonFormat`** (JSON) - Serializes to `{"level":"info","time":"...","scope":"target",...}`. Span fields serialized as nested JSON objects keyed by span name.

**`log()` function** - Bypass for tracing macros when you need dynamic k/v pairs (tracing requires compile-time keys). Uses thread-local buffer reuse to minimize allocations. Writes directly to the `NonBlocking` channel.

**`set_level(reset, level)`** - Dynamic log level modification via `reload::Handle`. Can append new directives or reset to defaults. Custom suppressions for noisy targets: `rmcp=warn`, `hickory_server=off`, `typespec_client_core=warn`.

### Non-Blocking Writer (`nonblocking.rs`)

Forked from `tracing-appender`. Key differences from upstream:
- **Vectored I/O batching**: Worker collects up to 64 messages per flush via `VectoredIOHelper`, writes them with `write_vectored` in a single syscall.
- **Configurable backpressure**: `lossy(false)` mode blocks senders instead of dropping logs. Default buffer is 10K lines (not the default 128K).
- **`write_vec()`**: Fast path that sends `Vec<u8>` directly without copying (vs `write()` which must copy from `&[u8]`).

### Date Caching (`date.rs`)

Thread-local `CachedDate` struct caches the formatted timestamp prefix (`YYYY-MM-DDThh:mm:ss.`). Only the microseconds and trailing `Z` are appended per log line. Cache invalidates once per second. This eliminates ~50% of per-log formatting cost.

### Worker Thread (`worker.rs`)

Dedicated OS thread (not tokio task) that:
1. Blocks on `crossbeam_channel::recv()` for the first message
2. Drains up to 63 more via `try_recv()` (non-blocking)
3. Flushes the batch with vectored I/O
4. On shutdown signal, waits for acknowledgment before exiting

### Test Logger (`testing` module)

`setup_test_logging()` configures a global JSON logger that tees to both stdout and an in-memory `MockWriter` buffer. `find(&[("key", "value")])` searches the buffer for matching JSON log lines. Used throughout the test suite.

## Relationships

- **Used by**: Every crate in the workspace (logging is universal)
- **Uses**: [[components/core/strng|strng]] for `RichStrng` in log fields
- **Related**: [[components/core/tracing|trcng]] (distributed tracing, separate from structured logging)
- **Parent**: [[features/core|Core]]
