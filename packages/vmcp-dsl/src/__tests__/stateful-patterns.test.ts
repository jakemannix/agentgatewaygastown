import { describe, it, expect } from 'vitest';
import {
  retry,
  timeout,
  cache,
  idempotent,
  circuitBreaker,
  deadLetter,
  saga,
  claimCheck,
  Duration,
  millis,
  seconds,
  minutes,
  hours,
  fixedBackoff,
  exponentialBackoff,
  linearBackoff,
} from '../patterns/stateful';

describe('Duration helpers', () => {
  it('should convert durations correctly', () => {
    expect(millis(500).toMillis()).toBe(500);
    expect(seconds(5).toMillis()).toBe(5000);
    expect(minutes(2).toMillis()).toBe(120000);
    expect(hours(1).toMillis()).toBe(3600000);

    expect(seconds(90).toSeconds()).toBe(90);
    expect(minutes(2).toSeconds()).toBe(120);
  });
});

describe('Backoff strategies', () => {
  it('should create fixed backoff', () => {
    const backoff = fixedBackoff(millis(100));
    expect(backoff).toEqual({ fixed: { delayMs: 100 } });
  });

  it('should create exponential backoff', () => {
    const backoff = exponentialBackoff(millis(100), seconds(5), 2);
    expect(backoff).toEqual({
      exponential: {
        initialDelayMs: 100,
        maxDelayMs: 5000,
        multiplier: 2,
      },
    });
  });

  it('should create linear backoff', () => {
    const backoff = linearBackoff(millis(100), millis(50), seconds(2));
    expect(backoff).toEqual({
      linear: {
        initialDelayMs: 100,
        incrementMs: 50,
        maxDelayMs: 2000,
      },
    });
  });
});

describe('Retry Pattern', () => {
  it('should create a retry with exponential backoff', () => {
    const spec = retry({
      inner: { tool: { name: 'unreliable_api' } },
      maxAttempts: 3,
      backoff: exponentialBackoff(millis(100), seconds(5), 2),
      jitter: 0.1,
    });

    expect(spec.retry).toBeDefined();
    expect(spec.retry?.maxAttempts).toBe(3);
    expect(spec.retry?.jitter).toBe(0.1);
    expect(spec.retry?.backoff).toEqual({
      exponential: {
        initialDelayMs: 100,
        maxDelayMs: 5000,
        multiplier: 2,
      },
    });
    expect(spec.retry?.inner).toEqual({ tool: { name: 'unreliable_api' } });
  });

  it('should support attempt timeout', () => {
    const spec = retry({
      inner: { tool: { name: 'slow_api' } },
      maxAttempts: 2,
      backoff: fixedBackoff(seconds(1)),
      attemptTimeout: seconds(10),
    });

    expect(spec.retry?.attemptTimeoutMs).toBe(10000);
  });
});

describe('Timeout Pattern', () => {
  it('should create a timeout wrapper', () => {
    const spec = timeout({
      inner: { tool: { name: 'slow_tool' } },
      duration: seconds(30),
    });

    expect(spec.timeout).toBeDefined();
    expect(spec.timeout?.durationMs).toBe(30000);
    expect(spec.timeout?.inner).toEqual({ tool: { name: 'slow_tool' } });
  });

  it('should support fallback and custom message', () => {
    const spec = timeout({
      inner: { tool: { name: 'slow_tool' } },
      duration: seconds(30),
      fallback: { tool: { name: 'cached_result' } },
      message: 'Operation timed out after 30 seconds',
    });

    expect(spec.timeout?.fallback).toEqual({ tool: { name: 'cached_result' } });
    expect(spec.timeout?.message).toBe('Operation timed out after 30 seconds');
  });
});

describe('Cache Pattern', () => {
  it('should create a cache wrapper', () => {
    const spec = cache({
      keyPaths: ['$.query', '$.filters.category'],
      inner: { tool: { name: 'web_search' } },
      store: 'result_cache',
      ttl: minutes(15),
    });

    expect(spec.cache).toBeDefined();
    expect(spec.cache?.keyPaths).toEqual(['$.query', '$.filters.category']);
    expect(spec.cache?.store).toBe('result_cache');
    expect(spec.cache?.ttlSeconds).toBe(900);
  });

  it('should support stale-while-revalidate', () => {
    const spec = cache({
      keyPaths: ['$.id'],
      inner: { tool: { name: 'expensive_lookup' } },
      store: 'cache',
      ttl: minutes(5),
      staleWhileRevalidate: minutes(1),
    });

    expect(spec.cache?.staleWhileRevalidateSeconds).toBe(60);
  });
});

describe('Idempotent Pattern', () => {
  it('should create an idempotent wrapper', () => {
    const spec = idempotent({
      keyPaths: ['$.documentId', '$.operation'],
      inner: { tool: { name: 'expensive_analysis' } },
      store: 'idempotency_store',
      ttl: hours(1),
    });

    expect(spec.idempotent).toBeDefined();
    expect(spec.idempotent?.keyPaths).toEqual(['$.documentId', '$.operation']);
    expect(spec.idempotent?.store).toBe('idempotency_store');
    expect(spec.idempotent?.ttlSeconds).toBe(3600);
    expect(spec.idempotent?.onDuplicate).toBe('cached');
  });

  it('should support different duplicate behaviors', () => {
    const spec = idempotent({
      keyPaths: ['$.requestId'],
      inner: { tool: { name: 'one_time_action' } },
      store: 'seen_requests',
      onDuplicate: 'error',
    });

    expect(spec.idempotent?.onDuplicate).toBe('error');
  });
});

describe('Circuit Breaker Pattern', () => {
  it('should create a circuit breaker', () => {
    const spec = circuitBreaker({
      name: 'payment_api',
      inner: { tool: { name: 'payment_service' } },
      store: 'circuit_state',
      failureThreshold: 5,
      failureWindow: minutes(1),
      resetTimeout: seconds(30),
    });

    expect(spec.circuitBreaker).toBeDefined();
    expect(spec.circuitBreaker?.name).toBe('payment_api');
    expect(spec.circuitBreaker?.failureThreshold).toBe(5);
    expect(spec.circuitBreaker?.failureWindowSeconds).toBe(60);
    expect(spec.circuitBreaker?.resetTimeoutSeconds).toBe(30);
  });

  it('should support fallback', () => {
    const spec = circuitBreaker({
      name: 'external_api',
      inner: { tool: { name: 'external_service' } },
      store: 'circuit_state',
      failureThreshold: 3,
      failureWindow: seconds(30),
      resetTimeout: seconds(15),
      fallback: { tool: { name: 'queue_for_later' } },
      successThreshold: 2,
    });

    expect(spec.circuitBreaker?.fallback).toEqual({ tool: { name: 'queue_for_later' } });
    expect(spec.circuitBreaker?.successThreshold).toBe(2);
  });
});

describe('Dead Letter Pattern', () => {
  it('should create a dead letter wrapper', () => {
    const spec = deadLetter({
      inner: { tool: { name: 'important_process' } },
      deadLetterTool: 'failed_jobs_queue',
      maxAttempts: 3,
      backoff: exponentialBackoff(millis(100), seconds(5)),
    });

    expect(spec.deadLetter).toBeDefined();
    expect(spec.deadLetter?.deadLetterTool).toBe('failed_jobs_queue');
    expect(spec.deadLetter?.maxAttempts).toBe(3);
  });

  it('should support rethrow option', () => {
    const spec = deadLetter({
      inner: { tool: { name: 'critical_operation' } },
      deadLetterTool: 'dlq',
      rethrow: true,
    });

    expect(spec.deadLetter?.rethrow).toBe(true);
  });
});

describe('Saga Pattern', () => {
  it('should create a saga with compensation', () => {
    const spec = saga({
      sagaIdPath: '$.orderId',
      steps: [
        {
          name: 'reserve_inventory',
          action: { tool: { name: 'reserve_inventory' } },
          compensate: { tool: { name: 'release_inventory' } },
        },
        {
          name: 'charge_payment',
          action: { tool: { name: 'charge_payment' } },
          compensate: { tool: { name: 'refund_payment' } },
        },
        {
          name: 'send_confirmation',
          action: { tool: { name: 'send_email' } },
          // No compensation - email sent is acceptable
        },
      ],
      store: 'saga_state',
      timeout: minutes(5),
    });

    expect(spec.saga).toBeDefined();
    expect(spec.saga?.steps).toHaveLength(3);
    expect(spec.saga?.sagaIdPath).toBe('$.orderId');
    expect(spec.saga?.timeoutMs).toBe(300000);
    expect(spec.saga?.store).toBe('saga_state');

    // Check step structure
    expect(spec.saga?.steps[0].id).toBe('saga_step_0');
    expect(spec.saga?.steps[0].name).toBe('reserve_inventory');
    expect(spec.saga?.steps[0].action).toEqual({ tool: { name: 'reserve_inventory' } });
    expect(spec.saga?.steps[0].compensate).toEqual({ tool: { name: 'release_inventory' } });

    // Last step has no compensation
    expect(spec.saga?.steps[2].compensate).toBeUndefined();
  });
});

describe('Claim Check Pattern', () => {
  it('should create a claim check wrapper', () => {
    const spec = claimCheck({
      storeTool: 'blob_store_put',
      retrieveTool: 'blob_store_get',
      inner: { tool: { name: 'process_metadata' } },
      retrieveAtEnd: true,
    });

    expect(spec.claimCheck).toBeDefined();
    expect(spec.claimCheck?.storeTool).toBe('blob_store_put');
    expect(spec.claimCheck?.retrieveTool).toBe('blob_store_get');
    expect(spec.claimCheck?.retrieveAtEnd).toBe(true);
  });
});

describe('Pattern Composition', () => {
  it('should compose retry with timeout', () => {
    // Retry wrapping a timeout-protected operation
    const spec = retry({
      inner: {
        pattern: timeout({
          inner: { tool: { name: 'slow_unreliable_api' } },
          duration: seconds(10),
        }),
      },
      maxAttempts: 3,
      backoff: exponentialBackoff(millis(100), seconds(2)),
    });

    expect(spec.retry).toBeDefined();
    expect(spec.retry?.inner.pattern?.timeout).toBeDefined();
  });

  it('should compose circuit breaker with retry', () => {
    // Circuit breaker protecting a retry-wrapped operation
    const spec = circuitBreaker({
      name: 'protected_api',
      inner: {
        pattern: retry({
          inner: { tool: { name: 'flaky_api' } },
          maxAttempts: 2,
          backoff: fixedBackoff(millis(100)),
        }),
      },
      store: 'circuit_state',
      failureThreshold: 5,
      failureWindow: minutes(1),
      resetTimeout: seconds(30),
    });

    expect(spec.circuitBreaker).toBeDefined();
    expect(spec.circuitBreaker?.inner.pattern?.retry).toBeDefined();
  });
});
