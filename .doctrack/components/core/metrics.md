---
feature: core
type: component
files:
  - crates/core/src/metrics.rs
  - crates/core/src/tokio_metrics.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Metrics (Prometheus Helpers)

## Responsibility

Prometheus metrics infrastructure: registry helpers, RAII deferred recording pattern, recorder traits, label encoding wrappers for optional/display/debug/arc types, custom dynamic label fields, and Tokio runtime metrics collection.

## Internal Logic

### Registry

`sub_registry(registry)` creates a sub-registry with the `agentgateway` prefix. All metrics in the system use this prefix.

### Deferred Recording (RAII)

`Deferred<F, T>` holds a reference and a closure. When dropped, the closure is called with the reference. This enables "record on completion" patterns:

```rust
let _record = histogram.defer_record(|h| h.observe(start.elapsed()));
// ... do work ...
// histogram is recorded when _record goes out of scope
```

The `DeferRecorder` trait provides `defer_record()` as a default method for any type.

### Recorder Traits

```rust
trait Recorder<E, T> { fn record(&self, event: E, meta: T); }
trait IncrementRecorder<E>: Recorder<E, u64> { fn increment(&self, event: E); }
```

`IncrementRecorder` is auto-implemented for any `Recorder<E, u64>`, calling `record(event, 1)`.

### Label Encoding Wrappers

| Type | Purpose |
|------|---------|
| `DefaultedUnknown<T>` | Wraps `Option<T>`, encodes as `"unknown"` when `None` instead of empty string |
| `EncodeDisplay<T>` | Encodes any `Display` type as a label value via `.to_string()` |
| `EncodeDebug<T>` | Encodes any `Debug` type as a label value via `{:?}` formatting |
| `EncodeArc<T>` | Makes `Arc<T>` encodable as a label set if `T: EncodeLabelSet` |
| `OptionallyEncode<T>` | Wraps `Option<T>`, omits the entire label (not just value) when `None` |
| `CustomField` | Dynamic label set from `Arc<[(RichStrng, DefaultedUnknown<RichStrng>)]>` |

`DefaultedUnknown<RichStrng>` has special methods: `.display()` returns `Option<DisplayValue>` for tracing fields, `.to_value()` returns `Option<impl Value>`.

### Tokio Runtime Metrics (`tokio_metrics.rs`)

`TokioCollector` implements `prometheus_client::Collector` and exposes:
- `tokio_global_queue_depth` - Tasks in the runtime's global queue
- `tokio_num_alive_tasks` - Currently alive tasks
- `tokio_num_workers` - Worker thread count

Registered via `TokioCollector::register(registry, &runtime_handle)`.

## Relationships

- **Used by**: `crates/agentgateway` for request/connection/proxy metrics
- **Uses**: [[components/core/strng|strng]] for `RichStrng` in `DefaultedUnknown` and `CustomField`
- **Parent**: [[features/core|Core]]
