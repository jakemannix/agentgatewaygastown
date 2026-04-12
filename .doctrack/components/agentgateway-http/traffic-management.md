---
feature: agentgateway-http
type: component
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/http/retry/mod.rs
  - crates/agentgateway/src/http/retry/body.rs
  - crates/agentgateway/src/http/deadletter/mod.rs
  - crates/agentgateway/src/http/stateful.rs
  - crates/agentgateway/src/http/sessionpersistence.rs
  - crates/agentgateway/src/http/outlierdetection.rs
  - crates/agentgateway/src/http/idempotent.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Traffic Management

This component covers resilience patterns (retry, circuit breaker, dead letter, outlier detection), session persistence, and idempotent request handling.

## Retry (`retry/`)

### Policy (`retry/mod.rs`)

- **`Policy`** configures retry behavior: `attempts` (max retries, default 1), `backoff` (optional duration between attempts), `codes` (HTTP status codes that trigger retry)
- Status codes are deserialized from u16 values

### Replay Body (`retry/body.rs`)

- **`ReplayBody`** enables request body replay for retries. HTTP request bodies are consumed on first read; `ReplayBody` buffers the body so it can be re-sent on retry.
- Located in `retry/body.rs` (~13KB implementation)
- Handles streaming bodies, buffering frames as they arrive

## Dead Letter (`deadletter/`)

Captures failed requests after all retry attempts are exhausted:

- **`Policy`** configures: `dead_letter_tool` (destination), `max_attempts`, `backoff`, `rethrow` (whether to propagate the error or return null)
- `execute_with_dead_letter()` -- generic async function that:
  1. Attempts the operation up to `max_attempts` times with optional backoff
  2. On final failure, sends a payload to the dead letter handler containing: `original_input`, `error`, `attempts`, `timestamp`
  3. Returns `Value::Null` (if `rethrow=false`) or the error (if `rethrow=true`)
- Uses trait-based abstraction: `Executor` (the operation) and `DeadLetterHandler` (the side-channel)

## Circuit Breaker (`stateful.rs`)

State-machine circuit breaker implementation:

- **Three states**: `Closed` (normal), `Open` (rejecting), `HalfOpen` (testing recovery)
- **`CircuitBreakerConfig`** -- `failure_threshold`, `success_threshold`, `timeout` (duration before transitioning Open to HalfOpen), `name`
- **`CircuitState`** -- tracks `state`, `failure_count`, `success_count`, `last_failure_time`, `last_state_change`
- **`StateStore` trait** -- pluggable backend for circuit state. Ships with `InMemoryStateStore` (HashMap behind RwLock). Could be extended to Redis or other distributed stores.
- **`CircuitBreakerError`** -- `CircuitOpen` (with optional `retry_after`), `StateError`, `OperationFailed`
- State transitions: Closed -> Open (on failure_threshold), Open -> HalfOpen (on timeout), HalfOpen -> Closed (on success_threshold), HalfOpen -> Open (on failure)

## Session Persistence (`sessionpersistence.rs`)

Encrypts and encodes session state for sticky routing:

- **`SessionState`** enum: `HTTP` (backend socket address) or `MCP` (list of per-backend MCP session IDs)
- **`Encoder`**: `Base64` (URL-safe, no padding) or `Aes` (AES-256-GCM via `aws-lc-rs`)
  - AES mode: 32-byte hex key, random nonce prepended to ciphertext, base64-encoded output
  - Base64 mode: simple encoding without encryption (for non-sensitive environments)
- `encode()` serializes state to JSON, then encrypts/encodes
- `decode()` decrypts/decodes, then deserializes back to `SessionState`
- Used by the [[features/agentgateway-mcp|MCP]] layer to maintain session affinity across multiple backends

## Outlier Detection (`outlierdetection.rs`)

Tracks backend health and ejects unhealthy backends from the load balancing pool.

## Idempotent Consumer (`idempotent.rs`)

Ensures duplicate requests (identified by idempotency key) return cached responses rather than executing the operation twice. ~13KB implementation.

## Related

- [[features/agentgateway-http|HTTP Module]] -- parent feature
- [[components/agentgateway-http/authentication]] -- auth happens before traffic management
- [[features/agentgateway-proxy|Proxy]] -- executes retry/circuit breaker logic during proxying
- [[features/agentgateway-mcp|MCP]] -- MCP sessions use session persistence
