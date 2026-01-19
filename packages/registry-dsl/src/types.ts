/**
 * TypeScript types for the Registry v2 IR
 *
 * These types mirror the proto definitions in crates/agentgateway/proto/registry.proto
 * Used by the TypeScript DSL for type-safe registry construction.
 */

// =============================================================================
// Core Registry
// =============================================================================

/**
 * Registry is the root container for the v2 registry IR.
 * Contains schemas, servers, agents, and tools for full registry definition.
 */
export interface Registry {
  /** Schema version for compatibility checking (e.g., "2.0") */
  schemaVersion: string;

  /** List of tool definitions (virtual tools and compositions) */
  tools: ToolDefinition[];

  /** Named JSON Schema definitions (v2) - can be referenced via "$ref": "#/schemas/<name>" */
  schemas: SchemaDefinition[];

  /** MCP server definitions with versioning (v2) */
  servers: ServerDefinition[];

  /** Agent definitions for A2A routing (v2) */
  agents: AgentDefinition[];
}

// =============================================================================
// Schema Definitions
// =============================================================================

/**
 * SchemaDefinition represents a named, reusable JSON Schema.
 * Tools can reference these via "$ref": "#/schemas/<name>"
 */
export interface SchemaDefinition {
  /** Unique schema name (used in $ref) */
  name: string;

  /** Optional description of this schema */
  description?: string;

  /** The JSON Schema definition */
  schema: JsonSchema;

  /** Semantic version of this schema */
  version?: string;

  /** Optional metadata (owner, classification, deprecation info, etc.) */
  metadata?: Record<string, unknown>;
}

/** JSON Schema type (simplified representation) */
export type JsonSchema = Record<string, unknown>;

// =============================================================================
// Server Definitions
// =============================================================================

/**
 * ServerDefinition represents an MCP server with versioning.
 * Enables version-aware routing: "server:version" key dispatch.
 */
export interface ServerDefinition {
  /** Server name (e.g., "doc-service") */
  name: string;

  /** Server version (e.g., "1.2.0") - forms "name:version" routing key */
  version: string;

  /** Optional description of this server */
  description?: string;

  /** Server capabilities (what protocols/features it supports) */
  capabilities?: ServerCapabilities;

  /** Tools provided by this server (for validation) */
  providedTools?: ServerTool[];

  /** Optional metadata (owner, team, health endpoint, etc.) */
  metadata?: Record<string, unknown>;
}

/** ServerCapabilities describes what an MCP server supports */
export interface ServerCapabilities {
  /** Supports MCP StreamableHTTP protocol */
  streamableHttp?: boolean;

  /** Supports MCP stdio protocol */
  stdio?: boolean;

  /** Supports SSE transport */
  sse?: boolean;

  /** Supports tool invocation */
  tools?: boolean;

  /** Supports prompts */
  prompts?: boolean;

  /** Supports resources */
  resources?: boolean;

  /** Supports sampling */
  sampling?: boolean;
}

/** ServerTool represents a tool provided by a server (for validation) */
export interface ServerTool {
  /** Tool name as exposed by the server */
  name: string;

  /** Expected input schema reference (e.g., "#/schemas/WeatherInput") */
  inputSchemaRef?: string;

  /** Expected output schema reference (e.g., "#/schemas/WeatherOutput") */
  outputSchemaRef?: string;
}

// =============================================================================
// Agent Definitions
// =============================================================================

/**
 * AgentDefinition represents an agent for A2A routing.
 * Enables agent multiplexing and agent-as-tool execution.
 */
export interface AgentDefinition {
  /** Unique agent name */
  name: string;

  /** Agent version (semantic versioning) */
  version: string;

  /** Human-readable description */
  description?: string;

  /** Agent endpoint configuration */
  endpoint: AgentEndpoint;

  /** Skills/capabilities this agent provides */
  skills?: AgentSkill[];

  /** Dependencies this agent has on other tools/agents */
  dependencies?: AgentDependency[];

  /** Optional metadata (owner, team, cost tier, etc.) */
  metadata?: Record<string, unknown>;
}

/** AgentEndpoint describes how to connect to an agent */
export type AgentEndpoint =
  | { a2a: A2AEndpoint }
  | { mcp: MCPEndpoint };

/** A2AEndpoint for Agent-to-Agent protocol connections */
export interface A2AEndpoint {
  /** Base URL for A2A requests */
  url: string;

  /** Authentication configuration (optional) */
  auth?: AgentAuth;
}

/** MCPEndpoint for MCP-native agent connections */
export interface MCPEndpoint {
  /** Server name reference (from servers list) */
  server: string;

  /** Server version (optional, uses latest if not specified) */
  serverVersion?: string;
}

/** AgentAuth describes authentication for agent connections */
export type AgentAuth =
  | { bearer: BearerAuth }
  | { apiKey: ApiKeyAuth }
  | { oauth2: OAuth2Auth };

export interface BearerAuth {
  /** Token value (supports ${ENV_VAR} substitution) */
  token: string;
}

export interface ApiKeyAuth {
  /** Header name for the API key */
  header: string;

  /** API key value (supports ${ENV_VAR} substitution) */
  key: string;
}

export interface OAuth2Auth {
  /** Token endpoint URL */
  tokenUrl: string;

  /** Client ID (supports ${ENV_VAR} substitution) */
  clientId: string;

  /** Client secret (supports ${ENV_VAR} substitution) */
  clientSecret: string;

  /** Scopes to request */
  scopes?: string[];
}

/** AgentSkill describes a capability an agent provides */
export interface AgentSkill {
  /** Skill name (e.g., "code_review", "research", "data_analysis") */
  name: string;

  /** Skill description */
  description?: string;

  /** Input schema for this skill */
  inputSchema?: SchemaRef;

  /** Output schema for this skill */
  outputSchema?: SchemaRef;

  /** Example invocations (for LLM context) */
  examples?: SkillExample[];
}

/** SchemaRef can be either a reference to a named schema or inline */
export type SchemaRef =
  | { ref: string }
  | { inline: JsonSchema };

/** SkillExample provides example invocations for a skill */
export interface SkillExample {
  /** Example input */
  input: unknown;

  /** Expected output */
  output?: unknown;

  /** Description of this example */
  description?: string;
}

/**
 * AgentDependency declares what tools/agents an agent depends on.
 * Used for dependency-scoped discovery.
 */
export type AgentDependency =
  | { tool: string }
  | { agent: string }
  | { server: ServerDependency };

/** ServerDependency declares dependency on tools from a server */
export interface ServerDependency {
  /** Server name */
  name: string;

  /** Optional version constraint (semver range, e.g., ">=1.0.0 <2.0.0") */
  versionConstraint?: string;

  /** Specific tools from this server (empty means all) */
  tools?: string[];
}

// =============================================================================
// Tool Definitions
// =============================================================================

/**
 * ToolDefinition represents either a virtual tool (1:1 mapping) or a composition (N:1 orchestration).
 */
export interface ToolDefinition {
  /** Name exposed to agents (unique identifier) */
  name: string;

  /** Optional description (for source-based, can inherit from backend) */
  description?: string;

  /** Tool implementation - either source-based or composition */
  implementation: ToolImplementation;

  /** Input schema override (JSON Schema) */
  inputSchema?: JsonSchema;

  /** Output transformation (applies to both virtual tools and compositions) */
  outputTransform?: OutputTransform;

  /** Semantic version of this tool definition */
  version?: string;

  /** Arbitrary metadata (owner, classification, etc.) */
  metadata?: Record<string, unknown>;
}

/** Tool implementation type */
export type ToolImplementation =
  | { source: SourceTool }
  | { spec: PatternSpec };

/** SourceTool defines a 1:1 mapping to a backend tool */
export interface SourceTool {
  /** Server name (v2: references ServerDefinition.name) */
  server: string;

  /** Original tool name on that server */
  tool: string;

  /** Fields to inject at call time (supports ${ENV_VAR} substitution) */
  defaults?: Record<string, unknown>;

  /** Fields to remove from schema (hidden from agents) */
  hideFields?: string[];

  /** Server version constraint (v2) */
  serverVersion?: string;
}

// =============================================================================
// Pattern Specifications
// =============================================================================

/** PatternSpec defines a composition pattern */
export type PatternSpec =
  // Stateless patterns
  | { pipeline: PipelineSpec }
  | { scatterGather: ScatterGatherSpec }
  | { filter: FilterSpec }
  | { schemaMap: SchemaMapSpec }
  | { mapEach: MapEachSpec }
  // Stateful patterns
  | { retry: RetrySpec }
  | { timeout: TimeoutSpec }
  | { cache: CacheSpec }
  | { idempotent: IdempotentSpec }
  | { circuitBreaker: CircuitBreakerSpec }
  | { deadLetter: DeadLetterSpec }
  | { saga: SagaSpec }
  | { claimCheck: ClaimCheckSpec };

// =============================================================================
// Pipeline Pattern
// =============================================================================

/** PipelineSpec executes steps sequentially, passing output to next step */
export interface PipelineSpec {
  steps: PipelineStep[];
}

export interface PipelineStep {
  /** Unique identifier for this step (for data binding references) */
  id: string;

  /** The operation to execute */
  operation: StepOperation;

  /** Input binding for this step */
  input?: DataBinding;
}

/** StepOperation defines what a step does */
export type StepOperation =
  | { tool: ToolCall }
  | { pattern: PatternSpec }
  | { agent: AgentCall };

/** AgentCall invokes a registered agent as a step operation */
export interface AgentCall {
  /** Agent name (references AgentDefinition.name) */
  name: string;

  /** Specific skill to invoke (optional, uses default if not specified) */
  skill?: string;

  /** Agent version constraint (optional, uses latest if not specified) */
  version?: string;
}

export interface ToolCall {
  /** Tool name (can be virtual tool, composition, or backend tool) */
  name: string;

  /** Server name override (v2: for direct backend tool calls) */
  server?: string;

  /** Server version constraint (v2) */
  serverVersion?: string;
}

/** DataBinding specifies where step input comes from */
export type DataBinding =
  | { input: InputBinding }
  | { step: StepBinding }
  | { constant: unknown };

export interface InputBinding {
  /** JSONPath into composition input (e.g., "$" for whole input, "$.query" for field) */
  path: string;
}

export interface StepBinding {
  /** ID of the step to reference */
  stepId: string;

  /** JSONPath into step output */
  path: string;
}

// =============================================================================
// Scatter-Gather Pattern
// =============================================================================

/** ScatterGatherSpec fans out to multiple targets in parallel and aggregates results */
export interface ScatterGatherSpec {
  /** Targets to invoke in parallel */
  targets: ScatterTarget[];

  /** How to aggregate results */
  aggregation: AggregationStrategy;

  /** Timeout in milliseconds (optional) */
  timeoutMs?: number;

  /** If true, fail immediately on first error; if false, collect partial results */
  failFast?: boolean;
}

export type ScatterTarget =
  | { tool: string }
  | { pattern: PatternSpec };

/** AggregationStrategy defines how to combine scatter-gather results */
export interface AggregationStrategy {
  /** Sequence of operations applied in order */
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
  /** JSONPath to the field to sort by */
  field: string;

  /** Sort order: "asc" or "desc" */
  order: 'asc' | 'desc';
}

export interface DedupeOp {
  /** JSONPath to the field to dedupe by */
  field: string;
}

export interface LimitOp {
  /** Maximum number of results */
  count: number;
}

// =============================================================================
// Filter Pattern
// =============================================================================

/** FilterSpec filters array elements based on a predicate */
export interface FilterSpec {
  /** The predicate to evaluate for each element */
  predicate: FieldPredicate;
}

export interface FieldPredicate {
  /** JSONPath to the field to evaluate */
  field: string;

  /** Comparison operator */
  op: 'eq' | 'ne' | 'gt' | 'gte' | 'lt' | 'lte' | 'contains' | 'in';

  /** Value to compare against */
  value: PredicateValue;
}

export type PredicateValue =
  | { stringValue: string }
  | { numberValue: number }
  | { boolValue: boolean }
  | { nullValue: true }
  | { listValue: PredicateValue[] };

// =============================================================================
// Schema Map Pattern
// =============================================================================

/** SchemaMapSpec transforms input to output using field mappings */
export interface SchemaMapSpec {
  /** Field name -> source mapping */
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
  | { nullValue: true };

export interface CoalesceSource {
  /** JSONPaths to try in order, returning first non-null */
  paths: string[];
}

export interface TemplateSource {
  /** Template string with {var} placeholders */
  template: string;

  /** Variable name -> JSONPath binding */
  vars: Record<string, string>;
}

export interface ConcatSource {
  /** JSONPaths to concatenate */
  paths: string[];

  /** Separator between values (default: empty string) */
  separator?: string;
}

// =============================================================================
// Map Each Pattern
// =============================================================================

/** MapEachSpec applies an operation to each element of an array */
export interface MapEachSpec {
  /** The operation to apply to each element */
  inner: MapEachInner;
}

export type MapEachInner =
  | { tool: string }
  | { pattern: PatternSpec };

// =============================================================================
// Output Transform
// =============================================================================

/** OutputTransform defines how to transform tool/composition output */
export interface OutputTransform {
  /** Field name -> source mapping */
  mappings: Record<string, FieldSource>;
}

// =============================================================================
// Stateful Patterns
// =============================================================================

/** RetrySpec - retry with configurable backoff on failure */
export interface RetrySpec {
  /** The operation to retry */
  inner: StepOperation;

  /** Maximum attempts (including initial) */
  maxAttempts: number;

  /** Backoff strategy */
  backoff: BackoffStrategy;

  /** Condition to retry (if absent, retry all errors) */
  retryIf?: FieldPredicate;

  /** Jitter factor (0.0 - 1.0) */
  jitter?: number;

  /** Per-attempt timeout in milliseconds */
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

/** TimeoutSpec - enforce maximum execution duration */
export interface TimeoutSpec {
  /** The operation to wrap */
  inner: StepOperation;

  /** Timeout duration in milliseconds */
  durationMs: number;

  /** Fallback on timeout (optional) */
  fallback?: StepOperation;

  /** Custom error message */
  message?: string;
}

/** CacheSpec - read-through caching with TTL */
export interface CacheSpec {
  /** JSONPath expressions to derive cache key */
  keyPaths: string[];

  /** The operation to cache */
  inner: StepOperation;

  /** Store reference name (configured in gateway) */
  store: string;

  /** TTL in seconds */
  ttlSeconds: number;

  /** Stale-while-revalidate window in seconds */
  staleWhileRevalidateSeconds?: number;

  /** Condition to cache result (if absent, always cache) */
  cacheIf?: FieldPredicate;
}

/** IdempotentSpec - prevent duplicate processing */
export interface IdempotentSpec {
  /** JSONPath expressions to derive idempotency key */
  keyPaths: string[];

  /** The operation to wrap */
  inner: StepOperation;

  /** Store reference name (configured in gateway) */
  store: string;

  /** TTL in seconds (0 = no expiry) */
  ttlSeconds?: number;

  /** Behavior on duplicate */
  onDuplicate: OnDuplicate;
}

export type OnDuplicate = 'cached' | 'skip' | 'error';

/** CircuitBreakerSpec - fail fast with automatic recovery */
export interface CircuitBreakerSpec {
  /** Unique name for this circuit (for state isolation) */
  name: string;

  /** The protected operation */
  inner: StepOperation;

  /** Store for circuit state */
  store: string;

  /** Number of failures to trip the circuit */
  failureThreshold: number;

  /** Window for counting failures (seconds) */
  failureWindowSeconds: number;

  /** Time to wait before half-open (seconds) */
  resetTimeoutSeconds: number;

  /** Successes needed in half-open to close (default: 1) */
  successThreshold?: number;

  /** Fallback when circuit is open (optional) */
  fallback?: StepOperation;

  /** Custom failure condition (if absent, any error) */
  failureIf?: FieldPredicate;
}

/** DeadLetterSpec - capture failures for later processing */
export interface DeadLetterSpec {
  /** The operation to wrap */
  inner: StepOperation;

  /** Tool to invoke on failure */
  deadLetterTool: string;

  /** Max attempts before dead-lettering (default: 1) */
  maxAttempts?: number;

  /** Backoff between attempts */
  backoff?: BackoffStrategy;

  /** Whether to rethrow after dead-lettering */
  rethrow: boolean;
}

/** SagaSpec - distributed transaction with compensation */
export interface SagaSpec {
  /** Ordered list of saga steps */
  steps: SagaStep[];

  /** Store for saga state (for recovery) */
  store?: string;

  /** JSONPath to derive saga instance ID */
  sagaIdPath?: string;

  /** Timeout for entire saga in milliseconds */
  timeoutMs?: number;

  /** Output binding */
  output?: DataBinding;
}

export interface SagaStep {
  /** Step identifier */
  id: string;

  /** Human-readable name */
  name: string;

  /** The action to perform */
  action: StepOperation;

  /** Compensating action (optional) */
  compensate?: StepOperation;

  /** Input binding for this step */
  input?: DataBinding;
}

/** ClaimCheckSpec - externalize large payloads */
export interface ClaimCheckSpec {
  /** Tool to store payload and return reference */
  storeTool: string;

  /** Tool to retrieve payload from reference */
  retrieveTool: string;

  /** Inner operation operating on reference */
  inner: StepOperation;

  /** Whether to retrieve original at end */
  retrieveAtEnd: boolean;
}
