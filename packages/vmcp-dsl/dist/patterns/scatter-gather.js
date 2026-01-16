"use strict";
/**
 * Scatter-gather pattern builder
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.ScatterGatherBuilder = exports.AggregationBuilder = void 0;
exports.scatterGather = scatterGather;
exports.agg = agg;
/**
 * Builder for aggregation strategies
 */
class AggregationBuilder {
    ops = [];
    /**
     * Flatten nested arrays
     */
    flatten() {
        this.ops.push({ flatten: true });
        return this;
    }
    /**
     * Sort results by field
     */
    sort(field, order = 'asc') {
        this.ops.push({ sort: { field, order } });
        return this;
    }
    /**
     * Sort ascending
     */
    sortAsc(field) {
        return this.sort(field, 'asc');
    }
    /**
     * Sort descending
     */
    sortDesc(field) {
        return this.sort(field, 'desc');
    }
    /**
     * Deduplicate by field
     */
    dedupe(field) {
        this.ops.push({ dedupe: { field } });
        return this;
    }
    /**
     * Limit results
     */
    limit(count) {
        this.ops.push({ limit: { count } });
        return this;
    }
    /**
     * Concatenate arrays (no flattening)
     */
    concat() {
        this.ops.push({ concat: true });
        return this;
    }
    /**
     * Merge objects
     */
    merge() {
        this.ops.push({ merge: true });
        return this;
    }
    /**
     * Build the aggregation strategy
     */
    build() {
        return { ops: this.ops };
    }
}
exports.AggregationBuilder = AggregationBuilder;
/**
 * Builder for scatter-gather patterns
 */
class ScatterGatherBuilder {
    _targets = [];
    _aggregation = { ops: [] };
    _timeoutMs;
    _failFast = false;
    /**
     * Add a tool target
     */
    target(toolName) {
        this._targets.push({ tool: toolName });
        return this;
    }
    /**
     * Add multiple tool targets
     */
    targets(...toolNames) {
        for (const name of toolNames) {
            this._targets.push({ tool: name });
        }
        return this;
    }
    /**
     * Add a pattern target
     */
    targetPattern(spec) {
        this._targets.push({ pattern: spec });
        return this;
    }
    /**
     * Set aggregation using builder
     */
    aggregate(builder) {
        this._aggregation = builder.build();
        return this;
    }
    /**
     * Set aggregation strategy directly
     */
    aggregation(strategy) {
        this._aggregation = strategy;
        return this;
    }
    /**
     * Shorthand: flatten results
     */
    flatten() {
        this._aggregation.ops.push({ flatten: true });
        return this;
    }
    /**
     * Set timeout in milliseconds
     */
    timeout(ms) {
        this._timeoutMs = ms;
        return this;
    }
    /**
     * Set fail fast behavior
     */
    failFast(value = true) {
        this._failFast = value;
        return this;
    }
    /**
     * Fail immediately on first error (alias for failFast)
     */
    failOnError() {
        this._failFast = true;
        return this;
    }
    /**
     * Build the scatter-gather pattern spec
     */
    build() {
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
    spec() {
        return {
            targets: this._targets,
            aggregation: this._aggregation,
            timeoutMs: this._timeoutMs,
            failFast: this._failFast,
        };
    }
}
exports.ScatterGatherBuilder = ScatterGatherBuilder;
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
function scatterGather() {
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
function agg() {
    return new AggregationBuilder();
}
//# sourceMappingURL=scatter-gather.js.map