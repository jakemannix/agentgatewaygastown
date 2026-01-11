# Design: Stateful Patterns for Tool Algebra

This document extends [DESIGN_TS_TO_RUNTIME.md](./DESIGN_TS_TO_RUNTIME.md) with stateful patterns that require external state stores, timing, and failure tracking.

---

## Overview

Stateful patterns differ from pure transformation patterns in that they:
1. **Maintain state** across invocations (caches, circuit state, seen-sets)
2. **Reference external stores** (Redis, in-memory, distributed)
3. **Have temporal semantics** (TTLs, timeouts, reset periods)

---

## Store Abstraction

All stateful patterns reference a **store** by name. Stores are configured at the gateway level:

```yaml
# agentgateway config
stores:
  result_cache:
    type: redis
    url: redis://localhost:6379
    prefix: "vmcp:cache:"

  circuit_state:
    type: memory
    # In-memory, per-instance (not distributed)

  idempotency_store:
    type: redis
    url: redis://localhost:6379
    prefix: "vmcp:idempotent:"
```

---

## Pattern 1: Idempotent

Prevent duplicate processing by tracking seen keys.

### TypeScript DSL

```typescript
// src/patterns/idempotent.ts

import { Pattern, Step, JsonValue } from '../types';
import { FieldPath, getPath } from './schema-map';

export interface IdempotentConfig<I, O> {
  /** Field path(s) to derive the idempotency key */
  key: FieldPath<string | number>[] | FieldPath<string | number>;
  /** The tool or pattern to wrap */
  inner: Step<I, O>;
  /** Store reference name (configured in gateway) */
  store: string;
  /** Time-to-live for the seen-set entry */
  ttl?: Duration;
  /** What to return on duplicate (default: cached result) */
  onDuplicate?: 'cached' | 'skip' | 'error';
}

export class Idempotent<I, O> implements Pattern<I, O> {
  readonly __pattern = 'idempotent';

  constructor(private config: IdempotentConfig<I, O>) {}

  toIR(): IdempotentIR {
    const keyPaths = Array.isArray(this.config.key)
      ? this.config.key.map(k => getPath(k))
      : [getPath(this.config.key)];

    return {
      pattern: 'idempotent',
      keyPaths,
      inner: stepToIR(this.config.inner),
      store: this.config.store,
      ttlSeconds: this.config.ttl?.toSeconds(),
      onDuplicate: this.config.onDuplicate ?? 'cached',
    };
  }
}

/**
 * Create an idempotent wrapper around a tool or pattern.
 *
 * Usage:
 *   const $ = path<AnalysisRequest>();
 *   idempotent({
 *     key: [$.documentId, $.operation],
 *     inner: expensiveAnalysisTool,
 *     store: 'idempotency_store',
 *     ttl: hours(1),
 *   })
 */
export function idempotent<I, O>(config: IdempotentConfig<I, O>): Idempotent<I, O> {
  return new Idempotent(config);
}
```

### Example Usage

```typescript
const $ = path<DocumentRequest>();

const deduplicatedAnalysis = idempotent({
  key: [$.documentId, $.analysisType],
  inner: expensiveAnalysisTool,
  store: 'idempotency_store',
  ttl: hours(1),
  onDuplicate: 'cached',
});
```

---

## Pattern 2: Cache (Read-Through)

Cache tool results with configurable TTL and key derivation.

### TypeScript DSL

```typescript
// src/patterns/cache.ts

export interface CacheConfig<I, O> {
  /** Field path(s) to derive the cache key */
  key: FieldPath<string | number>[] | FieldPath<string | number>;
  /** The tool or pattern to wrap */
  inner: Step<I, O>;
  /** Store reference name */
  store: string;
  /** Time-to-live for cached entries */
  ttl: Duration;
  /** Whether to refresh in background on near-expiry */
  staleWhileRevalidate?: Duration;
  /** Condition to skip caching (e.g., error responses) */
  cacheIf?: Predicate<O>;
}

export class Cache<I, O> implements Pattern<I, O> {
  readonly __pattern = 'cache';

  constructor(private config: CacheConfig<I, O>) {}

  toIR(): CacheIR {
    const keyPaths = Array.isArray(this.config.key)
      ? this.config.key.map(k => getPath(k))
      : [getPath(this.config.key)];

    return {
      pattern: 'cache',
      keyPaths,
      inner: stepToIR(this.config.inner),
      store: this.config.store,
      ttlSeconds: this.config.ttl.toSeconds(),
      staleWhileRevalidateSeconds: this.config.staleWhileRevalidate?.toSeconds(),
      cacheIf: this.config.cacheIf ? predicateToIR(this.config.cacheIf) : undefined,
    };
  }
}

/**
 * Create a caching wrapper around a tool or pattern.
 *
 * Usage:
 *   const $ = path<SearchQuery>();
 *   cache({
 *     key: $.query,
 *     inner: webSearchTool,
 *     store: 'result_cache',
 *     ttl: minutes(15),
 *   })
 */
export function cache<I, O>(config: CacheConfig<I, O>): Cache<I, O> {
  return new Cache(config);
}
```

### Example Usage

```typescript
const $ = path<SearchQuery>();

const cachedSearch = cache({
  key: [$.query, $.filters.category],
  inner: webSearchTool,
  store: 'result_cache',
  ttl: minutes(15),
  staleWhileRevalidate: minutes(5),
});
```

---

## Pattern 3: Circuit Breaker

Fail fast when a tool is experiencing errors, with automatic recovery.

### TypeScript DSL

```typescript
// src/patterns/circuit-breaker.ts

export type CircuitState = 'closed' | 'open' | 'half_open';

export interface CircuitBreakerConfig<I, O> {
  /** The tool or pattern to protect */
  inner: Step<I, O>;
  /** Store for circuit state (should be shared across instances for distributed) */
  store: string;
  /** Circuit name (for state isolation) */
  name: string;
  /** Number of failures before opening */
  failureThreshold: number;
  /** Window for counting failures */
  failureWindow: Duration;
  /** How long to stay open before trying half-open */
  resetTimeout: Duration;
  /** Number of successes in half-open to close */
  successThreshold?: number;
  /** Fallback when circuit is open */
  fallback?: Step<I, O>;
  /** What counts as a failure (default: any error) */
  failureIf?: Predicate<O>;
}

export class CircuitBreaker<I, O> implements Pattern<I, O> {
  readonly __pattern = 'circuit_breaker';

  constructor(private config: CircuitBreakerConfig<I, O>) {}

  toIR(): CircuitBreakerIR {
    return {
      pattern: 'circuit_breaker',
      name: this.config.name,
      inner: stepToIR(this.config.inner),
      store: this.config.store,
      failureThreshold: this.config.failureThreshold,
      failureWindowSeconds: this.config.failureWindow.toSeconds(),
      resetTimeoutSeconds: this.config.resetTimeout.toSeconds(),
      successThreshold: this.config.successThreshold ?? 1,
      fallback: this.config.fallback ? stepToIR(this.config.fallback) : undefined,
      failureIf: this.config.failureIf ? predicateToIR(this.config.failureIf) : undefined,
    };
  }
}

/**
 * Create a circuit breaker around a tool or pattern.
 *
 * Usage:
 *   circuitBreaker({
 *     name: 'external_api',
 *     inner: externalApiTool,
 *     store: 'circuit_state',
 *     failureThreshold: 5,
 *     failureWindow: minutes(1),
 *     resetTimeout: seconds(30),
 *     fallback: cachedFallbackTool,
 *   })
 */
export function circuitBreaker<I, O>(config: CircuitBreakerConfig<I, O>): CircuitBreaker<I, O> {
  return new CircuitBreaker(config);
}
```

### Example Usage

```typescript
const protectedApi = circuitBreaker({
  name: 'payment_api',
  inner: paymentApiTool,
  store: 'circuit_state',
  failureThreshold: 5,
  failureWindow: minutes(1),
  resetTimeout: seconds(30),
  fallback: queueForLaterTool,
});
```

---

## Pattern 4: Retry

Retry failed operations with configurable backoff.

### TypeScript DSL

```typescript
// src/patterns/retry.ts

export type BackoffStrategy =
  | { type: 'fixed'; delayMs: number }
  | { type: 'exponential'; initialDelayMs: number; maxDelayMs: number; multiplier: number }
  | { type: 'linear'; initialDelayMs: number; incrementMs: number; maxDelayMs: number };

export interface RetryConfig<I, O> {
  /** The tool or pattern to retry */
  inner: Step<I, O>;
  /** Maximum number of attempts (including initial) */
  maxAttempts: number;
  /** Backoff strategy between attempts */
  backoff: BackoffStrategy;
  /** Which errors to retry (default: all) */
  retryIf?: Predicate<Error>;
  /** Jitter factor (0-1) to add randomness to delays */
  jitter?: number;
  /** Timeout for each individual attempt */
  attemptTimeout?: Duration;
}

export class Retry<I, O> implements Pattern<I, O> {
  readonly __pattern = 'retry';

  constructor(private config: RetryConfig<I, O>) {}

  toIR(): RetryIR {
    return {
      pattern: 'retry',
      inner: stepToIR(this.config.inner),
      maxAttempts: this.config.maxAttempts,
      backoff: this.config.backoff,
      retryIf: this.config.retryIf ? predicateToIR(this.config.retryIf) : undefined,
      jitter: this.config.jitter,
      attemptTimeoutMs: this.config.attemptTimeout?.toMillis(),
    };
  }
}

/**
 * Create a retry wrapper around a tool or pattern.
 *
 * Usage:
 *   retry({
 *     inner: unreliableTool,
 *     maxAttempts: 3,
 *     backoff: { type: 'exponential', initialDelayMs: 100, maxDelayMs: 5000, multiplier: 2 },
 *   })
 */
export function retry<I, O>(config: RetryConfig<I, O>): Retry<I, O> {
  return new Retry(config);
}

// Convenience backoff builders
export const fixedBackoff = (delayMs: number): BackoffStrategy =>
  ({ type: 'fixed', delayMs });

export const exponentialBackoff = (
  initialDelayMs: number,
  maxDelayMs: number,
  multiplier: number = 2
): BackoffStrategy =>
  ({ type: 'exponential', initialDelayMs, maxDelayMs, multiplier });

export const linearBackoff = (
  initialDelayMs: number,
  incrementMs: number,
  maxDelayMs: number
): BackoffStrategy =>
  ({ type: 'linear', initialDelayMs, incrementMs, maxDelayMs });
```

### Example Usage

```typescript
const resilientFetch = retry({
  inner: webFetchTool,
  maxAttempts: 3,
  backoff: exponentialBackoff(100, 5000, 2),
  jitter: 0.1,
  attemptTimeout: seconds(10),
});
```

---

## Pattern 5: Timeout

Enforce a maximum duration for tool execution.

### TypeScript DSL

```typescript
// src/patterns/timeout.ts

export interface TimeoutConfig<I, O> {
  /** The tool or pattern to wrap */
  inner: Step<I, O>;
  /** Maximum duration before timeout */
  duration: Duration;
  /** Fallback on timeout (optional) */
  fallback?: Step<I, O>;
  /** Error message on timeout */
  message?: string;
}

export class Timeout<I, O> implements Pattern<I, O> {
  readonly __pattern = 'timeout';

  constructor(private config: TimeoutConfig<I, O>) {}

  toIR(): TimeoutIR {
    return {
      pattern: 'timeout',
      inner: stepToIR(this.config.inner),
      durationMs: this.config.duration.toMillis(),
      fallback: this.config.fallback ? stepToIR(this.config.fallback) : undefined,
      message: this.config.message,
    };
  }
}

/**
 * Create a timeout wrapper around a tool or pattern.
 *
 * Usage:
 *   timeout({
 *     inner: slowTool,
 *     duration: seconds(30),
 *     fallback: cachedResultTool,
 *   })
 */
export function timeout<I, O>(config: TimeoutConfig<I, O>): Timeout<I, O> {
  return new Timeout(config);
}
```

---

## Pattern 6: Dead Letter

Capture failed invocations for later inspection or reprocessing.

### TypeScript DSL

```typescript
// src/patterns/dead-letter.ts

export interface DeadLetterConfig<I, O> {
  /** The tool or pattern to wrap */
  inner: Step<I, O>;
  /** Tool to invoke on failure (receives original input + error) */
  deadLetter: ToolRef<DeadLetterPayload<I>, void>;
  /** Maximum retries before dead-lettering */
  maxAttempts?: number;
  /** Backoff between retries */
  backoff?: BackoffStrategy;
  /** Whether to rethrow after dead-lettering */
  rethrow?: boolean;
}

export interface DeadLetterPayload<I> {
  originalInput: I;
  error: string;
  errorType: string;
  timestamp: string;
  attempts: number;
}

export class DeadLetter<I, O> implements Pattern<I, O> {
  readonly __pattern = 'dead_letter';

  constructor(private config: DeadLetterConfig<I, O>) {}

  toIR(): DeadLetterIR {
    return {
      pattern: 'dead_letter',
      inner: stepToIR(this.config.inner),
      deadLetterTool: this.config.deadLetter.name,
      maxAttempts: this.config.maxAttempts ?? 1,
      backoff: this.config.backoff,
      rethrow: this.config.rethrow ?? false,
    };
  }
}

/**
 * Create a dead letter wrapper around a tool or pattern.
 *
 * Usage:
 *   deadLetter({
 *     inner: importantProcessingTool,
 *     deadLetter: failedJobsQueueTool,
 *     maxAttempts: 3,
 *     backoff: exponentialBackoff(100, 5000, 2),
 *   })
 */
export function deadLetter<I, O>(config: DeadLetterConfig<I, O>): DeadLetter<I, O> {
  return new DeadLetter(config);
}
```

---

## Pattern 7: Saga (Distributed Transaction)

Coordinate multi-step operations with compensating actions on failure.

### TypeScript DSL

```typescript
// src/patterns/saga.ts

export interface SagaStep<I, O> {
  /** Step name for tracing/logging */
  name: string;
  /** The action to perform */
  action: Step<I, O>;
  /** Compensating action if later steps fail (receives action's output) */
  compensate?: Step<O, void>;
  /** Input binding for this step */
  input?: DataBinding;
}

export interface SagaConfig<I, O> {
  /** Ordered list of saga steps */
  steps: SagaStep<any, any>[];
  /** Store for saga state (for recovery) */
  store?: string;
  /** Saga instance name derivation */
  sagaId?: FieldPath<string>;
  /** Timeout for entire saga */
  timeout?: Duration;
  /** What to return on successful completion */
  output?: DataBinding;
}

export class Saga<I, O> implements Pattern<I, O> {
  readonly __pattern = 'saga';

  constructor(private config: SagaConfig<I, O>) {}

  toIR(): SagaIR {
    return {
      pattern: 'saga',
      steps: this.config.steps.map((step, idx) => ({
        id: `saga_step_${idx}`,
        name: step.name,
        action: stepToIR(step.action),
        compensate: step.compensate ? stepToIR(step.compensate) : undefined,
        input: step.input ? dataBindingToIR(step.input) : { source: 'previous' },
      })),
      store: this.config.store,
      sagaIdPath: this.config.sagaId ? getPath(this.config.sagaId) : undefined,
      timeoutMs: this.config.timeout?.toMillis(),
      output: this.config.output ? dataBindingToIR(this.config.output) : undefined,
    };
  }
}

/**
 * Create a saga (distributed transaction with compensation).
 *
 * Usage:
 *   saga({
 *     steps: [
 *       { name: 'reserve', action: reserveInventoryTool, compensate: releaseInventoryTool },
 *       { name: 'charge', action: chargePaymentTool, compensate: refundPaymentTool },
 *       { name: 'ship', action: initiateShippingTool, compensate: cancelShippingTool },
 *     ],
 *     store: 'saga_state',
 *     timeout: minutes(5),
 *   })
 */
export function saga<I, O>(config: SagaConfig<I, O>): Saga<I, O> {
  return new Saga(config);
}
```

### Example Usage

```typescript
const $ = path<OrderRequest>();

const orderSaga = saga({
  sagaId: $.orderId,
  steps: [
    {
      name: 'reserve_inventory',
      action: reserveInventoryTool,
      compensate: releaseInventoryTool
    },
    {
      name: 'charge_payment',
      action: chargePaymentTool,
      compensate: refundPaymentTool
    },
    {
      name: 'initiate_shipping',
      action: initiateShippingTool,
      compensate: cancelShippingTool
    },
    {
      name: 'send_confirmation',
      action: sendConfirmationEmailTool,
      // No compensation - email already sent is acceptable
    },
  ],
  store: 'saga_state',
  timeout: minutes(5),
});
```

---

## Pattern 8: Claim Check

Store large payloads externally and pass references through the pipeline.

### TypeScript DSL

```typescript
// src/patterns/claim-check.ts

export interface ClaimCheckConfig<I, O, Ref = string> {
  /** Tool to store the payload and return a reference */
  store: ToolRef<I, Ref>;
  /** Tool to retrieve payload from reference */
  retrieve: ToolRef<Ref, I>;
  /** The pipeline to execute with the reference */
  inner: Step<Ref, O>;
  /** Whether to retrieve the original at the end */
  retrieveAtEnd?: boolean;
}

export class ClaimCheck<I, O> implements Pattern<I, O> {
  readonly __pattern = 'claim_check';

  constructor(private config: ClaimCheckConfig<I, O>) {}

  toIR(): ClaimCheckIR {
    return {
      pattern: 'claim_check',
      storeTool: this.config.store.name,
      retrieveTool: this.config.retrieve.name,
      inner: stepToIR(this.config.inner),
      retrieveAtEnd: this.config.retrieveAtEnd ?? false,
    };
  }
}

/**
 * Create a claim check pattern for large payload handling.
 *
 * Usage:
 *   claimCheck({
 *     store: blobStorePutTool,
 *     retrieve: blobStoreGetTool,
 *     inner: pipeline([
 *       extractMetadataTool,
 *       routingDecisionTool,
 *     ]),
 *   })
 */
export function claimCheck<I, O>(config: ClaimCheckConfig<I, O>): ClaimCheck<I, O> {
  return new ClaimCheck(config);
}
```

---

## IR Protobuf Definitions

```protobuf
// schema/vmcp-ir-stateful.proto

syntax = "proto3";
package vmcp.ir.v1;

import "vmcp-ir.proto";

// ============================================
// Stateful Pattern Specifications
// ============================================

// Extended PatternSpec with stateful patterns
message PatternSpec {
  oneof pattern {
    // Stateless (existing)
    PipelineSpec pipeline = 1;
    RouterSpec router = 2;
    ScatterGatherSpec scatter_gather = 3;
    FilterSpec filter = 4;
    EnricherSpec enricher = 5;
    SchemaMapSpec schema_map = 6;
    MapEachSpec map_each = 7;

    // Stateful (new)
    IdempotentSpec idempotent = 20;
    CacheSpec cache = 21;
    CircuitBreakerSpec circuit_breaker = 22;
    RetrySpec retry = 23;
    TimeoutSpec timeout = 24;
    DeadLetterSpec dead_letter = 25;
    SagaSpec saga = 26;
    ClaimCheckSpec claim_check = 27;
  }
}

// ============================================
// Idempotent Pattern
// ============================================

message IdempotentSpec {
  // JSONPath expressions to derive idempotency key
  repeated string key_paths = 1;

  // The wrapped tool or pattern
  StepOperation inner = 2;

  // Store reference name (configured in gateway)
  string store = 3;

  // TTL in seconds (0 = no expiry)
  uint32 ttl_seconds = 4;

  // Behavior on duplicate
  OnDuplicate on_duplicate = 5;
}

enum OnDuplicate {
  ON_DUPLICATE_UNSPECIFIED = 0;
  ON_DUPLICATE_CACHED = 1;      // Return cached result
  ON_DUPLICATE_SKIP = 2;        // Return null/empty
  ON_DUPLICATE_ERROR = 3;       // Return error
}

// ============================================
// Cache Pattern
// ============================================

message CacheSpec {
  // JSONPath expressions to derive cache key
  repeated string key_paths = 1;

  // The wrapped tool or pattern
  StepOperation inner = 2;

  // Store reference name
  string store = 3;

  // TTL in seconds
  uint32 ttl_seconds = 4;

  // Stale-while-revalidate window in seconds
  optional uint32 stale_while_revalidate_seconds = 5;

  // Condition to cache result (if absent, always cache)
  optional PredicateSpec cache_if = 6;
}

// ============================================
// Circuit Breaker Pattern
// ============================================

message CircuitBreakerSpec {
  // Unique name for this circuit (for state isolation)
  string name = 1;

  // The protected tool or pattern
  StepOperation inner = 2;

  // Store for circuit state
  string store = 3;

  // Number of failures to trip the circuit
  uint32 failure_threshold = 4;

  // Window for counting failures (seconds)
  uint32 failure_window_seconds = 5;

  // Time to wait before half-open (seconds)
  uint32 reset_timeout_seconds = 6;

  // Successes needed in half-open to close
  uint32 success_threshold = 7;

  // Fallback when circuit is open (optional)
  optional StepOperation fallback = 8;

  // Custom failure condition (if absent, any error)
  optional PredicateSpec failure_if = 9;
}

// ============================================
// Retry Pattern
// ============================================

message RetrySpec {
  // The tool or pattern to retry
  StepOperation inner = 1;

  // Maximum attempts (including initial)
  uint32 max_attempts = 2;

  // Backoff strategy
  BackoffStrategy backoff = 3;

  // Condition to retry (if absent, retry all errors)
  optional PredicateSpec retry_if = 4;

  // Jitter factor (0.0 - 1.0)
  optional float jitter = 5;

  // Per-attempt timeout in milliseconds
  optional uint32 attempt_timeout_ms = 6;
}

message BackoffStrategy {
  oneof strategy {
    FixedBackoff fixed = 1;
    ExponentialBackoff exponential = 2;
    LinearBackoff linear = 3;
  }
}

message FixedBackoff {
  uint32 delay_ms = 1;
}

message ExponentialBackoff {
  uint32 initial_delay_ms = 1;
  uint32 max_delay_ms = 2;
  float multiplier = 3;
}

message LinearBackoff {
  uint32 initial_delay_ms = 1;
  uint32 increment_ms = 2;
  uint32 max_delay_ms = 3;
}

// ============================================
// Timeout Pattern
// ============================================

message TimeoutSpec {
  // The tool or pattern to wrap
  StepOperation inner = 1;

  // Timeout duration in milliseconds
  uint32 duration_ms = 2;

  // Fallback on timeout (optional)
  optional StepOperation fallback = 3;

  // Custom error message
  optional string message = 4;
}

// ============================================
// Dead Letter Pattern
// ============================================

message DeadLetterSpec {
  // The tool or pattern to wrap
  StepOperation inner = 1;

  // Tool to invoke on failure
  string dead_letter_tool = 2;

  // Max attempts before dead-lettering
  uint32 max_attempts = 3;

  // Backoff between attempts
  optional BackoffStrategy backoff = 4;

  // Whether to rethrow after dead-lettering
  bool rethrow = 5;
}

// ============================================
// Saga Pattern
// ============================================

message SagaSpec {
  // Ordered list of saga steps
  repeated SagaStepDef steps = 1;

  // Store for saga state (for recovery)
  optional string store = 2;

  // JSONPath to derive saga instance ID
  optional string saga_id_path = 3;

  // Timeout for entire saga in milliseconds
  optional uint32 timeout_ms = 4;

  // Output binding
  optional DataBinding output = 5;
}

message SagaStepDef {
  // Step identifier
  string id = 1;

  // Human-readable name
  string name = 2;

  // The action to perform
  StepOperation action = 3;

  // Compensating action (optional)
  optional StepOperation compensate = 4;

  // Input binding for this step
  DataBinding input = 5;
}

// ============================================
// Claim Check Pattern
// ============================================

message ClaimCheckSpec {
  // Tool to store payload and return reference
  string store_tool = 1;

  // Tool to retrieve payload from reference
  string retrieve_tool = 2;

  // Inner pattern operating on reference
  StepOperation inner = 3;

  // Whether to retrieve original at end
  bool retrieve_at_end = 4;
}

// ============================================
// Predicate Specification (for conditions)
// ============================================

message PredicateSpec {
  // Field to evaluate
  string field = 1;

  // Comparison operator
  ComparisonOp op = 2;

  // Value to compare against
  LiteralValue value = 3;
}

enum ComparisonOp {
  COMPARISON_OP_UNSPECIFIED = 0;
  COMPARISON_OP_EQ = 1;
  COMPARISON_OP_NE = 2;
  COMPARISON_OP_GT = 3;
  COMPARISON_OP_GTE = 4;
  COMPARISON_OP_LT = 5;
  COMPARISON_OP_LTE = 6;
  COMPARISON_OP_CONTAINS = 7;
  COMPARISON_OP_IN = 8;
  COMPARISON_OP_IS_NULL = 9;
  COMPARISON_OP_IS_NOT_NULL = 10;
  COMPARISON_OP_IS_ERROR = 11;
}

message LiteralValue {
  oneof value {
    string string_value = 1;
    double number_value = 2;
    bool bool_value = 3;
    NullValue null_value = 4;
    ListValue list_value = 5;
  }
}

message NullValue {}

message ListValue {
  repeated LiteralValue values = 1;
}
```

---

## Duration Helper Types

```typescript
// src/duration.ts

export class Duration {
  private constructor(private ms: number) {}

  static millis(n: number): Duration { return new Duration(n); }
  static seconds(n: number): Duration { return new Duration(n * 1000); }
  static minutes(n: number): Duration { return new Duration(n * 60 * 1000); }
  static hours(n: number): Duration { return new Duration(n * 60 * 60 * 1000); }

  toMillis(): number { return this.ms; }
  toSeconds(): number { return Math.floor(this.ms / 1000); }
}

// Convenience exports
export const millis = Duration.millis;
export const seconds = Duration.seconds;
export const minutes = Duration.minutes;
export const hours = Duration.hours;
```

---

## Complete TypeScript Index

```typescript
// src/index.ts

// Types
export * from './types';
export * from './duration';

// Stateless patterns
export * from './patterns/pipeline';
export * from './patterns/router';
export * from './patterns/scatter-gather';
export * from './patterns/filter';
export * from './patterns/schema-map';
export * from './patterns/map-each';
export * from './patterns/enricher';

// Stateful patterns
export * from './patterns/idempotent';
export * from './patterns/cache';
export * from './patterns/circuit-breaker';
export * from './patterns/retry';
export * from './patterns/timeout';
export * from './patterns/dead-letter';
export * from './patterns/saga';
export * from './patterns/claim-check';

// Composition builder
export * from './composition';
export * from './compiler';
```

---

## Rust Runtime: IR Types and Stubs

### IR Types (Rust)

```rust
// src/mcp/compositions/ir_stateful.rs

use serde::{Deserialize, Serialize};

// ============================================
// Stateful Pattern Specs
// ============================================

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdempotentSpec {
    pub key_paths: Vec<String>,
    pub inner: Box<StepOperation>,
    pub store: String,
    #[serde(default)]
    pub ttl_seconds: Option<u32>,
    #[serde(default)]
    pub on_duplicate: OnDuplicate,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnDuplicate {
    #[default]
    Cached,
    Skip,
    Error,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheSpec {
    pub key_paths: Vec<String>,
    pub inner: Box<StepOperation>,
    pub store: String,
    pub ttl_seconds: u32,
    #[serde(default)]
    pub stale_while_revalidate_seconds: Option<u32>,
    #[serde(default)]
    pub cache_if: Option<PredicateSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerSpec {
    pub name: String,
    pub inner: Box<StepOperation>,
    pub store: String,
    pub failure_threshold: u32,
    pub failure_window_seconds: u32,
    pub reset_timeout_seconds: u32,
    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,
    #[serde(default)]
    pub fallback: Option<Box<StepOperation>>,
    #[serde(default)]
    pub failure_if: Option<PredicateSpec>,
}

fn default_success_threshold() -> u32 { 1 }

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrySpec {
    pub inner: Box<StepOperation>,
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    #[serde(default)]
    pub retry_if: Option<PredicateSpec>,
    #[serde(default)]
    pub jitter: Option<f32>,
    #[serde(default)]
    pub attempt_timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BackoffStrategy {
    Fixed { delay_ms: u32 },
    Exponential {
        initial_delay_ms: u32,
        max_delay_ms: u32,
        #[serde(default = "default_multiplier")]
        multiplier: f32,
    },
    Linear {
        initial_delay_ms: u32,
        increment_ms: u32,
        max_delay_ms: u32,
    },
}

fn default_multiplier() -> f32 { 2.0 }

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeoutSpec {
    pub inner: Box<StepOperation>,
    pub duration_ms: u32,
    #[serde(default)]
    pub fallback: Option<Box<StepOperation>>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadLetterSpec {
    pub inner: Box<StepOperation>,
    pub dead_letter_tool: String,
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default)]
    pub backoff: Option<BackoffStrategy>,
    #[serde(default)]
    pub rethrow: bool,
}

fn default_max_attempts() -> u32 { 1 }

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SagaSpec {
    pub steps: Vec<SagaStepDef>,
    #[serde(default)]
    pub store: Option<String>,
    #[serde(default)]
    pub saga_id_path: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
    #[serde(default)]
    pub output: Option<DataBinding>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SagaStepDef {
    pub id: String,
    pub name: String,
    pub action: StepOperation,
    #[serde(default)]
    pub compensate: Option<StepOperation>,
    pub input: DataBinding,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimCheckSpec {
    pub store_tool: String,
    pub retrieve_tool: String,
    pub inner: Box<StepOperation>,
    #[serde(default)]
    pub retrieve_at_end: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PredicateSpec {
    pub field: String,
    pub op: ComparisonOp,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    In,
    IsNull,
    IsNotNull,
    IsError,
}
```

### Extended PatternSpec Enum

```rust
// src/mcp/compositions/ir.rs (modified)

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PatternSpec {
    // Stateless (implemented)
    Pipeline(PipelineSpec),
    Router(RouterSpec),
    ScatterGather(ScatterGatherSpec),
    Filter(FilterSpec),
    Enricher(EnricherSpec),
    SchemaMap(SchemaMapSpec),
    MapEach(MapEachSpec),

    // Stateful (not yet implemented)
    Idempotent(IdempotentSpec),
    Cache(CacheSpec),
    CircuitBreaker(CircuitBreakerSpec),
    Retry(RetrySpec),
    Timeout(TimeoutSpec),
    DeadLetter(DeadLetterSpec),
    Saga(SagaSpec),
    ClaimCheck(ClaimCheckSpec),
}
```

### Pattern Executor with Helpful Error Messages

```rust
// src/mcp/compositions/executor.rs

use super::ir::*;
use super::error::CompositionError;
use anyhow::Result;

impl CompositionExecutor {
    pub async fn execute(
        &self,
        composition_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let registry = self.registry.read().await;
        let composition = registry
            .get_composition(composition_name)
            .ok_or_else(|| anyhow::anyhow!("Composition not found: {}", composition_name))?;

        let mut ctx = ExecutionContext::new(input, composition.clone());

        self.execute_pattern(&composition.spec, &mut ctx).await
    }

    async fn execute_pattern(
        &self,
        spec: &PatternSpec,
        ctx: &mut ExecutionContext,
    ) -> Result<serde_json::Value> {
        match spec {
            // Stateless patterns (implemented)
            PatternSpec::Pipeline(s) => self.patterns.pipeline.execute(s, ctx, self).await,
            PatternSpec::Router(s) => self.patterns.router.execute(s, ctx, self).await,
            PatternSpec::ScatterGather(s) => self.patterns.scatter_gather.execute(s, ctx, self).await,
            PatternSpec::Filter(s) => self.patterns.filter.execute(s, ctx, self).await,
            PatternSpec::Enricher(s) => self.patterns.enricher.execute(s, ctx, self).await,
            PatternSpec::SchemaMap(s) => self.patterns.schema_map.execute(s, ctx, self).await,
            PatternSpec::MapEach(s) => self.patterns.map_each.execute(s, ctx, self).await,

            // Stateful patterns (not yet implemented - return helpful errors)
            PatternSpec::Idempotent(s) => Err(not_implemented_error("idempotent", s)),
            PatternSpec::Cache(s) => Err(not_implemented_error("cache", s)),
            PatternSpec::CircuitBreaker(s) => Err(not_implemented_error("circuit_breaker", s)),
            PatternSpec::Retry(s) => Err(not_implemented_error("retry", s)),
            PatternSpec::Timeout(s) => Err(not_implemented_error("timeout", s)),
            PatternSpec::DeadLetter(s) => Err(not_implemented_error("dead_letter", s)),
            PatternSpec::Saga(s) => Err(not_implemented_error("saga", s)),
            PatternSpec::ClaimCheck(s) => Err(not_implemented_error("claim_check", s)),
        }
    }
}

/// Generate a helpful error message for unimplemented stateful patterns.
fn not_implemented_error<T: std::fmt::Debug>(pattern_name: &str, spec: &T) -> anyhow::Error {
    CompositionError::PatternNotImplemented {
        pattern: pattern_name.to_string(),
        message: format!(
            "The '{}' pattern is defined in the IR but not yet implemented in the runtime.\n\n\
            This is a stateful pattern that requires:\n\
            {}\n\n\
            Spec received:\n{:#?}\n\n\
            To track implementation progress, see: \
            https://github.com/agentgateway/agentgateway/issues/XXX",
            pattern_name,
            pattern_requirements(pattern_name),
            spec
        ),
    }.into()
}

/// Return human-readable requirements for each stateful pattern.
fn pattern_requirements(pattern: &str) -> &'static str {
    match pattern {
        "idempotent" => "\
  - A configured state store (Redis, memory, etc.) for tracking seen keys
  - JSONPath evaluation for key extraction from input
  - TTL management for key expiry
  - Configuration: Add a 'stores' section to your gateway config",

        "cache" => "\
  - A configured cache store (Redis, memory, etc.)
  - JSONPath evaluation for cache key derivation
  - TTL and stale-while-revalidate semantics
  - Optional conditional caching based on response predicates
  - Configuration: Add a 'stores' section to your gateway config",

        "circuit_breaker" => "\
  - A configured state store for circuit state (failure counts, state transitions)
  - Failure counting within sliding windows
  - State machine: CLOSED -> OPEN -> HALF_OPEN -> CLOSED
  - Fallback execution when circuit is open
  - Configuration: Add a 'stores' section to your gateway config",

        "retry" => "\
  - Backoff calculation (fixed, exponential, linear)
  - Jitter application for thundering herd prevention
  - Per-attempt timeout enforcement
  - Conditional retry based on error predicates
  - No external store required (stateless within single invocation)",

        "timeout" => "\
  - Tokio timeout wrapper around inner pattern execution
  - Optional fallback execution on timeout
  - Custom error message generation
  - No external store required",

        "dead_letter" => "\
  - Retry logic with backoff (similar to retry pattern)
  - Dead letter tool invocation with structured payload
  - Payload includes: original input, error details, attempt count, timestamp
  - Optional rethrow after dead-lettering",

        "saga" => "\
  - Saga state persistence for crash recovery
  - Ordered step execution with output threading
  - Compensation execution in reverse order on failure
  - Saga instance ID tracking for debugging
  - Configuration: Add a 'stores' section for saga state persistence",

        "claim_check" => "\
  - Store tool invocation to persist large payload
  - Reference threading through inner pattern
  - Retrieve tool invocation to restore payload
  - No external store configuration (uses specified tools)",

        _ => "Unknown pattern requirements",
    }
}
```

### Error Types

```rust
// src/mcp/compositions/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompositionError {
    #[error("Composition not found: {name}")]
    NotFound { name: String },

    #[error("Pattern '{pattern}' is not yet implemented.\n\n{message}")]
    PatternNotImplemented {
        pattern: String,
        message: String,
    },

    #[error("Store '{store}' not configured. Stateful patterns require store configuration.\n\
             Add to your gateway config:\n\
             stores:\n  {store}:\n    type: redis\n    url: redis://localhost:6379")]
    StoreNotConfigured { store: String },

    #[error("Invalid JSONPath '{path}': {reason}")]
    InvalidJsonPath { path: String, reason: String },

    #[error("Pattern execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Saga step '{step}' failed, compensation required: {reason}")]
    SagaStepFailed { step: String, reason: String },

    #[error("Circuit '{name}' is OPEN. Requests are being rejected. \
             Reset timeout: {reset_timeout_seconds}s")]
    CircuitOpen {
        name: String,
        reset_timeout_seconds: u32,
    },

    #[error("Timeout after {duration_ms}ms waiting for pattern execution")]
    Timeout { duration_ms: u32 },

    #[error("Max retries ({max_attempts}) exhausted. Last error: {last_error}")]
    RetriesExhausted {
        max_attempts: u32,
        last_error: String,
    },

    #[error("Duplicate request detected (key: {key}). \
             Policy: {policy:?}")]
    DuplicateRequest {
        key: String,
        policy: OnDuplicate,
    },
}
```

### Example Error Output

When a user tries to use an unimplemented pattern, they'll see:

```
Error: Pattern 'circuit_breaker' is not yet implemented.

The 'circuit_breaker' pattern is defined in the IR but not yet implemented in the runtime.

This is a stateful pattern that requires:
  - A configured state store for circuit state (failure counts, state transitions)
  - Failure counting within sliding windows
  - State machine: CLOSED -> OPEN -> HALF_OPEN -> CLOSED
  - Fallback execution when circuit is open
  - Configuration: Add a 'stores' section to your gateway config

Spec received:
CircuitBreakerSpec {
    name: "payment_api",
    inner: Tool { name: "payment_service" },
    store: "circuit_state",
    failure_threshold: 5,
    failure_window_seconds: 60,
    reset_timeout_seconds: 30,
    success_threshold: 1,
    fallback: Some(Tool { name: "queue_for_later" }),
    failure_if: None,
}

To track implementation progress, see: https://github.com/agentgateway/agentgateway/issues/XXX
```

---

## Implementation Roadmap

| Pattern | Complexity | Dependencies | Priority |
|---------|------------|--------------|----------|
| **Timeout** | Low | None | P0 - Enables basic resilience |
| **Retry** | Medium | Backoff calculation | P0 - Enables basic resilience |
| **Cache** | Medium | Store abstraction | P1 - High value |
| **Idempotent** | Medium | Store abstraction | P1 - Prevents duplicate work |
| **CircuitBreaker** | High | Store + state machine | P2 - Advanced resilience |
| **DeadLetter** | Medium | Retry + tool invocation | P2 - Observability |
| **ClaimCheck** | Low | Tool invocation | P2 - Large payload handling |
| **Saga** | High | Store + compensation logic | P3 - Complex workflows |

### Store Abstraction (Required for P1+)

```rust
// src/mcp/compositions/store.rs

use async_trait::async_trait;

#[async_trait]
pub trait CompositionStore: Send + Sync {
    /// Get a value by key
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Set a value with optional TTL
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()>;

    /// Delete a key
    async fn delete(&self, key: &str) -> Result<()>;

    /// Check if key exists
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Increment a counter (for circuit breaker failure counts)
    async fn incr(&self, key: &str, ttl: Option<Duration>) -> Result<u64>;
}

// Implementations: MemoryStore, RedisStore, etc.
```
