/**
 * Stateful pattern builders
 *
 * These patterns require external state stores and are not yet implemented
 * in the runtime. The IR is defined so compositions can be authored and
 * validated, with helpful errors when execution is attempted.
 */
import type { PatternSpec, StepOperation, ThrottleStrategy, OnExceeded, BackoffStrategy, FieldPredicate, DataBinding, OnDuplicate } from '../types.js';
export declare class Duration {
    private ms;
    private constructor();
    static millis(n: number): Duration;
    static seconds(n: number): Duration;
    static minutes(n: number): Duration;
    static hours(n: number): Duration;
    toMillis(): number;
    toSeconds(): number;
}
export declare const millis: typeof Duration.millis;
export declare const seconds: typeof Duration.seconds;
export declare const minutes: typeof Duration.minutes;
export declare const hours: typeof Duration.hours;
export declare function fixedBackoff(delay: Duration): BackoffStrategy;
export declare function exponentialBackoff(initialDelay: Duration, maxDelay: Duration, multiplier?: number): BackoffStrategy;
export declare function linearBackoff(initialDelay: Duration, increment: Duration, maxDelay: Duration): BackoffStrategy;
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
export declare function retry(config: RetryConfig): PatternSpec;
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
export declare function timeout(config: TimeoutConfig): PatternSpec;
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
export declare function cache(config: CacheConfig): PatternSpec;
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
export declare function idempotent(config: IdempotentConfig): PatternSpec;
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
export declare function circuitBreaker(config: CircuitBreakerConfig): PatternSpec;
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
export declare function deadLetter(config: DeadLetterConfig): PatternSpec;
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
export declare function saga(config: SagaConfig): PatternSpec;
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
export declare function claimCheck(config: ClaimCheckConfig): PatternSpec;
export interface ThrottleConfig {
    /** The tool or pattern to throttle */
    inner: StepOperation;
    /** Maximum requests per window */
    rate: number;
    /** Time window */
    window: Duration;
    /** Rate limiting strategy (default: sliding_window) */
    strategy?: ThrottleStrategy;
    /** Behavior when rate exceeded (default: wait) */
    onExceeded?: OnExceeded;
    /** Store for distributed throttling (optional for single-instance) */
    store?: string;
}
/**
 * Create a throttle (rate limiter) wrapper around a tool or pattern.
 *
 * @example
 * // Rate limit to 100 requests per minute
 * throttle({
 *   inner: { tool: { name: 'expensive_api' } },
 *   rate: 100,
 *   window: minutes(1),
 *   strategy: 'sliding_window',
 *   onExceeded: 'wait',
 * })
 *
 * @example
 * // Token bucket with 10 requests per second, reject on exceeded
 * throttle({
 *   inner: { tool: { name: 'rate_limited_api' } },
 *   rate: 10,
 *   window: seconds(1),
 *   strategy: 'token_bucket',
 *   onExceeded: 'reject',
 * })
 *
 * @example
 * // Distributed throttling with Redis store
 * throttle({
 *   inner: { tool: { name: 'shared_api' } },
 *   rate: 1000,
 *   window: minutes(1),
 *   store: 'redis_rate_limiter',
 * })
 */
export declare function throttle(config: ThrottleConfig): PatternSpec;
//# sourceMappingURL=stateful.d.ts.map