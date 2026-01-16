/**
 * Stateful pattern builders
 *
 * These patterns require external state stores and are not yet implemented
 * in the runtime. The IR is defined so compositions can be authored and
 * validated, with helpful errors when execution is attempted.
 */

import type {
  PatternSpec,
  StepOperation,
  RetrySpec,
  TimeoutSpec,
  CacheSpec,
  IdempotentSpec,
  CircuitBreakerSpec,
  DeadLetterSpec,
  SagaSpec,
  SagaStep,
  ClaimCheckSpec,
  BackoffStrategy,
  FieldPredicate,
  DataBinding,
  OnDuplicate,
} from '../types.js';

// =============================================================================
// Duration helpers
// =============================================================================

export class Duration {
  private constructor(private ms: number) {}

  static millis(n: number): Duration {
    return new Duration(n);
  }
  static seconds(n: number): Duration {
    return new Duration(n * 1000);
  }
  static minutes(n: number): Duration {
    return new Duration(n * 60 * 1000);
  }
  static hours(n: number): Duration {
    return new Duration(n * 60 * 60 * 1000);
  }

  toMillis(): number {
    return this.ms;
  }
  toSeconds(): number {
    return Math.floor(this.ms / 1000);
  }
}

export const millis = Duration.millis;
export const seconds = Duration.seconds;
export const minutes = Duration.minutes;
export const hours = Duration.hours;

// =============================================================================
// Backoff strategies
// =============================================================================

export function fixedBackoff(delay: Duration): BackoffStrategy {
  return { fixed: { delayMs: delay.toMillis() } };
}

export function exponentialBackoff(
  initialDelay: Duration,
  maxDelay: Duration,
  multiplier: number = 2
): BackoffStrategy {
  return {
    exponential: {
      initialDelayMs: initialDelay.toMillis(),
      maxDelayMs: maxDelay.toMillis(),
      multiplier,
    },
  };
}

export function linearBackoff(
  initialDelay: Duration,
  increment: Duration,
  maxDelay: Duration
): BackoffStrategy {
  return {
    linear: {
      initialDelayMs: initialDelay.toMillis(),
      incrementMs: increment.toMillis(),
      maxDelayMs: maxDelay.toMillis(),
    },
  };
}

// =============================================================================
// Retry pattern
// =============================================================================

export interface RetryConfig {
  /** The tool or pattern to retry */
  inner: StepOperation;
  /** Maximum number of attempts (including initial) */
  maxAttempts: number;
  /** Backoff strategy between attempts */
  backoff: BackoffStrategy;
  /** Condition to retry (if absent, retry all errors) */
  retryIf?: FieldPredicate;
  /** Jitter factor (0-1) to add randomness */
  jitter?: number;
  /** Per-attempt timeout */
  attemptTimeout?: Duration;
}

/**
 * Create a retry wrapper around a tool or pattern.
 *
 * @example
 * retry({
 *   inner: { tool: { name: 'unreliable_api' } },
 *   maxAttempts: 3,
 *   backoff: exponentialBackoff(millis(100), seconds(5), 2),
 *   jitter: 0.1,
 * })
 */
export function retry(config: RetryConfig): PatternSpec {
  const spec: RetrySpec = {
    inner: config.inner,
    maxAttempts: config.maxAttempts,
    backoff: config.backoff,
    retryIf: config.retryIf,
    jitter: config.jitter,
    attemptTimeoutMs: config.attemptTimeout?.toMillis(),
  };
  return { retry: spec };
}

// =============================================================================
// Timeout pattern
// =============================================================================

export interface TimeoutConfig {
  /** The tool or pattern to wrap */
  inner: StepOperation;
  /** Maximum duration before timeout */
  duration: Duration;
  /** Fallback on timeout (optional) */
  fallback?: StepOperation;
  /** Custom error message */
  message?: string;
}

/**
 * Create a timeout wrapper around a tool or pattern.
 *
 * @example
 * timeout({
 *   inner: { tool: { name: 'slow_tool' } },
 *   duration: seconds(30),
 *   fallback: { tool: { name: 'cached_fallback' } },
 * })
 */
export function timeout(config: TimeoutConfig): PatternSpec {
  const spec: TimeoutSpec = {
    inner: config.inner,
    durationMs: config.duration.toMillis(),
    fallback: config.fallback,
    message: config.message,
  };
  return { timeout: spec };
}

// =============================================================================
// Cache pattern
// =============================================================================

export interface CacheConfig {
  /** JSONPath expressions to derive cache key */
  keyPaths: string[];
  /** The tool or pattern to cache */
  inner: StepOperation;
  /** Store reference name (configured in gateway) */
  store: string;
  /** Time-to-live for cached entries */
  ttl: Duration;
  /** Stale-while-revalidate window */
  staleWhileRevalidate?: Duration;
  /** Condition to cache (if absent, always cache) */
  cacheIf?: FieldPredicate;
}

/**
 * Create a caching wrapper around a tool or pattern.
 *
 * @example
 * cache({
 *   keyPaths: ['$.query', '$.filters.category'],
 *   inner: { tool: { name: 'web_search' } },
 *   store: 'result_cache',
 *   ttl: minutes(15),
 * })
 */
export function cache(config: CacheConfig): PatternSpec {
  const spec: CacheSpec = {
    keyPaths: config.keyPaths,
    inner: config.inner,
    store: config.store,
    ttlSeconds: config.ttl.toSeconds(),
    staleWhileRevalidateSeconds: config.staleWhileRevalidate?.toSeconds(),
    cacheIf: config.cacheIf,
  };
  return { cache: spec };
}

// =============================================================================
// Idempotent pattern
// =============================================================================

export interface IdempotentConfig {
  /** JSONPath expressions to derive idempotency key */
  keyPaths: string[];
  /** The tool or pattern to wrap */
  inner: StepOperation;
  /** Store reference name (configured in gateway) */
  store: string;
  /** Time-to-live for seen-set entry */
  ttl?: Duration;
  /** Behavior on duplicate (default: 'cached') */
  onDuplicate?: OnDuplicate;
}

/**
 * Create an idempotent wrapper to prevent duplicate processing.
 *
 * @example
 * idempotent({
 *   keyPaths: ['$.documentId', '$.operation'],
 *   inner: { tool: { name: 'expensive_analysis' } },
 *   store: 'idempotency_store',
 *   ttl: hours(1),
 * })
 */
export function idempotent(config: IdempotentConfig): PatternSpec {
  const spec: IdempotentSpec = {
    keyPaths: config.keyPaths,
    inner: config.inner,
    store: config.store,
    ttlSeconds: config.ttl?.toSeconds(),
    onDuplicate: config.onDuplicate ?? 'cached',
  };
  return { idempotent: spec };
}

// =============================================================================
// Circuit Breaker pattern
// =============================================================================

export interface CircuitBreakerConfig {
  /** Unique name for this circuit (for state isolation) */
  name: string;
  /** The tool or pattern to protect */
  inner: StepOperation;
  /** Store for circuit state */
  store: string;
  /** Number of failures to trip the circuit */
  failureThreshold: number;
  /** Window for counting failures */
  failureWindow: Duration;
  /** Time to wait before half-open */
  resetTimeout: Duration;
  /** Successes needed in half-open to close (default: 1) */
  successThreshold?: number;
  /** Fallback when circuit is open */
  fallback?: StepOperation;
  /** Custom failure condition */
  failureIf?: FieldPredicate;
}

/**
 * Create a circuit breaker around a tool or pattern.
 *
 * @example
 * circuitBreaker({
 *   name: 'payment_api',
 *   inner: { tool: { name: 'payment_service' } },
 *   store: 'circuit_state',
 *   failureThreshold: 5,
 *   failureWindow: minutes(1),
 *   resetTimeout: seconds(30),
 *   fallback: { tool: { name: 'queue_for_later' } },
 * })
 */
export function circuitBreaker(config: CircuitBreakerConfig): PatternSpec {
  const spec: CircuitBreakerSpec = {
    name: config.name,
    inner: config.inner,
    store: config.store,
    failureThreshold: config.failureThreshold,
    failureWindowSeconds: config.failureWindow.toSeconds(),
    resetTimeoutSeconds: config.resetTimeout.toSeconds(),
    successThreshold: config.successThreshold,
    fallback: config.fallback,
    failureIf: config.failureIf,
  };
  return { circuitBreaker: spec };
}

// =============================================================================
// Dead Letter pattern
// =============================================================================

export interface DeadLetterConfig {
  /** The tool or pattern to wrap */
  inner: StepOperation;
  /** Tool to invoke on failure */
  deadLetterTool: string;
  /** Max attempts before dead-lettering (default: 1) */
  maxAttempts?: number;
  /** Backoff between attempts */
  backoff?: BackoffStrategy;
  /** Whether to rethrow after dead-lettering */
  rethrow?: boolean;
}

/**
 * Create a dead letter wrapper to capture failures.
 *
 * @example
 * deadLetter({
 *   inner: { tool: { name: 'important_process' } },
 *   deadLetterTool: 'failed_jobs_queue',
 *   maxAttempts: 3,
 *   backoff: exponentialBackoff(millis(100), seconds(5)),
 * })
 */
export function deadLetter(config: DeadLetterConfig): PatternSpec {
  const spec: DeadLetterSpec = {
    inner: config.inner,
    deadLetterTool: config.deadLetterTool,
    maxAttempts: config.maxAttempts,
    backoff: config.backoff,
    rethrow: config.rethrow,
  };
  return { deadLetter: spec };
}

// =============================================================================
// Saga pattern
// =============================================================================

export interface SagaStepConfig {
  /** Step name for tracing/logging */
  name: string;
  /** The action to perform */
  action: StepOperation;
  /** Compensating action if later steps fail */
  compensate?: StepOperation;
  /** Input binding for this step */
  input?: DataBinding;
}

export interface SagaConfig {
  /** Ordered list of saga steps */
  steps: SagaStepConfig[];
  /** Store for saga state (for recovery) */
  store?: string;
  /** JSONPath to derive saga instance ID */
  sagaIdPath?: string;
  /** Timeout for entire saga */
  timeout?: Duration;
  /** Output binding */
  output?: DataBinding;
}

/**
 * Create a saga (distributed transaction with compensation).
 *
 * @example
 * saga({
 *   sagaIdPath: '$.orderId',
 *   steps: [
 *     {
 *       name: 'reserve_inventory',
 *       action: { tool: { name: 'reserve_inventory' } },
 *       compensate: { tool: { name: 'release_inventory' } },
 *     },
 *     {
 *       name: 'charge_payment',
 *       action: { tool: { name: 'charge_payment' } },
 *       compensate: { tool: { name: 'refund_payment' } },
 *     },
 *   ],
 *   store: 'saga_state',
 *   timeout: minutes(5),
 * })
 */
export function saga(config: SagaConfig): PatternSpec {
  const steps: SagaStep[] = config.steps.map((step, idx) => ({
    id: `saga_step_${idx}`,
    name: step.name,
    action: step.action,
    compensate: step.compensate,
    input: step.input ?? { input: { path: '$' } },
  }));

  const spec: SagaSpec = {
    steps,
    store: config.store,
    sagaIdPath: config.sagaIdPath,
    timeoutMs: config.timeout?.toMillis(),
    output: config.output,
  };
  return { saga: spec };
}

// =============================================================================
// Claim Check pattern
// =============================================================================

export interface ClaimCheckConfig {
  /** Tool to store payload and return reference */
  storeTool: string;
  /** Tool to retrieve payload from reference */
  retrieveTool: string;
  /** Inner pattern operating on reference */
  inner: StepOperation;
  /** Whether to retrieve original at end */
  retrieveAtEnd?: boolean;
}

/**
 * Create a claim check pattern for large payload handling.
 *
 * @example
 * claimCheck({
 *   storeTool: 'blob_store_put',
 *   retrieveTool: 'blob_store_get',
 *   inner: { pattern: pipeline([...]) },
 *   retrieveAtEnd: true,
 * })
 */
export function claimCheck(config: ClaimCheckConfig): PatternSpec {
  const spec: ClaimCheckSpec = {
    storeTool: config.storeTool,
    retrieveTool: config.retrieveTool,
    inner: config.inner,
    retrieveAtEnd: config.retrieveAtEnd,
  };
  return { claimCheck: spec };
}
