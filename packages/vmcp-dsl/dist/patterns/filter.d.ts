/**
 * Filter pattern builder
 */
import type { PatternSpec, FilterSpec, PredicateOp } from '../types.js';
/**
 * Builder for filter patterns
 */
export declare class FilterBuilder {
    private predicate;
    /**
     * Set the field to filter on
     */
    field(pathExpr: string): this;
    /**
     * Equal to
     */
    eq(value: string | number | boolean): this;
    /**
     * Not equal to
     */
    ne(value: string | number | boolean): this;
    /**
     * Greater than
     */
    gt(value: number): this;
    /**
     * Greater than or equal
     */
    gte(value: number): this;
    /**
     * Less than
     */
    lt(value: number): this;
    /**
     * Less than or equal
     */
    lte(value: number): this;
    /**
     * String contains
     */
    contains(value: string): this;
    /**
     * Value in list
     */
    in(values: (string | number | boolean)[]): this;
    /**
     * Is null
     */
    isNull(): this;
    /**
     * Is not null
     */
    isNotNull(): this;
    /**
     * Build the filter pattern spec
     */
    build(): PatternSpec;
    /**
     * Get the raw spec
     */
    spec(): FilterSpec;
}
/**
 * Create a filter pattern
 *
 * @example
 * const highScoreFilter = filter()
 *   .field('$.score')
 *   .gt(0.7)
 *   .build();
 *
 * const typeFilter = filter()
 *   .field('$.type')
 *   .in(['pdf', 'html', 'doc'])
 *   .build();
 */
export declare function filter(): FilterBuilder;
/**
 * Shorthand for creating a simple filter
 *
 * @example
 * const spec = filterBy('$.score', 'gt', 0.5);
 */
export declare function filterBy(field: string, op: PredicateOp, value: string | number | boolean | (string | number | boolean)[]): PatternSpec;
//# sourceMappingURL=filter.d.ts.map