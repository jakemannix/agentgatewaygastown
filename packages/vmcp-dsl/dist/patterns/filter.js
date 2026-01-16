"use strict";
/**
 * Filter pattern builder
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.FilterBuilder = void 0;
exports.filter = filter;
exports.filterBy = filterBy;
const path_builder_js_1 = require("../path-builder.js");
/**
 * Builder for filter patterns
 */
class FilterBuilder {
    predicate = {};
    /**
     * Set the field to filter on
     */
    field(pathExpr) {
        this.predicate.field = (0, path_builder_js_1.path)(pathExpr);
        return this;
    }
    /**
     * Equal to
     */
    eq(value) {
        this.predicate.op = 'eq';
        this.predicate.value = toPredicateValue(value);
        return this;
    }
    /**
     * Not equal to
     */
    ne(value) {
        this.predicate.op = 'ne';
        this.predicate.value = toPredicateValue(value);
        return this;
    }
    /**
     * Greater than
     */
    gt(value) {
        this.predicate.op = 'gt';
        this.predicate.value = { numberValue: value };
        return this;
    }
    /**
     * Greater than or equal
     */
    gte(value) {
        this.predicate.op = 'gte';
        this.predicate.value = { numberValue: value };
        return this;
    }
    /**
     * Less than
     */
    lt(value) {
        this.predicate.op = 'lt';
        this.predicate.value = { numberValue: value };
        return this;
    }
    /**
     * Less than or equal
     */
    lte(value) {
        this.predicate.op = 'lte';
        this.predicate.value = { numberValue: value };
        return this;
    }
    /**
     * String contains
     */
    contains(value) {
        this.predicate.op = 'contains';
        this.predicate.value = { stringValue: value };
        return this;
    }
    /**
     * Value in list
     */
    in(values) {
        this.predicate.op = 'in';
        this.predicate.value = { listValue: values.map(toPredicateValue) };
        return this;
    }
    /**
     * Is null
     */
    isNull() {
        this.predicate.op = 'eq';
        this.predicate.value = { nullValue: true };
        return this;
    }
    /**
     * Is not null
     */
    isNotNull() {
        this.predicate.op = 'ne';
        this.predicate.value = { nullValue: true };
        return this;
    }
    /**
     * Build the filter pattern spec
     */
    build() {
        if (!this.predicate.field) {
            throw new Error('Field is required');
        }
        if (!this.predicate.op) {
            throw new Error('Operator is required');
        }
        if (!this.predicate.value) {
            throw new Error('Value is required');
        }
        return { filter: { predicate: this.predicate } };
    }
    /**
     * Get the raw spec
     */
    spec() {
        if (!this.predicate.field || !this.predicate.op || !this.predicate.value) {
            throw new Error('Incomplete filter predicate');
        }
        return { predicate: this.predicate };
    }
}
exports.FilterBuilder = FilterBuilder;
/**
 * Convert a primitive value to PredicateValue
 */
function toPredicateValue(value) {
    if (typeof value === 'string') {
        return { stringValue: value };
    }
    if (typeof value === 'number') {
        return { numberValue: value };
    }
    if (typeof value === 'boolean') {
        return { boolValue: value };
    }
    throw new Error(`Unsupported value type: ${typeof value}`);
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
function filter() {
    return new FilterBuilder();
}
/**
 * Shorthand for creating a simple filter
 *
 * @example
 * const spec = filterBy('$.score', 'gt', 0.5);
 */
function filterBy(field, op, value) {
    const builder = new FilterBuilder().field(field);
    switch (op) {
        case 'eq':
            return builder.eq(value).build();
        case 'ne':
            return builder.ne(value).build();
        case 'gt':
            return builder.gt(value).build();
        case 'gte':
            return builder.gte(value).build();
        case 'lt':
            return builder.lt(value).build();
        case 'lte':
            return builder.lte(value).build();
        case 'contains':
            return builder.contains(value).build();
        case 'in':
            return builder.in(value).build();
        default:
            throw new Error(`Unknown operator: ${op}`);
    }
}
//# sourceMappingURL=filter.js.map