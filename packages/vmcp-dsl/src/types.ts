/**
 * Core type definitions for vMCP tool compositions
 * These types correspond to the registry.proto schema
 */

// =============================================================================
// Registry and Tool Definitions
// =============================================================================

/** Registry containing tool definitions */
export interface Registry {
  schemaVersion: string;
  tools: ToolDefinition[];
}

/** A tool definition - either source-based or composition */
export interface ToolDefinition {
  name: string;
  description?: string;
  implementation: ToolImplementation;
  inputSchema?: JSONSchema;
  outputTransform?: OutputTransform;
  version?: string;
  metadata?: Record<string, unknown>;
}

/** Tool implementation - either source or spec */
export type ToolImplementation =
  | { source: SourceTool }
  | { spec: PatternSpec };

/** Source tool - 1:1 mapping to a backend tool */
export interface SourceTool {
  target: string;
  tool: string;
  defaults?: Record<string, unknown>;
  hideFields?: string[];
}

// =============================================================================
// Pattern Specifications
// =============================================================================

/** Pattern specification - one of the supported patterns */
export type PatternSpec =
  // Stateless patterns (implemented)
  | { pipeline: PipelineSpec }
  | { scatterGather: ScatterGatherSpec }
  | { filter: FilterSpec }
  | { schemaMap: SchemaMapSpec }
  | { mapEach: MapEachSpec }
  // Stateful patterns (IR defined, runtime not yet implemented)
  | { retry: RetrySpec }
  | { timeout: TimeoutSpec }
  | { cache: CacheSpec }
  | { idempotent: IdempotentSpec }
  | { circuitBreaker: CircuitBreakerSpec }
  | { deadLetter: DeadLetterSpec }
  | { saga: SagaSpec }
  | { claimCheck: ClaimCheckSpec };

/** Pipeline pattern - sequential execution */
export interface PipelineSpec {
  steps: PipelineStep[];
}

export interface PipelineStep {
  id: string;
  operation: StepOperation;
  input?: DataBinding;
}

export type StepOperation =
  | { tool: ToolCall }
  | { pattern: PatternSpec };

export interface ToolCall {
  name: string;
}

/** Data binding - where step input comes from */
export type DataBinding =
  | { input: InputBinding }
  | { step: StepBinding }
  | { constant: unknown };

export interface InputBinding {
  path: string;
}

export interface StepBinding {
  stepId: string;
  path: string;
}

/** Scatter-gather pattern - parallel fan-out with aggregation */
export interface ScatterGatherSpec {
  targets: ScatterTarget[];
  aggregation: AggregationStrategy;
  timeoutMs?: number;
  failFast?: boolean;
}

export type ScatterTarget =
  | { tool: string }
  | { pattern: PatternSpec };

export interface AggregationStrategy {
  ops: AggregationOp[];
}

export type AggregationOp =
  | { flatten: boolean }
  | { sort: SortOp }
  | { dedupe: DedupeOp }
  | { limit: LimitOp }
  | { concat: boolean }
  | { merge: boolean };

export interface SortOp {
  field: string;
  order: 'asc' | 'desc';
}

export interface DedupeOp {
  field: string;
}

export interface LimitOp {
  count: number;
}

/** Filter pattern - predicate-based filtering */
export interface FilterSpec {
  predicate: FieldPredicate;
}

export interface FieldPredicate {
  field: string;
  op: PredicateOp;
  value: PredicateValue;
}

export type PredicateOp = 'eq' | 'ne' | 'gt' | 'gte' | 'lt' | 'lte' | 'contains' | 'in';

export type PredicateValue =
  | { stringValue: string }
  | { numberValue: number }
  | { boolValue: boolean }
  | { nullValue: boolean }
  | { listValue: PredicateValue[] };

/** Schema map pattern - field transformation */
export interface SchemaMapSpec {
  mappings: Record<string, FieldSource>;
}

export type FieldSource =
  | { path: string }
  | { literal: LiteralValue }
  | { coalesce: CoalesceSource }
  | { template: TemplateSource }
  | { concat: ConcatSource }
  | { nested: SchemaMapSpec };

export type LiteralValue =
  | { stringValue: string }
  | { numberValue: number }
  | { boolValue: boolean }
  | { nullValue: boolean };

export interface CoalesceSource {
  paths: string[];
}

export interface TemplateSource {
  template: string;
  vars: Record<string, string>;
}

export interface ConcatSource {
  paths: string[];
  separator?: string;
}

/** Map-each pattern - apply to array elements */
export interface MapEachSpec {
  inner: MapEachInner;
}

export type MapEachInner =
  | { tool: string }
  | { pattern: PatternSpec };

// =============================================================================
// Stateful Pattern Specifications
// =============================================================================

/** Retry pattern - retry with backoff on failure */
export interface RetrySpec {
  inner: StepOperation;
  maxAttempts: number;
  backoff: BackoffStrategy;
  retryIf?: FieldPredicate;
  jitter?: number;
  attemptTimeoutMs?: number;
}

export type BackoffStrategy =
  | { fixed: FixedBackoff }
  | { exponential: ExponentialBackoff }
  | { linear: LinearBackoff };

export interface FixedBackoff {
  delayMs: number;
}

export interface ExponentialBackoff {
  initialDelayMs: number;
  maxDelayMs: number;
  multiplier?: number;
}

export interface LinearBackoff {
  initialDelayMs: number;
  incrementMs: number;
  maxDelayMs: number;
}

/** Timeout pattern - enforce max execution duration */
export interface TimeoutSpec {
  inner: StepOperation;
  durationMs: number;
  fallback?: StepOperation;
  message?: string;
}

/** Cache pattern - read-through caching */
export interface CacheSpec {
  keyPaths: string[];
  inner: StepOperation;
  store: string;
  ttlSeconds: number;
  staleWhileRevalidateSeconds?: number;
  cacheIf?: FieldPredicate;
}

/** Idempotent pattern - prevent duplicate processing */
export interface IdempotentSpec {
  keyPaths: string[];
  inner: StepOperation;
  store: string;
  ttlSeconds?: number;
  onDuplicate: OnDuplicate;
}

export type OnDuplicate = 'cached' | 'skip' | 'error';

/** Circuit breaker pattern - fail fast with recovery */
export interface CircuitBreakerSpec {
  name: string;
  inner: StepOperation;
  store: string;
  failureThreshold: number;
  failureWindowSeconds: number;
  resetTimeoutSeconds: number;
  successThreshold?: number;
  fallback?: StepOperation;
  failureIf?: FieldPredicate;
}

/** Dead letter pattern - capture failures */
export interface DeadLetterSpec {
  inner: StepOperation;
  deadLetterTool: string;
  maxAttempts?: number;
  backoff?: BackoffStrategy;
  rethrow?: boolean;
}

/** Saga pattern - distributed transaction with compensation */
export interface SagaSpec {
  steps: SagaStep[];
  store?: string;
  sagaIdPath?: string;
  timeoutMs?: number;
  output?: DataBinding;
}

export interface SagaStep {
  id: string;
  name: string;
  action: StepOperation;
  compensate?: StepOperation;
  input: DataBinding;
}

/** Claim check pattern - externalize large payloads */
export interface ClaimCheckSpec {
  storeTool: string;
  retrieveTool: string;
  inner: StepOperation;
  retrieveAtEnd?: boolean;
}

// =============================================================================
// Output Transform
// =============================================================================

export interface OutputTransform {
  mappings: Record<string, FieldSource>;
}

// =============================================================================
// JSON Schema (simplified)
// =============================================================================

export interface JSONSchema {
  type?: string;
  properties?: Record<string, JSONSchema>;
  required?: string[];
  items?: JSONSchema;
  [key: string]: unknown;
}

// =============================================================================
// Type Guards
// =============================================================================

export function isSourceTool(impl: ToolImplementation): impl is { source: SourceTool } {
  return 'source' in impl;
}

export function isComposition(impl: ToolImplementation): impl is { spec: PatternSpec } {
  return 'spec' in impl;
}

export function isPipeline(spec: PatternSpec): spec is { pipeline: PipelineSpec } {
  return 'pipeline' in spec;
}

export function isScatterGather(spec: PatternSpec): spec is { scatterGather: ScatterGatherSpec } {
  return 'scatterGather' in spec;
}

export function isFilter(spec: PatternSpec): spec is { filter: FilterSpec } {
  return 'filter' in spec;
}

export function isSchemaMap(spec: PatternSpec): spec is { schemaMap: SchemaMapSpec } {
  return 'schemaMap' in spec;
}

export function isMapEach(spec: PatternSpec): spec is { mapEach: MapEachSpec } {
  return 'mapEach' in spec;
}

// Stateful pattern type guards
export function isRetry(spec: PatternSpec): spec is { retry: RetrySpec } {
  return 'retry' in spec;
}

export function isTimeout(spec: PatternSpec): spec is { timeout: TimeoutSpec } {
  return 'timeout' in spec;
}

export function isCache(spec: PatternSpec): spec is { cache: CacheSpec } {
  return 'cache' in spec;
}

export function isIdempotent(spec: PatternSpec): spec is { idempotent: IdempotentSpec } {
  return 'idempotent' in spec;
}

export function isCircuitBreaker(spec: PatternSpec): spec is { circuitBreaker: CircuitBreakerSpec } {
  return 'circuitBreaker' in spec;
}

export function isDeadLetter(spec: PatternSpec): spec is { deadLetter: DeadLetterSpec } {
  return 'deadLetter' in spec;
}

export function isSaga(spec: PatternSpec): spec is { saga: SagaSpec } {
  return 'saga' in spec;
}

export function isClaimCheck(spec: PatternSpec): spec is { claimCheck: ClaimCheckSpec } {
  return 'claimCheck' in spec;
}

