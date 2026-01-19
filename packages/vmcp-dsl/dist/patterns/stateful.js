"use strict";
/**
 * Stateful pattern builders
 *
 * These patterns require external state stores and are not yet implemented
 * in the runtime. The IR is defined so compositions can be authored and
 * validated, with helpful errors when execution is attempted.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.hours = exports.minutes = exports.seconds = exports.millis = exports.Duration = void 0;
exports.fixedBackoff = fixedBackoff;
exports.exponentialBackoff = exponentialBackoff;
exports.linearBackoff = linearBackoff;
exports.retry = retry;
exports.timeout = timeout;
exports.cache = cache;
exports.idempotent = idempotent;
exports.circuitBreaker = circuitBreaker;
exports.deadLetter = deadLetter;
exports.saga = saga;
exports.claimCheck = claimCheck;
exports.throttle = throttle;
// =============================================================================
// Duration helpers
// =============================================================================
class Duration {
    ms;
    constructor(ms) {
        this.ms = ms;
    }
    static millis(n) {
        return new Duration(n);
    }
    static seconds(n) {
        return new Duration(n * 1000);
    }
    static minutes(n) {
        return new Duration(n * 60 * 1000);
    }
    static hours(n) {
        return new Duration(n * 60 * 60 * 1000);
    }
    toMillis() {
        return this.ms;
    }
    toSeconds() {
        return Math.floor(this.ms / 1000);
    }
}
exports.Duration = Duration;
exports.millis = Duration.millis;
exports.seconds = Duration.seconds;
exports.minutes = Duration.minutes;
exports.hours = Duration.hours;
// =============================================================================
// Backoff strategies
// =============================================================================
function fixedBackoff(delay) {
    return { fixed: { delayMs: delay.toMillis() } };
}
function exponentialBackoff(initialDelay, maxDelay, multiplier = 2) {
    return {
        exponential: {
            initialDelayMs: initialDelay.toMillis(),
            maxDelayMs: maxDelay.toMillis(),
            multiplier,
        },
    };
}
function linearBackoff(initialDelay, increment, maxDelay) {
    return {
        linear: {
            initialDelayMs: initialDelay.toMillis(),
            incrementMs: increment.toMillis(),
            maxDelayMs: maxDelay.toMillis(),
        },
    };
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
function retry(config) {
    const spec = {
        inner: config.inner,
        maxAttempts: config.maxAttempts,
        backoff: config.backoff,
        retryIf: config.retryIf,
        jitter: config.jitter,
        attemptTimeoutMs: config.attemptTimeout?.toMillis(),
    };
    return { retry: spec };
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
function timeout(config) {
    const spec = {
        inner: config.inner,
        durationMs: config.duration.toMillis(),
        fallback: config.fallback,
        message: config.message,
    };
    return { timeout: spec };
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
function cache(config) {
    const spec = {
        keyPaths: config.keyPaths,
        inner: config.inner,
        store: config.store,
        ttlSeconds: config.ttl.toSeconds(),
        staleWhileRevalidateSeconds: config.staleWhileRevalidate?.toSeconds(),
        cacheIf: config.cacheIf,
    };
    return { cache: spec };
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
function idempotent(config) {
    const spec = {
        keyPaths: config.keyPaths,
        inner: config.inner,
        store: config.store,
        ttlSeconds: config.ttl?.toSeconds(),
        onDuplicate: config.onDuplicate ?? 'cached',
    };
    return { idempotent: spec };
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
function circuitBreaker(config) {
    const spec = {
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
function deadLetter(config) {
    const spec = {
        inner: config.inner,
        deadLetterTool: config.deadLetterTool,
        maxAttempts: config.maxAttempts,
        backoff: config.backoff,
        rethrow: config.rethrow,
    };
    return { deadLetter: spec };
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
function saga(config) {
    const steps = config.steps.map((step, idx) => ({
        id: `saga_step_${idx}`,
        name: step.name,
        action: step.action,
        compensate: step.compensate,
        input: step.input ?? { input: { path: '$' } },
    }));
    const spec = {
        steps,
        store: config.store,
        sagaIdPath: config.sagaIdPath,
        timeoutMs: config.timeout?.toMillis(),
        output: config.output,
    };
    return { saga: spec };
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
function claimCheck(config) {
    const spec = {
        storeTool: config.storeTool,
        retrieveTool: config.retrieveTool,
        inner: config.inner,
        retrieveAtEnd: config.retrieveAtEnd,
    };
    return { claimCheck: spec };
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
function throttle(config) {
    const spec = {
        inner: config.inner,
        rate: config.rate,
        windowMs: config.window.toMillis(),
        strategy: config.strategy,
        onExceeded: config.onExceeded,
        store: config.store,
    };
    return { throttle: spec };
}
//# sourceMappingURL=stateful.js.map