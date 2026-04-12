---
feature: core
type: component
files:
  - crates/core/src/trcng.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Tracing (OpenTelemetry / OTLP)

## Responsibility

Distributed tracing via OpenTelemetry with OTLP export, HTTP context propagation, baggage enrichment, and configurable custom span tag rules.

## Internal Logic

### Initialization

`init_tracer(Config)` sets up the full OTel pipeline:
1. Creates a composite propagator (Baggage + TraceContext W3C)
2. Builds an OTLP span exporter via gRPC (tonic)
3. Registers a custom `EnrichWithBaggageSpanProcessor` that copies baggage attributes onto spans at start time
4. Sets the global tracer provider with the "agentgateway" service name

### Context Propagation

- `extract_context_from_request(&HeaderMap)` - Extracts trace context from incoming HTTP headers using the global propagator
- `add_context_to_request(&mut HeaderMap, &Context)` - Injects trace context into outgoing headers, always adds `baggage: is_synthetic=true`

### Custom Tag Rules

`Config.tags` is a `HashMap<String, String>` loaded from config. Values starting with `@` are treated as claim lookups (e.g., `"user_id": "@jwt.sub"` looks up `jwt.sub` from the request context via the `Claim` trait). Plain values are inserted directly as span attributes.

### Span Creation

`start_span(name, &impl Claim)` creates a span builder with custom tag rules applied. The `Claim` trait abstracts access to request-scoped claims (JWT fields, headers, etc.).

### Key Types

```rust
pub struct Config {
    pub tracer: Tracer,       // Currently only Otlp { endpoint: Option<String> }
    pub tags: HashMap<String, String>,
}

pub trait Claim {
    fn get_claim(&self, key: &str) -> Option<&str>;
}
```

### Design Notes

- The tracer and tag rules are stored in `OnceLock` statics -- initialized once, immutable after
- Resource is also a singleton with service name "agentgateway"
- If no OTLP endpoint is configured, `init_tracer` is a no-op

## Relationships

- **Used by**: `crates/agentgateway` proxy module for request tracing
- **Related**: [[components/core/telemetry|telemetry]] (structured logging, separate concern)
- **Parent**: [[features/core|Core]]
