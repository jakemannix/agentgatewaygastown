/**
 * TypeScript types for the Cache pattern.
 *
 * These types mirror the Rust CacheSpec and related structures
 * defined in crates/agentgateway/src/stateful/cache.rs
 */

/**
 * Predicate for conditional caching.
 * The cache will only store results when the predicate evaluates to true.
 */
export interface CachePredicate {
  /** The field to check in the result (supports dot-notation paths) */
  field: string;
  /** The expected value for caching to occur */
  equals: unknown;
}

/**
 * Specification for the cache pattern.
 *
 * The cache pattern wraps an inner operation and caches its results
 * based on key paths derived from the input.
 */
export interface CacheSpec {
  /**
   * JSON paths to extract from the input to form the cache key.
   * Multiple paths will be concatenated with ":" separator.
   * Supports dot-notation for nested fields (e.g., "user.id").
   */
  keyPaths: string[];

  /** Time-to-live in seconds for cached values */
  ttlSeconds: number;

  /**
   * Optional predicate to determine if the result should be cached.
   * If not set, all successful results are cached.
   */
  cacheIf?: CachePredicate;

  /**
   * Optional stale-while-revalidate duration in seconds.
   * If set, stale values can be returned while revalidating in the background.
   */
  staleWhileRevalidateSeconds?: number;
}

/**
 * Builder class for creating CacheSpec configurations.
 *
 * @example
 * ```typescript
 * const spec = cache(['user.id', 'action'], 60)
 *   .cacheIf('status', 'success')
 *   .staleWhileRevalidate(30)
 *   .build();
 * ```
 */
export class CacheBuilder {
  private spec: CacheSpec;

  constructor(keyPaths: string[], ttlSeconds: number) {
    this.spec = {
      keyPaths,
      ttlSeconds,
    };
  }

  /**
   * Add a conditional cache predicate.
   * Results will only be cached if the specified field equals the expected value.
   */
  cacheIf(field: string, equals: unknown): CacheBuilder {
    this.spec.cacheIf = { field, equals };
    return this;
  }

  /**
   * Enable stale-while-revalidate behavior.
   * Stale values can be returned for the specified duration while revalidating.
   */
  staleWhileRevalidate(seconds: number): CacheBuilder {
    this.spec.staleWhileRevalidateSeconds = seconds;
    return this;
  }

  /** Build the final CacheSpec */
  build(): CacheSpec {
    return { ...this.spec };
  }
}

/**
 * Create a new cache specification builder.
 *
 * @param keyPaths - JSON paths to extract from input for cache key derivation
 * @param ttlSeconds - Time-to-live in seconds for cached values
 * @returns A CacheBuilder instance for fluent configuration
 *
 * @example
 * ```typescript
 * // Simple cache with 60 second TTL
 * const simpleCache = cache(['request.id'], 60).build();
 *
 * // Cache with conditional caching and SWR
 * const advancedCache = cache(['user.id', 'endpoint'], 300)
 *   .cacheIf('response.status', 200)
 *   .staleWhileRevalidate(60)
 *   .build();
 * ```
 */
export function cache(keyPaths: string[], ttlSeconds: number): CacheBuilder {
  return new CacheBuilder(keyPaths, ttlSeconds);
}

/**
 * Derive a cache key from an input object using the specified key paths.
 * This is a utility function that mirrors the Rust derive_cache_key behavior.
 *
 * @param keyPaths - JSON paths to extract
 * @param input - The input object to extract values from
 * @returns The derived cache key string
 * @throws Error if a required path is not found in the input
 */
export function deriveCacheKey(keyPaths: string[], input: Record<string, unknown>): string {
  const parts: string[] = [];

  for (const path of keyPaths) {
    const value = getJsonPath(input, path);
    if (value === undefined) {
      throw new Error(`Cache key path '${path}' not found in input`);
    }

    if (typeof value === 'string') {
      parts.push(value);
    } else if (typeof value === 'number' || typeof value === 'boolean') {
      parts.push(String(value));
    } else if (value === null) {
      parts.push('null');
    } else {
      // For arrays/objects, use JSON serialization
      parts.push(JSON.stringify(value));
    }
  }

  return parts.join(':');
}

/**
 * Get a value from an object using a dot-separated path.
 */
function getJsonPath(obj: Record<string, unknown>, path: string): unknown {
  const segments = path.split('.');
  let current: unknown = obj;

  for (const segment of segments) {
    if (current === null || current === undefined) {
      return undefined;
    }

    if (typeof current === 'object') {
      if (Array.isArray(current)) {
        const index = parseInt(segment, 10);
        if (isNaN(index)) {
          return undefined;
        }
        current = current[index];
      } else {
        current = (current as Record<string, unknown>)[segment];
      }
    } else {
      return undefined;
    }
  }

  return current;
}

/**
 * Evaluate a cache predicate against a result value.
 */
export function evaluatePredicate(predicate: CachePredicate, result: Record<string, unknown>): boolean {
  const value = getJsonPath(result, predicate.field);
  return JSON.stringify(value) === JSON.stringify(predicate.equals);
}
