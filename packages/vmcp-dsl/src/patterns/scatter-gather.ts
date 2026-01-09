/**
 * Scatter-gather pattern builder
 */

import type {
  PatternSpec,
  ScatterGatherSpec,
  ScatterTarget,
  AggregationStrategy,
  AggregationOp,
} from '../types.js';

/**
 * Builder for aggregation strategies
 */
export class AggregationBuilder {
  private ops: AggregationOp[] = [];

  /**
   * Flatten nested arrays
   */
  flatten(): this {
    this.ops.push({ flatten: true });
    return this;
  }

  /**
   * Sort results by field
   */
  sort(field: string, order: 'asc' | 'desc' = 'asc'): this {
    this.ops.push({ sort: { field, order } });
    return this;
  }

  /**
   * Sort ascending
   */
  sortAsc(field: string): this {
    return this.sort(field, 'asc');
  }

  /**
   * Sort descending
   */
  sortDesc(field: string): this {
    return this.sort(field, 'desc');
  }

  /**
   * Deduplicate by field
   */
  dedupe(field: string): this {
    this.ops.push({ dedupe: { field } });
    return this;
  }

  /**
   * Limit results
   */
  limit(count: number): this {
    this.ops.push({ limit: { count } });
    return this;
  }

  /**
   * Concatenate arrays (no flattening)
   */
  concat(): this {
    this.ops.push({ concat: true });
    return this;
  }

  /**
   * Merge objects
   */
  merge(): this {
    this.ops.push({ merge: true });
    return this;
  }

  /**
   * Build the aggregation strategy
   */
  build(): AggregationStrategy {
    return { ops: this.ops };
  }
}

/**
 * Builder for scatter-gather patterns
 */
export class ScatterGatherBuilder {
  private _targets: ScatterTarget[] = [];
  private _aggregation: AggregationStrategy = { ops: [] };
  private _timeoutMs?: number;
  private _failFast: boolean = false;

  /**
   * Add a tool target
   */
  target(toolName: string): this {
    this._targets.push({ tool: toolName });
    return this;
  }

  /**
   * Add multiple tool targets
   */
  targets(...toolNames: string[]): this {
    for (const name of toolNames) {
      this._targets.push({ tool: name });
    }
    return this;
  }

  /**
   * Add a pattern target
   */
  targetPattern(spec: PatternSpec): this {
    this._targets.push({ pattern: spec });
    return this;
  }

  /**
   * Set aggregation using builder
   */
  aggregate(builder: AggregationBuilder): this {
    this._aggregation = builder.build();
    return this;
  }

  /**
   * Set aggregation strategy directly
   */
  aggregation(strategy: AggregationStrategy): this {
    this._aggregation = strategy;
    return this;
  }

  /**
   * Shorthand: flatten results
   */
  flatten(): this {
    this._aggregation.ops.push({ flatten: true });
    return this;
  }

  /**
   * Set timeout in milliseconds
   */
  timeout(ms: number): this {
    this._timeoutMs = ms;
    return this;
  }

  /**
   * Set fail fast behavior
   */
  failFast(value: boolean = true): this {
    this._failFast = value;
    return this;
  }

  /**
   * Fail immediately on first error (alias for failFast)
   */
  failOnError(): this {
    this._failFast = true;
    return this;
  }

  /**
   * Build the scatter-gather pattern spec
   */
  build(): PatternSpec {
    return {
      scatterGather: {
        targets: this._targets,
        aggregation: this._aggregation,
        timeoutMs: this._timeoutMs,
        failFast: this._failFast,
      },
    };
  }

  /**
   * Get the raw spec
   */
  spec(): ScatterGatherSpec {
    return {
      targets: this._targets,
      aggregation: this._aggregation,
      timeoutMs: this._timeoutMs,
      failFast: this._failFast,
    };
  }
}

/**
 * Create a scatter-gather pattern
 *
 * @example
 * const multiSearch = scatterGather()
 *   .targets('search_web', 'search_arxiv', 'search_wikipedia')
 *   .aggregate(agg().flatten().sortDesc('$.score').limit(10))
 *   .timeout(5000)
 *   .build();
 */
export function scatterGather(): ScatterGatherBuilder {
  return new ScatterGatherBuilder();
}

/**
 * Create an aggregation builder
 *
 * @example
 * const aggregation = agg()
 *   .flatten()
 *   .sortDesc('$.relevance')
 *   .dedupe('$.id')
 *   .limit(20)
 *   .build();
 */
export function agg(): AggregationBuilder {
  return new AggregationBuilder();
}

