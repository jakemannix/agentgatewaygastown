/**
 * TypeScript types for workflow patterns (Enterprise Integration Patterns).
 *
 * These types mirror the Rust IR types in crates/agentgateway/src/workflow/types.rs
 */

import { z } from "zod";

/**
 * Operators for field predicate comparisons.
 */
export enum PredicateOperator {
  /** Equals comparison */
  Eq = "eq",
  /** Not equals comparison */
  Neq = "neq",
  /** Greater than */
  Gt = "gt",
  /** Greater than or equal */
  Gte = "gte",
  /** Less than */
  Lt = "lt",
  /** Less than or equal */
  Lte = "lte",
  /** Contains (for strings or arrays) */
  Contains = "contains",
  /** Starts with (for strings) */
  StartsWith = "starts_with",
  /** Ends with (for strings) */
  EndsWith = "ends_with",
  /** Matches regex pattern */
  Matches = "matches",
  /** Value exists (is not null) */
  Exists = "exists",
  /** Value is in a list */
  In = "in",
}

/**
 * A field predicate that evaluates to true or false based on input data.
 */
export interface FieldPredicate {
  /** The field path to evaluate (e.g., "input.type", "data.status") */
  field: string;
  /** The operator for comparison */
  operator: PredicateOperator;
  /** The value to compare against */
  value: unknown;
}

/**
 * A step operation in a workflow.
 */
export type StepOperation =
  | { type: "passthrough" }
  | { type: "transform"; expression: string }
  | { type: "toolCall"; tool: string; args?: unknown }
  | { type: "router"; routes: RouteCase[]; otherwise?: StepOperation }
  | { type: "sequence"; steps: StepOperation[] }
  | { type: "parallel"; branches: StepOperation[] };

/**
 * Specification for a content router.
 */
export interface RouterSpec {
  /** The routes to evaluate, in order */
  routes: RouteCase[];
  /** Operation to execute if no route matches */
  otherwise?: StepOperation;
}

/**
 * A single route case with a predicate and target operation.
 */
export interface RouteCase {
  /** The condition that must be true for this route to match */
  when: FieldPredicate;
  /** The operation to execute when the condition matches */
  then: StepOperation;
}

// Zod schemas for validation

export const predicateOperatorSchema = z.nativeEnum(PredicateOperator);

export const fieldPredicateSchema = z.object({
  field: z.string().min(1),
  operator: predicateOperatorSchema,
  value: z.unknown(),
});

// Forward declaration for recursive types
export const stepOperationSchema: z.ZodType<StepOperation> = z.lazy(() =>
  z.discriminatedUnion("type", [
    z.object({ type: z.literal("passthrough") }),
    z.object({ type: z.literal("transform"), expression: z.string() }),
    z.object({
      type: z.literal("toolCall"),
      tool: z.string(),
      args: z.unknown().optional(),
    }),
    z.object({
      type: z.literal("router"),
      routes: z.array(routeCaseSchema),
      otherwise: stepOperationSchema.optional(),
    }),
    z.object({
      type: z.literal("sequence"),
      steps: z.array(stepOperationSchema),
    }),
    z.object({
      type: z.literal("parallel"),
      branches: z.array(stepOperationSchema),
    }),
  ])
);

export const routeCaseSchema = z.object({
  when: fieldPredicateSchema,
  then: stepOperationSchema,
});

export const routerSpecSchema = z.object({
  routes: z.array(routeCaseSchema),
  otherwise: stepOperationSchema.optional(),
});
