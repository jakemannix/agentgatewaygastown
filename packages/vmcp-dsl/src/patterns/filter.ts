/**
 * Filter pattern builder
 */

import type {
  PatternSpec,
  FilterSpec,
  FieldPredicate,
  PredicateOp,
  PredicateValue,
} from '../types.js';
import { path } from '../path-builder.js';

/**
 * Builder for filter patterns
 */
export class FilterBuilder {
  private predicate: Partial<FieldPredicate> = {};

  /**
   * Set the field to filter on
   */
  field(pathExpr: string): this {
    this.predicate.field = path(pathExpr);
    return this;
  }

  /**
   * Equal to
   */
  eq(value: string | number | boolean): this {
    this.predicate.op = 'eq';
    this.predicate.value = toPredicateValue(value);
    return this;
  }

  /**
   * Not equal to
   */
  ne(value: string | number | boolean): this {
    this.predicate.op = 'ne';
    this.predicate.value = toPredicateValue(value);
    return this;
  }

  /**
   * Greater than
   */
  gt(value: number): this {
    this.predicate.op = 'gt';
    this.predicate.value = { numberValue: value };
    return this;
  }

  /**
   * Greater than or equal
   */
  gte(value: number): this {
    this.predicate.op = 'gte';
    this.predicate.value = { numberValue: value };
    return this;
  }

  /**
   * Less than
   */
  lt(value: number): this {
    this.predicate.op = 'lt';
    this.predicate.value = { numberValue: value };
    return this;
  }

  /**
   * Less than or equal
   */
  lte(value: number): this {
    this.predicate.op = 'lte';
    this.predicate.value = { numberValue: value };
    return this;
  }

  /**
   * String contains
   */
  contains(value: string): this {
    this.predicate.op = 'contains';
    this.predicate.value = { stringValue: value };
    return this;
  }

  /**
   * Value in list
   */
  in(values: (string | number | boolean)[]): this {
    this.predicate.op = 'in';
    this.predicate.value = { listValue: values.map(toPredicateValue) };
    return this;
  }

  /**
   * Is null
   */
  isNull(): this {
    this.predicate.op = 'eq';
    this.predicate.value = { nullValue: true };
    return this;
  }

  /**
   * Is not null
   */
  isNotNull(): this {
    this.predicate.op = 'ne';
    this.predicate.value = { nullValue: true };
    return this;
  }

  /**
   * Build the filter pattern spec
   */
  build(): PatternSpec {
    if (!this.predicate.field) {
      throw new Error('Field is required');
    }
    if (!this.predicate.op) {
      throw new Error('Operator is required');
    }
    if (!this.predicate.value) {
      throw new Error('Value is required');
    }
    return { filter: { predicate: this.predicate as FieldPredicate } };
  }

  /**
   * Get the raw spec
   */
  spec(): FilterSpec {
    if (!this.predicate.field || !this.predicate.op || !this.predicate.value) {
      throw new Error('Incomplete filter predicate');
    }
    return { predicate: this.predicate as FieldPredicate };
  }
}

/**
 * Convert a primitive value to PredicateValue
 */
function toPredicateValue(value: string | number | boolean): PredicateValue {
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
export function filter(): FilterBuilder {
  return new FilterBuilder();
}

/**
 * Shorthand for creating a simple filter
 *
 * @example
 * const spec = filterBy('$.score', 'gt', 0.5);
 */
export function filterBy(
  field: string,
  op: PredicateOp,
  value: string | number | boolean | (string | number | boolean)[]
): PatternSpec {
  const builder = new FilterBuilder().field(field);

  switch (op) {
    case 'eq':
      return builder.eq(value as string | number | boolean).build();
    case 'ne':
      return builder.ne(value as string | number | boolean).build();
    case 'gt':
      return builder.gt(value as number).build();
    case 'gte':
      return builder.gte(value as number).build();
    case 'lt':
      return builder.lt(value as number).build();
    case 'lte':
      return builder.lte(value as number).build();
    case 'contains':
      return builder.contains(value as string).build();
    case 'in':
      return builder.in(value as (string | number | boolean)[]).build();
    default:
      throw new Error(`Unknown operator: ${op}`);
  }
}

