---
type: reference
original_path: docs/DESIGN_STATEFUL_PATTERNS.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# Design: Stateful Patterns for Tool Algebra (imported)

Source: `docs/DESIGN_STATEFUL_PATTERNS.md`

Extends the TypeScript DSL design with patterns that require external state stores, timing, and failure tracking. **Status: Designed but NOT YET IMPLEMENTED in the Rust executor.**

## Store Abstraction
All stateful patterns reference a **store** by name, configured at gateway level (Redis, in-memory, etc.):
```yaml
stores:
  result_cache:
    type: redis
    url: redis://localhost:6379
  circuit_state:
    type: memory
```

## Five Stateful Patterns Designed

### 1. Idempotent
Prevent duplicate processing by tracking seen keys. Derives idempotency key from field paths, stores in external store with configurable TTL. Options on duplicate: `cached`, `skip`, `error`.

### 2. Cache (Read-Through)
Cache tool results with configurable TTL and key derivation. Supports `staleWhileRevalidate` for background refresh and conditional caching (`cacheIf` predicate).

### 3. Circuit Breaker
Fail-fast when a tool experiences errors. Three states: closed, open, half-open. Configurable failure threshold, failure window, reset timeout, success threshold. Supports fallback tool on open circuit.

### 4. Retry
Retry failed operations with configurable backoff strategies:
- Fixed delay
- Exponential backoff (with multiplier)
- Linear backoff (with increment)
Supports jitter, per-attempt timeout, and conditional retry (`retryIf` predicate).

### 5. Timeout
Enforce maximum duration for tool execution. Supports fallback on timeout and custom error messages.

## Key Design Decision
Each pattern follows the same `Pattern<I, O>` interface and `toIR()` serialization contract as the stateless patterns. They compose with other patterns — e.g., `retry(circuitBreaker(cache(tool)))`.

## Related Notes
- [[references/imported/DESIGN_TS_TO_RUNTIME|TS to Runtime Design]] — the three-layer architecture these extend
- [[references/imported/virtual-tools-vision|Virtual Tools Vision]] — lists these as "not yet implemented"
- [[references/imported/mcp-algebra|MCP Algebra]] — the conceptual patterns catalog
