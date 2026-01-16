/**
 * Builder utilities for workflow patterns (Enterprise Integration Patterns).
 *
 * Provides a fluent DSL for constructing workflow operations.
 */

import {
  FieldPredicate,
  PredicateOperator,
  RouteCase,
  RouterSpec,
  StepOperation,
} from "./workflow-types";

// ============================================================================
// Predicate Builders
// ============================================================================

/**
 * Creates a field predicate for equality comparison.
 */
export function fieldEq(field: string, value: unknown): FieldPredicate {
  return { field, operator: PredicateOperator.Eq, value };
}

/**
 * Creates a field predicate for inequality comparison.
 */
export function fieldNeq(field: string, value: unknown): FieldPredicate {
  return { field, operator: PredicateOperator.Neq, value };
}

/**
 * Creates a field predicate for greater than comparison.
 */
export function fieldGt(field: string, value: number): FieldPredicate {
  return { field, operator: PredicateOperator.Gt, value };
}

/**
 * Creates a field predicate for greater than or equal comparison.
 */
export function fieldGte(field: string, value: number): FieldPredicate {
  return { field, operator: PredicateOperator.Gte, value };
}

/**
 * Creates a field predicate for less than comparison.
 */
export function fieldLt(field: string, value: number): FieldPredicate {
  return { field, operator: PredicateOperator.Lt, value };
}

/**
 * Creates a field predicate for less than or equal comparison.
 */
export function fieldLte(field: string, value: number): FieldPredicate {
  return { field, operator: PredicateOperator.Lte, value };
}

/**
 * Creates a field predicate for contains check (strings or arrays).
 */
export function fieldContains(field: string, value: unknown): FieldPredicate {
  return { field, operator: PredicateOperator.Contains, value };
}

/**
 * Creates a field predicate for string prefix check.
 */
export function fieldStartsWith(field: string, prefix: string): FieldPredicate {
  return { field, operator: PredicateOperator.StartsWith, value: prefix };
}

/**
 * Creates a field predicate for string suffix check.
 */
export function fieldEndsWith(field: string, suffix: string): FieldPredicate {
  return { field, operator: PredicateOperator.EndsWith, value: suffix };
}

/**
 * Creates a field predicate for regex matching.
 */
export function fieldMatches(field: string, pattern: string): FieldPredicate {
  return { field, operator: PredicateOperator.Matches, value: pattern };
}

/**
 * Creates a field predicate for existence check.
 */
export function fieldExists(field: string): FieldPredicate {
  return { field, operator: PredicateOperator.Exists, value: null };
}

/**
 * Creates a field predicate for membership check.
 */
export function fieldIn(field: string, values: unknown[]): FieldPredicate {
  return { field, operator: PredicateOperator.In, value: values };
}

// ============================================================================
// Operation Builders
// ============================================================================

/**
 * Creates a passthrough operation that returns input unchanged.
 */
export function passthrough(): StepOperation {
  return { type: "passthrough" };
}

/**
 * Creates a transform operation with a CEL/template expression.
 */
export function transform(expression: string): StepOperation {
  return { type: "transform", expression };
}

/**
 * Creates a tool call operation.
 */
export function toolCall(tool: string, args?: unknown): StepOperation {
  return { type: "toolCall", tool, args };
}

/**
 * Creates a sequence of operations executed in order.
 */
export function sequence(...steps: StepOperation[]): StepOperation {
  return { type: "sequence", steps };
}

/**
 * Creates parallel operations executed concurrently.
 */
export function parallel(...branches: StepOperation[]): StepOperation {
  return { type: "parallel", branches };
}

// ============================================================================
// Router Builder (Fluent API)
// ============================================================================

/**
 * Builder for creating content routers with a fluent API.
 *
 * @example
 * ```typescript
 * const router = route()
 *   .when(fieldEq("type", "error"), toolCall("handleError"))
 *   .when(fieldGte("status.code", 400), transform("error_response"))
 *   .otherwise(passthrough())
 *   .build();
 * ```
 */
export class RouterBuilder {
  private routes: RouteCase[] = [];
  private otherwiseOp?: StepOperation;

  /**
   * Adds a route case to the router.
   *
   * @param predicate - The condition that must match
   * @param operation - The operation to execute when condition matches
   * @returns This builder for chaining
   */
  when(predicate: FieldPredicate, operation: StepOperation): RouterBuilder {
    this.routes.push({ when: predicate, then: operation });
    return this;
  }

  /**
   * Sets the fallback operation when no routes match.
   *
   * @param operation - The default operation
   * @returns This builder for chaining
   */
  otherwise(operation: StepOperation): RouterBuilder {
    this.otherwiseOp = operation;
    return this;
  }

  /**
   * Builds the router specification.
   *
   * @returns The complete RouterSpec
   */
  build(): RouterSpec {
    return {
      routes: this.routes,
      otherwise: this.otherwiseOp,
    };
  }

  /**
   * Builds the router as a StepOperation.
   *
   * @returns A StepOperation wrapping the router
   */
  buildOperation(): StepOperation {
    return {
      type: "router",
      routes: this.routes,
      otherwise: this.otherwiseOp,
    };
  }
}

/**
 * Creates a new router builder.
 *
 * @example
 * ```typescript
 * const errorRouter = route()
 *   .when(fieldEq("type", "error"), toolCall("logError"))
 *   .when(fieldEq("type", "warning"), toolCall("logWarning"))
 *   .otherwise(passthrough())
 *   .build();
 * ```
 */
export function route(): RouterBuilder {
  return new RouterBuilder();
}

// ============================================================================
// Convenience Helpers
// ============================================================================

/**
 * Creates a simple router from an object mapping values to operations.
 *
 * @param field - The field to switch on
 * @param cases - Object mapping field values to operations
 * @param defaultOp - Optional default operation
 *
 * @example
 * ```typescript
 * const typeRouter = switchOn("type", {
 *   error: toolCall("handleError"),
 *   success: toolCall("handleSuccess"),
 * }, passthrough());
 * ```
 */
export function switchOn(
  field: string,
  cases: Record<string, StepOperation>,
  defaultOp?: StepOperation
): RouterSpec {
  const builder = route();

  for (const [value, operation] of Object.entries(cases)) {
    builder.when(fieldEq(field, value), operation);
  }

  if (defaultOp) {
    builder.otherwise(defaultOp);
  }

  return builder.build();
}

/**
 * Creates a router that matches on numeric ranges.
 *
 * @param field - The field to check
 * @param ranges - Array of [min, max, operation] tuples
 * @param defaultOp - Optional default operation
 *
 * @example
 * ```typescript
 * const statusRouter = rangeSwitch("status.code", [
 *   [200, 299, transform("success_response")],
 *   [400, 499, transform("client_error_response")],
 *   [500, 599, transform("server_error_response")],
 * ], passthrough());
 * ```
 */
export function rangeSwitch(
  field: string,
  ranges: [number, number, StepOperation][],
  defaultOp?: StepOperation
): RouterSpec {
  const builder = route();

  // Sort ranges by min value descending so higher ranges match first
  const sortedRanges = [...ranges].sort((a, b) => b[0] - a[0]);

  for (const [min, , operation] of sortedRanges) {
    builder.when(fieldGte(field, min), operation);
  }

  if (defaultOp) {
    builder.otherwise(defaultOp);
  }

  return builder.build();
}
