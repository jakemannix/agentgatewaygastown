/**
 * Core type definitions for vMCP tool compositions
 * These types correspond to the registry.proto schema
 */
/** Registry containing tool definitions (v1 - legacy) */
export interface Registry {
    schemaVersion: string;
    tools: ToolDefinition[];
}
/** Registry v2 with schemas, servers, tools, and agents */
export interface RegistryV2 {
    schemaVersion: '2.0';
    schemas: SchemaDefinition[];
    servers: ServerDefinition[];
    tools: ToolDefinitionV2[];
    agents: AgentDefinition[];
}
/** Schema definition with versioning */
export interface SchemaDefinition {
    name: string;
    version: string;
    description?: string;
    schema: JSONSchema;
    metadata?: Record<string, unknown>;
}
/** Reference to a schema by name and version: "#SchemaName:Version" */
export interface SchemaRef {
    $ref: string;
}
/** Server (MCP backend) registration */
export interface ServerDefinition {
    name: string;
    version: string;
    description?: string;
    provides: ToolProvision[];
    deprecated?: boolean;
    deprecationMessage?: string;
    metadata?: Record<string, unknown>;
}
/** Tool provision - what tools a server provides */
export interface ToolProvision {
    tool: string;
    version: string;
}
/** Typed dependency reference */
export interface Dependency {
    type: 'tool' | 'agent';
    name: string;
    version: string;
    skill?: string;
}
/** Tool definition v2 with versioned source and dependencies */
export interface ToolDefinitionV2 {
    name: string;
    version: string;
    description?: string;
    source?: ToolSourceV2;
    spec?: PatternSpec;
    depends?: Dependency[];
    inputSchema?: JSONSchema | SchemaRef;
    outputSchema?: JSONSchema | SchemaRef;
    outputTransform?: OutputTransform;
    metadata?: Record<string, unknown>;
}
/** Source tool v2 - references server by name and version */
export interface ToolSourceV2 {
    server: string;
    serverVersion: string;
    tool: string;
    defaults?: Record<string, unknown>;
    hideFields?: string[];
}
/** Agent definition (A2A compatible) */
export interface AgentDefinition {
    name: string;
    version: string;
    description: string;
    url: string;
    protocolVersion: string;
    defaultInputModes: string[];
    defaultOutputModes: string[];
    skills: AgentSkill[];
    capabilities: AgentCapabilities;
    provider?: AgentProvider;
}
/** Agent skill definition */
export interface AgentSkill {
    id: string;
    name: string;
    description: string;
    tags: string[];
    examples?: string[];
    inputModes: string[];
    outputModes: string[];
    inputSchema?: JSONSchema | SchemaRef;
    outputSchema?: JSONSchema | SchemaRef;
}
/** Agent capabilities */
export interface AgentCapabilities {
    streaming?: boolean;
    pushNotifications?: boolean;
    stateTransitionHistory?: boolean;
    extensions?: AgentExtension[];
}
/** Agent extension */
export interface AgentExtension {
    uri: string;
    description?: string;
    required?: boolean;
    params?: Record<string, unknown>;
}
/** Agent provider information */
export interface AgentProvider {
    organization: string;
    url?: string;
}
/** Agent call in a pipeline step (Phase 2) */
export interface AgentCall {
    name: string;
    skill: string;
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
export type ToolImplementation = {
    source: SourceTool;
} | {
    spec: PatternSpec;
};
/** Source tool - 1:1 mapping to a backend tool */
export interface SourceTool {
    target: string;
    tool: string;
    defaults?: Record<string, unknown>;
    hideFields?: string[];
}
/** Pattern specification - one of the supported patterns */
export type PatternSpec = {
    pipeline: PipelineSpec;
} | {
    scatterGather: ScatterGatherSpec;
} | {
    filter: FilterSpec;
} | {
    schemaMap: SchemaMapSpec;
} | {
    mapEach: MapEachSpec;
} | {
    retry: RetrySpec;
} | {
    timeout: TimeoutSpec;
} | {
    cache: CacheSpec;
} | {
    idempotent: IdempotentSpec;
} | {
    circuitBreaker: CircuitBreakerSpec;
} | {
    deadLetter: DeadLetterSpec;
} | {
    saga: SagaSpec;
} | {
    claimCheck: ClaimCheckSpec;
} | {
    throttle: ThrottleSpec;
};
/** Pipeline pattern - sequential execution */
export interface PipelineSpec {
    steps: PipelineStep[];
}
export interface PipelineStep {
    id: string;
    operation: StepOperation;
    input?: DataBinding;
}
export type StepOperation = {
    tool: ToolCall;
} | {
    pattern: PatternSpec;
} | {
    agent: AgentCall;
};
export interface ToolCall {
    name: string;
}
/** Data binding - where step input comes from */
export type DataBinding = {
    input: InputBinding;
} | {
    step: StepBinding;
} | {
    constant: unknown;
};
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
export type ScatterTarget = {
    tool: string;
} | {
    pattern: PatternSpec;
};
export interface AggregationStrategy {
    ops: AggregationOp[];
}
export type AggregationOp = {
    flatten: boolean;
} | {
    sort: SortOp;
} | {
    dedupe: DedupeOp;
} | {
    limit: LimitOp;
} | {
    concat: boolean;
} | {
    merge: boolean;
};
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
export type PredicateValue = {
    stringValue: string;
} | {
    numberValue: number;
} | {
    boolValue: boolean;
} | {
    nullValue: boolean;
} | {
    listValue: PredicateValue[];
};
/** Schema map pattern - field transformation */
export interface SchemaMapSpec {
    mappings: Record<string, FieldSource>;
}
export type FieldSource = {
    path: string;
} | {
    literal: LiteralValue;
} | {
    coalesce: CoalesceSource;
} | {
    template: TemplateSource;
} | {
    concat: ConcatSource;
} | {
    nested: SchemaMapSpec;
};
export type LiteralValue = {
    stringValue: string;
} | {
    numberValue: number;
} | {
    boolValue: boolean;
} | {
    nullValue: boolean;
};
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
export type MapEachInner = {
    tool: string;
} | {
    pattern: PatternSpec;
};
/** Retry pattern - retry with backoff on failure */
export interface RetrySpec {
    inner: StepOperation;
    maxAttempts: number;
    backoff: BackoffStrategy;
    retryIf?: FieldPredicate;
    jitter?: number;
    attemptTimeoutMs?: number;
}
export type BackoffStrategy = {
    fixed: FixedBackoff;
} | {
    exponential: ExponentialBackoff;
} | {
    linear: LinearBackoff;
};
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
/** Throttle pattern - rate limiting for tool invocations */
export interface ThrottleSpec {
    inner: StepOperation;
    rate: number;
    windowMs: number;
    strategy?: ThrottleStrategy;
    onExceeded?: OnExceeded;
    store?: string;
}
/** Rate limiting strategy */
export type ThrottleStrategy = 'sliding_window' | 'token_bucket' | 'fixed_window' | 'leaky_bucket';
/** Behavior when rate limit is exceeded */
export type OnExceeded = 'wait' | 'reject' | 'queue';
export interface OutputTransform {
    mappings: Record<string, FieldSource>;
}
export interface JSONSchema {
    type?: string;
    properties?: Record<string, JSONSchema>;
    required?: string[];
    items?: JSONSchema;
    [key: string]: unknown;
}
export declare function isSourceTool(impl: ToolImplementation): impl is {
    source: SourceTool;
};
export declare function isComposition(impl: ToolImplementation): impl is {
    spec: PatternSpec;
};
export declare function isPipeline(spec: PatternSpec): spec is {
    pipeline: PipelineSpec;
};
export declare function isScatterGather(spec: PatternSpec): spec is {
    scatterGather: ScatterGatherSpec;
};
export declare function isFilter(spec: PatternSpec): spec is {
    filter: FilterSpec;
};
export declare function isSchemaMap(spec: PatternSpec): spec is {
    schemaMap: SchemaMapSpec;
};
export declare function isMapEach(spec: PatternSpec): spec is {
    mapEach: MapEachSpec;
};
export declare function isRetry(spec: PatternSpec): spec is {
    retry: RetrySpec;
};
export declare function isTimeout(spec: PatternSpec): spec is {
    timeout: TimeoutSpec;
};
export declare function isCache(spec: PatternSpec): spec is {
    cache: CacheSpec;
};
export declare function isIdempotent(spec: PatternSpec): spec is {
    idempotent: IdempotentSpec;
};
export declare function isCircuitBreaker(spec: PatternSpec): spec is {
    circuitBreaker: CircuitBreakerSpec;
};
export declare function isDeadLetter(spec: PatternSpec): spec is {
    deadLetter: DeadLetterSpec;
};
export declare function isSaga(spec: PatternSpec): spec is {
    saga: SagaSpec;
};
export declare function isClaimCheck(spec: PatternSpec): spec is {
    claimCheck: ClaimCheckSpec;
};
export declare function isThrottle(spec: PatternSpec): spec is {
    throttle: ThrottleSpec;
};
/** Check if schema or inline schema is a reference */
export declare function isSchemaRef(schema: JSONSchema | SchemaRef): schema is SchemaRef;
/** Check if a tool definition v2 has a source (vs composition) */
export declare function hasSourceV2(tool: ToolDefinitionV2): tool is ToolDefinitionV2 & {
    source: ToolSourceV2;
};
/** Check if a tool definition v2 has a spec (composition) */
export declare function hasSpecV2(tool: ToolDefinitionV2): tool is ToolDefinitionV2 & {
    spec: PatternSpec;
};
/** Check if a step operation is a tool call */
export declare function isToolOperation(op: StepOperation): op is {
    tool: ToolCall;
};
/** Check if a step operation is a pattern */
export declare function isPatternOperation(op: StepOperation): op is {
    pattern: PatternSpec;
};
/** Check if a step operation is an agent call */
export declare function isAgentOperation(op: StepOperation): op is {
    agent: AgentCall;
};
/** Check if a dependency is a tool dependency */
export declare function isToolDependency(dep: Dependency): boolean;
/** Check if a dependency is an agent dependency */
export declare function isAgentDependency(dep: Dependency): boolean;
/** Parse a schema reference string into name and version */
export declare function parseSchemaRef(ref: string): {
    name: string;
    version: string;
} | null;
//# sourceMappingURL=types.d.ts.map