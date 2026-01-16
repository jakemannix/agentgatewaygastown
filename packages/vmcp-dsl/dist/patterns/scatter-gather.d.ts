/**
 * Scatter-gather pattern builder
 */
import type { PatternSpec, ScatterGatherSpec, AggregationStrategy } from '../types.js';
/**
 * Builder for aggregation strategies
 */
export declare class AggregationBuilder {
    private ops;
    /**
     * Flatten nested arrays
     */
    flatten(): this;
    /**
     * Sort results by field
     */
    sort(field: string, order?: 'asc' | 'desc'): this;
    /**
     * Sort ascending
     */
    sortAsc(field: string): this;
    /**
     * Sort descending
     */
    sortDesc(field: string): this;
    /**
     * Deduplicate by field
     */
    dedupe(field: string): this;
    /**
     * Limit results
     */
    limit(count: number): this;
    /**
     * Concatenate arrays (no flattening)
     */
    concat(): this;
    /**
     * Merge objects
     */
    merge(): this;
    /**
     * Build the aggregation strategy
     */
    build(): AggregationStrategy;
}
/**
 * Builder for scatter-gather patterns
 */
export declare class ScatterGatherBuilder {
    private _targets;
    private _aggregation;
    private _timeoutMs?;
    private _failFast;
    /**
     * Add a tool target
     */
    target(toolName: string): this;
    /**
     * Add multiple tool targets
     */
    targets(...toolNames: string[]): this;
    /**
     * Add a pattern target
     */
    targetPattern(spec: PatternSpec): this;
    /**
     * Set aggregation using builder
     */
    aggregate(builder: AggregationBuilder): this;
    /**
     * Set aggregation strategy directly
     */
    aggregation(strategy: AggregationStrategy): this;
    /**
     * Shorthand: flatten results
     */
    flatten(): this;
    /**
     * Set timeout in milliseconds
     */
    timeout(ms: number): this;
    /**
     * Set fail fast behavior
     */
    failFast(value?: boolean): this;
    /**
     * Fail immediately on first error (alias for failFast)
     */
    failOnError(): this;
    /**
     * Build the scatter-gather pattern spec
     */
    build(): PatternSpec;
    /**
     * Get the raw spec
     */
    spec(): ScatterGatherSpec;
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
export declare function scatterGather(): ScatterGatherBuilder;
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
export declare function agg(): AggregationBuilder;
//# sourceMappingURL=scatter-gather.d.ts.map