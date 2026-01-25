export declare const protobufPackage = "agentgateway.dev.registry";
export declare enum OnDuplicate {
    ON_DUPLICATE_UNSPECIFIED = "ON_DUPLICATE_UNSPECIFIED",
    /** ON_DUPLICATE_CACHED - Return cached result */
    ON_DUPLICATE_CACHED = "ON_DUPLICATE_CACHED",
    /** ON_DUPLICATE_SKIP - Return null/empty */
    ON_DUPLICATE_SKIP = "ON_DUPLICATE_SKIP",
    /** ON_DUPLICATE_ERROR - Return error */
    ON_DUPLICATE_ERROR = "ON_DUPLICATE_ERROR",
    UNRECOGNIZED = "UNRECOGNIZED"
}
export declare function onDuplicateFromJSON(object: any): OnDuplicate;
export declare function onDuplicateToJSON(object: OnDuplicate): string;
/**
 * Registry is the root container for the v2 registry IR
 * Contains schemas, servers, agents, and tools for full registry definition
 */
export interface Registry {
    /** Schema version for compatibility checking (e.g., "2.0") */
    schemaVersion: string;
    /** List of tool definitions (virtual tools and compositions) */
    tools: ToolDefinition[];
    /**
     * Named JSON Schema definitions (v2)
     * Can be referenced by tools via "$ref": "#/schemas/<name>"
     */
    schemas: SchemaDefinition[];
    /**
     * MCP server definitions with versioning (v2)
     * Enables version-aware routing and validation
     */
    servers: ServerDefinition[];
    /**
     * Agent definitions for A2A routing (v2)
     * Enables agent multiplexing and agent-as-tool execution
     */
    agents: AgentDefinition[];
}
/**
 * SchemaDefinition represents a named, reusable JSON Schema
 * Tools can reference these via "$ref": "#/schemas/<name>"
 */
export interface SchemaDefinition {
    /** Unique schema name (used in $ref) */
    name: string;
    /** Optional description of this schema */
    description?: string | undefined;
    /** The JSON Schema definition */
    schema?: {
        [key: string]: any;
    } | undefined;
    /** Semantic version of this schema */
    version?: string | undefined;
    /** Optional metadata (owner, classification, deprecation info, etc.) */
    metadata: {
        [key: string]: any | undefined;
    };
}
export interface SchemaDefinition_MetadataEntry {
    key: string;
    value?: any | undefined;
}
/**
 * ServerDefinition represents an MCP server with versioning
 * Enables version-aware routing: "server:version" key dispatch
 */
export interface ServerDefinition {
    /** Server name (e.g., "doc-service") */
    name: string;
    /** Server version (e.g., "1.2.0") - forms "name:version" routing key */
    version: string;
    /** Optional description of this server */
    description?: string | undefined;
    /** Server capabilities (what protocols/features it supports) */
    capabilities?: ServerCapabilities | undefined;
    /**
     * Tools provided by this server (for validation)
     * Maps tool name -> expected input/output schema refs
     */
    providedTools: ServerTool[];
    /** Optional metadata (owner, team, health endpoint, etc.) */
    metadata: {
        [key: string]: any | undefined;
    };
}
export interface ServerDefinition_MetadataEntry {
    key: string;
    value?: any | undefined;
}
/** ServerCapabilities describes what an MCP server supports */
export interface ServerCapabilities {
    /** Supports MCP StreamableHTTP protocol */
    streamableHttp: boolean;
    /** Supports MCP stdio protocol */
    stdio: boolean;
    /** Supports SSE transport */
    sse: boolean;
    /** Supports tool invocation */
    tools: boolean;
    /** Supports prompts */
    prompts: boolean;
    /** Supports resources */
    resources: boolean;
    /** Supports sampling */
    sampling: boolean;
}
/** ServerTool represents a tool provided by a server (for validation) */
export interface ServerTool {
    /** Tool name as exposed by the server */
    name: string;
    /** Expected input schema reference (e.g., "#/schemas/WeatherInput") */
    inputSchemaRef?: string | undefined;
    /** Expected output schema reference (e.g., "#/schemas/WeatherOutput") */
    outputSchemaRef?: string | undefined;
}
/**
 * AgentDefinition represents an agent for A2A routing
 * Enables agent multiplexing and agent-as-tool execution
 */
export interface AgentDefinition {
    /** Unique agent name */
    name: string;
    /** Agent version (semantic versioning) */
    version: string;
    /** Human-readable description */
    description?: string | undefined;
    /** Agent endpoint configuration */
    endpoint?: AgentEndpoint | undefined;
    /** Skills/capabilities this agent provides */
    skills: AgentSkill[];
    /** Dependencies this agent has on other tools/agents */
    dependencies: AgentDependency[];
    /** Optional metadata (owner, team, cost tier, etc.) */
    metadata: {
        [key: string]: any | undefined;
    };
}
export interface AgentDefinition_MetadataEntry {
    key: string;
    value?: any | undefined;
}
/** AgentEndpoint describes how to connect to an agent */
export interface AgentEndpoint {
    transport?: //
    /** A2A HTTP endpoint */
    {
        $case: "a2a";
        a2a: A2AEndpoint;
    } | //
    /** MCP endpoint (for MCP-native agents) */
    {
        $case: "mcp";
        mcp: MCPEndpoint;
    } | undefined;
}
/** A2AEndpoint for Agent-to-Agent protocol connections */
export interface A2AEndpoint {
    /** Base URL for A2A requests */
    url: string;
    /** Authentication configuration (optional) */
    auth?: AgentAuth | undefined;
}
/** MCPEndpoint for MCP-native agent connections */
export interface MCPEndpoint {
    /** Server name reference (from servers list) */
    server: string;
    /** Server version (optional, uses latest if not specified) */
    serverVersion?: string | undefined;
}
/** AgentAuth describes authentication for agent connections */
export interface AgentAuth {
    authType?: //
    /** Bearer token authentication */
    {
        $case: "bearer";
        bearer: BearerAuth;
    } | //
    /** API key authentication */
    {
        $case: "apiKey";
        apiKey: ApiKeyAuth;
    } | //
    /** OAuth2 client credentials */
    {
        $case: "oauth2";
        oauth2: OAuth2Auth;
    } | undefined;
}
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
    scopes: string[];
}
/**
 * AgentSkill describes a capability an agent provides
 * Used for capability-based routing and discovery
 */
export interface AgentSkill {
    /** Skill name (e.g., "code_review", "research", "data_analysis") */
    name: string;
    /** Skill description */
    description?: string | undefined;
    /** Input schema for this skill (JSON Schema reference or inline) */
    inputSchema?: SchemaRef | undefined;
    /** Output schema for this skill */
    outputSchema?: SchemaRef | undefined;
    /** Example invocations (for LLM context) */
    examples: SkillExample[];
}
/** SchemaRef can be either a reference to a named schema or inline */
export interface SchemaRef {
    schema?: //
    /** Reference to a named schema: "#/schemas/<name>" */
    {
        $case: "ref";
        ref: string;
    } | //
    /** Inline JSON Schema definition */
    {
        $case: "inline";
        inline: {
            [key: string]: any;
        } | undefined;
    } | undefined;
}
/** SkillExample provides example invocations for a skill */
export interface SkillExample {
    /** Example input */
    input?: any | undefined;
    /** Expected output */
    output?: any | undefined;
    /** Description of this example */
    description?: string | undefined;
}
/**
 * AgentDependency declares what tools/agents an agent depends on
 * Used for dependency-scoped discovery (WP11)
 */
export interface AgentDependency {
    dependency?: //
    /** Depends on a specific tool by name */
    {
        $case: "tool";
        tool: string;
    } | //
    /** Depends on another agent by name */
    {
        $case: "agent";
        agent: string;
    } | //
    /** Depends on tools from a specific server */
    {
        $case: "server";
        server: ServerDependency;
    } | undefined;
}
/** ServerDependency declares dependency on tools from a server */
export interface ServerDependency {
    /** Server name */
    name: string;
    /** Optional version constraint (semver range, e.g., ">=1.0.0 <2.0.0") */
    versionConstraint?: string | undefined;
    /** Specific tools from this server (empty means all) */
    tools: string[];
}
/** ToolDefinition represents either a virtual tool (1:1 mapping) or a composition (N:1 orchestration) */
export interface ToolDefinition {
    /** Name exposed to agents (unique identifier) */
    name: string;
    /** Optional description (for source-based, can inherit from backend) */
    description?: string | undefined;
    /** Tool implementation - either source-based or composition */
    implementation?: //
    /** Virtual tool: adapts a single backend tool (1:1) */
    {
        $case: "source";
        source: SourceTool;
    } | //
    /** Composition: orchestrates multiple tools (N:1) */
    {
        $case: "spec";
        spec: PatternSpec;
    } | undefined;
    /** Input schema override (JSON Schema as struct) */
    inputSchema?: {
        [key: string]: any;
    } | undefined;
    /** Output transformation (applies to both virtual tools and compositions) */
    outputTransform?: OutputTransform | undefined;
    /** Semantic version of this tool definition */
    version?: string | undefined;
    /** Arbitrary metadata (owner, classification, etc.) */
    metadata: {
        [key: string]: any | undefined;
    };
}
export interface ToolDefinition_MetadataEntry {
    key: string;
    value?: any | undefined;
}
/** SourceTool defines a 1:1 mapping to a backend tool */
export interface SourceTool {
    /**
     * Server name (v2: references ServerDefinition.name)
     * For v1 compatibility, this is the target name in YAML config
     */
    server: string;
    /** Original tool name on that server */
    tool: string;
    /** Fields to inject at call time (supports ${ENV_VAR} substitution) */
    defaults: {
        [key: string]: any | undefined;
    };
    /** Fields to remove from schema (hidden from agents) */
    hideFields: string[];
    /**
     * Server version constraint (v2)
     * If specified, forms "server:version" routing key
     * Supports semver ranges (e.g., ">=1.0.0 <2.0.0") or exact versions
     */
    serverVersion?: string | undefined;
}
export interface SourceTool_DefaultsEntry {
    key: string;
    value?: any | undefined;
}
/** PatternSpec defines a composition pattern */
export interface PatternSpec {
    pattern?: //
    /** Stateless patterns (implemented) */
    {
        $case: "pipeline";
        pipeline: PipelineSpec;
    } | {
        $case: "scatterGather";
        scatterGather: ScatterGatherSpec;
    } | {
        $case: "filter";
        filter: FilterSpec;
    } | {
        $case: "schemaMap";
        schemaMap: SchemaMapSpec;
    } | {
        $case: "mapEach";
        mapEach: MapEachSpec;
    } | //
    /** Stateful patterns (IR defined, runtime not yet implemented) */
    {
        $case: "retry";
        retry: RetrySpec;
    } | {
        $case: "timeout";
        timeout: TimeoutSpec;
    } | {
        $case: "cache";
        cache: CacheSpec;
    } | {
        $case: "idempotent";
        idempotent: IdempotentSpec;
    } | {
        $case: "circuitBreaker";
        circuitBreaker: CircuitBreakerSpec;
    } | {
        $case: "deadLetter";
        deadLetter: DeadLetterSpec;
    } | {
        $case: "saga";
        saga: SagaSpec;
    } | {
        $case: "claimCheck";
        claimCheck: ClaimCheckSpec;
    } | undefined;
}
/** PipelineSpec executes steps sequentially, passing output to next step */
export interface PipelineSpec {
    steps: PipelineStep[];
}
export interface PipelineStep {
    /** Unique identifier for this step (for data binding references) */
    id: string;
    /** The operation to execute */
    operation?: StepOperation | undefined;
    /** Input binding for this step */
    input?: DataBinding | undefined;
}
/** StepOperation defines what a step does */
export interface StepOperation {
    op?: //
    /** Call a tool by name (resolved from registry or backend) */
    {
        $case: "tool";
        tool: ToolCall;
    } | //
    /** Inline pattern (no separate name) */
    {
        $case: "pattern";
        pattern: PatternSpec;
    } | //
    /** Call an agent (v2: for agent-as-tool execution) */
    {
        $case: "agent";
        agent: AgentCall;
    } | undefined;
}
/** AgentCall invokes a registered agent as a step operation */
export interface AgentCall {
    /** Agent name (references AgentDefinition.name) */
    name: string;
    /** Specific skill to invoke (optional, uses default if not specified) */
    skill?: string | undefined;
    /** Agent version constraint (optional, uses latest if not specified) */
    version?: string | undefined;
}
export interface ToolCall {
    /** Tool name (can be virtual tool, composition, or backend tool) */
    name: string;
    /**
     * Server name override (v2: for direct backend tool calls)
     * If specified, bypasses registry lookup and calls tool directly on server
     */
    server?: string | undefined;
    /** Server version constraint (v2) */
    serverVersion?: string | undefined;
}
/** DataBinding specifies where step input comes from */
export interface DataBinding {
    source?: //
    /** From composition input */
    {
        $case: "input";
        input: InputBinding;
    } | //
    /** From a previous step's output */
    {
        $case: "step";
        step: StepBinding;
    } | //
    /** Constant value */
    {
        $case: "constant";
        constant: any | undefined;
    } | //
    /** Construct an object from multiple bindings */
    {
        $case: "construct";
        construct: ConstructBinding;
    } | undefined;
}
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
/**
 * ConstructBinding builds an object from multiple bindings
 * Enables symmetric input construction (like outputTransform does for outputs)
 */
export interface ConstructBinding {
    /** Field name -> binding that produces the field value */
    fields: {
        [key: string]: DataBinding;
    };
}
export interface ConstructBinding_FieldsEntry {
    key: string;
    value?: DataBinding | undefined;
}
/** ScatterGatherSpec fans out to multiple targets in parallel and aggregates results */
export interface ScatterGatherSpec {
    /** Targets to invoke in parallel */
    targets: ScatterTarget[];
    /** How to aggregate results */
    aggregation?: AggregationStrategy | undefined;
    /** Timeout in milliseconds (optional) */
    timeoutMs?: number | undefined;
    /** If true, fail immediately on first error; if false, collect partial results */
    failFast: boolean;
}
export interface ScatterTarget {
    target?: //
    /** Tool name (resolved from registry or backend) */
    {
        $case: "tool";
        tool: string;
    } | //
    /** Inline pattern */
    {
        $case: "pattern";
        pattern: PatternSpec;
    } | undefined;
}
/** AggregationStrategy defines how to combine scatter-gather results */
export interface AggregationStrategy {
    /** Sequence of operations applied in order */
    ops: AggregationOp[];
}
export interface AggregationOp {
    op?: //
    /** Flatten array of arrays into single array */
    {
        $case: "flatten";
        flatten: boolean;
    } | //
    /** Sort by field */
    {
        $case: "sort";
        sort: SortOp;
    } | //
    /** Deduplicate by field */
    {
        $case: "dedupe";
        dedupe: DedupeOp;
    } | //
    /** Take first N results */
    {
        $case: "limit";
        limit: LimitOp;
    } | //
    /** Keep arrays nested (no flattening) */
    {
        $case: "concat";
        concat: boolean;
    } | //
    /** Merge objects (for object results) */
    {
        $case: "merge";
        merge: boolean;
    } | undefined;
}
export interface SortOp {
    /** JSONPath to the field to sort by */
    field: string;
    /** Sort order: "asc" or "desc" */
    order: string;
}
export interface DedupeOp {
    /** JSONPath to the field to dedupe by */
    field: string;
}
export interface LimitOp {
    /** Maximum number of results */
    count: number;
}
/** FilterSpec filters array elements based on a predicate */
export interface FilterSpec {
    /** The predicate to evaluate for each element */
    predicate?: FieldPredicate | undefined;
}
export interface FieldPredicate {
    /** JSONPath to the field to evaluate */
    field: string;
    /** Comparison operator: "eq", "ne", "gt", "gte", "lt", "lte", "contains", "in" */
    op: string;
    /** Value to compare against */
    value?: PredicateValue | undefined;
}
export interface PredicateValue {
    value?: {
        $case: "stringValue";
        stringValue: string;
    } | {
        $case: "numberValue";
        numberValue: number;
    } | {
        $case: "boolValue";
        boolValue: boolean;
    } | //
    /** If true, represents null */
    {
        $case: "nullValue";
        nullValue: boolean;
    } | //
    /** For "in" operator */
    {
        $case: "listValue";
        listValue: ValueList;
    } | undefined;
}
export interface ValueList {
    values: PredicateValue[];
}
/** SchemaMapSpec transforms input to output using field mappings */
export interface SchemaMapSpec {
    /** Field name -> source mapping */
    mappings: {
        [key: string]: FieldSource;
    };
}
export interface SchemaMapSpec_MappingsEntry {
    key: string;
    value?: FieldSource | undefined;
}
export interface FieldSource {
    source?: //
    /** JSONPath extraction from input */
    {
        $case: "path";
        path: string;
    } | //
    /** Constant value */
    {
        $case: "literal";
        literal: LiteralValue;
    } | //
    /** First non-null from multiple paths */
    {
        $case: "coalesce";
        coalesce: CoalesceSource;
    } | //
    /** String template with variable substitution */
    {
        $case: "template";
        template: TemplateSource;
    } | //
    /** Concatenate multiple fields */
    {
        $case: "concat";
        concat: ConcatSource;
    } | //
    /** Nested object mapping */
    {
        $case: "nested";
        nested: SchemaMapSpec;
    } | undefined;
}
export interface LiteralValue {
    value?: {
        $case: "stringValue";
        stringValue: string;
    } | {
        $case: "numberValue";
        numberValue: number;
    } | {
        $case: "boolValue";
        boolValue: boolean;
    } | //
    /** If true, value is null */
    {
        $case: "nullValue";
        nullValue: boolean;
    } | undefined;
}
export interface CoalesceSource {
    /** JSONPaths to try in order, returning first non-null */
    paths: string[];
}
export interface TemplateSource {
    /** Template string with {var} placeholders */
    template: string;
    /** Variable name -> JSONPath binding */
    vars: {
        [key: string]: string;
    };
}
export interface TemplateSource_VarsEntry {
    key: string;
    value: string;
}
export interface ConcatSource {
    /** JSONPaths to concatenate */
    paths: string[];
    /** Separator between values (default: empty string) */
    separator?: string | undefined;
}
/** MapEachSpec applies an operation to each element of an array */
export interface MapEachSpec {
    /** The operation to apply to each element */
    inner?: MapEachInner | undefined;
}
export interface MapEachInner {
    inner?: //
    /** Tool name to call for each element */
    {
        $case: "tool";
        tool: string;
    } | //
    /** Pattern to apply for each element */
    {
        $case: "pattern";
        pattern: PatternSpec;
    } | undefined;
}
/**
 * OutputTransform defines how to transform tool/composition output
 * This is the unified, enhanced version supporting all mapping features
 */
export interface OutputTransform {
    /** Field name -> source mapping */
    mappings: {
        [key: string]: FieldSource;
    };
}
export interface OutputTransform_MappingsEntry {
    key: string;
    value?: FieldSource | undefined;
}
/** RetrySpec - retry with configurable backoff on failure */
export interface RetrySpec {
    /** The operation to retry */
    inner?: StepOperation | undefined;
    /** Maximum attempts (including initial) */
    maxAttempts: number;
    /** Backoff strategy */
    backoff?: BackoffStrategy | undefined;
    /** Condition to retry (if absent, retry all errors) */
    retryIf?: FieldPredicate | undefined;
    /** Jitter factor (0.0 - 1.0) */
    jitter?: number | undefined;
    /** Per-attempt timeout in milliseconds */
    attemptTimeoutMs?: number | undefined;
}
export interface BackoffStrategy {
    strategy?: {
        $case: "fixed";
        fixed: FixedBackoff;
    } | {
        $case: "exponential";
        exponential: ExponentialBackoff;
    } | {
        $case: "linear";
        linear: LinearBackoff;
    } | undefined;
}
export interface FixedBackoff {
    delayMs: number;
}
export interface ExponentialBackoff {
    initialDelayMs: number;
    maxDelayMs: number;
    /** Default: 2.0 */
    multiplier?: number | undefined;
}
export interface LinearBackoff {
    initialDelayMs: number;
    incrementMs: number;
    maxDelayMs: number;
}
/** TimeoutSpec - enforce maximum execution duration */
export interface TimeoutSpec {
    /** The operation to wrap */
    inner?: StepOperation | undefined;
    /** Timeout duration in milliseconds */
    durationMs: number;
    /** Fallback on timeout (optional) */
    fallback?: StepOperation | undefined;
    /** Custom error message */
    message?: string | undefined;
}
/** CacheSpec - read-through caching with TTL */
export interface CacheSpec {
    /** JSONPath expressions to derive cache key */
    keyPaths: string[];
    /** The operation to cache */
    inner?: StepOperation | undefined;
    /** Store reference name (configured in gateway) */
    store: string;
    /** TTL in seconds */
    ttlSeconds: number;
    /** Stale-while-revalidate window in seconds */
    staleWhileRevalidateSeconds?: number | undefined;
    /** Condition to cache result (if absent, always cache) */
    cacheIf?: FieldPredicate | undefined;
}
/** IdempotentSpec - prevent duplicate processing */
export interface IdempotentSpec {
    /** JSONPath expressions to derive idempotency key */
    keyPaths: string[];
    /** The operation to wrap */
    inner?: StepOperation | undefined;
    /** Store reference name (configured in gateway) */
    store: string;
    /** TTL in seconds (0 = no expiry) */
    ttlSeconds?: number | undefined;
    /** Behavior on duplicate */
    onDuplicate: OnDuplicate;
}
/** CircuitBreakerSpec - fail fast with automatic recovery */
export interface CircuitBreakerSpec {
    /** Unique name for this circuit (for state isolation) */
    name: string;
    /** The protected operation */
    inner?: StepOperation | undefined;
    /** Store for circuit state */
    store: string;
    /** Number of failures to trip the circuit */
    failureThreshold: number;
    /** Window for counting failures (seconds) */
    failureWindowSeconds: number;
    /** Time to wait before half-open (seconds) */
    resetTimeoutSeconds: number;
    /** Successes needed in half-open to close (default: 1) */
    successThreshold?: number | undefined;
    /** Fallback when circuit is open (optional) */
    fallback?: StepOperation | undefined;
    /** Custom failure condition (if absent, any error) */
    failureIf?: FieldPredicate | undefined;
}
/** DeadLetterSpec - capture failures for later processing */
export interface DeadLetterSpec {
    /** The operation to wrap */
    inner?: StepOperation | undefined;
    /** Tool to invoke on failure */
    deadLetterTool: string;
    /** Max attempts before dead-lettering (default: 1) */
    maxAttempts?: number | undefined;
    /** Backoff between attempts */
    backoff?: BackoffStrategy | undefined;
    /** Whether to rethrow after dead-lettering */
    rethrow: boolean;
}
/** SagaSpec - distributed transaction with compensation */
export interface SagaSpec {
    /** Ordered list of saga steps */
    steps: SagaStep[];
    /** Store for saga state (for recovery) */
    store?: string | undefined;
    /** JSONPath to derive saga instance ID */
    sagaIdPath?: string | undefined;
    /** Timeout for entire saga in milliseconds */
    timeoutMs?: number | undefined;
    /** Output binding */
    output?: DataBinding | undefined;
}
export interface SagaStep {
    /** Step identifier */
    id: string;
    /** Human-readable name */
    name: string;
    /** The action to perform */
    action?: StepOperation | undefined;
    /** Compensating action (optional) */
    compensate?: StepOperation | undefined;
    /** Input binding for this step */
    input?: DataBinding | undefined;
}
/** ClaimCheckSpec - externalize large payloads */
export interface ClaimCheckSpec {
    /** Tool to store payload and return reference */
    storeTool: string;
    /** Tool to retrieve payload from reference */
    retrieveTool: string;
    /** Inner operation operating on reference */
    inner?: StepOperation | undefined;
    /** Whether to retrieve original at end */
    retrieveAtEnd: boolean;
}
export declare const Registry: MessageFns<Registry>;
export declare const SchemaDefinition: MessageFns<SchemaDefinition>;
export declare const SchemaDefinition_MetadataEntry: MessageFns<SchemaDefinition_MetadataEntry>;
export declare const ServerDefinition: MessageFns<ServerDefinition>;
export declare const ServerDefinition_MetadataEntry: MessageFns<ServerDefinition_MetadataEntry>;
export declare const ServerCapabilities: MessageFns<ServerCapabilities>;
export declare const ServerTool: MessageFns<ServerTool>;
export declare const AgentDefinition: MessageFns<AgentDefinition>;
export declare const AgentDefinition_MetadataEntry: MessageFns<AgentDefinition_MetadataEntry>;
export declare const AgentEndpoint: MessageFns<AgentEndpoint>;
export declare const A2AEndpoint: MessageFns<A2AEndpoint>;
export declare const MCPEndpoint: MessageFns<MCPEndpoint>;
export declare const AgentAuth: MessageFns<AgentAuth>;
export declare const BearerAuth: MessageFns<BearerAuth>;
export declare const ApiKeyAuth: MessageFns<ApiKeyAuth>;
export declare const OAuth2Auth: MessageFns<OAuth2Auth>;
export declare const AgentSkill: MessageFns<AgentSkill>;
export declare const SchemaRef: MessageFns<SchemaRef>;
export declare const SkillExample: MessageFns<SkillExample>;
export declare const AgentDependency: MessageFns<AgentDependency>;
export declare const ServerDependency: MessageFns<ServerDependency>;
export declare const ToolDefinition: MessageFns<ToolDefinition>;
export declare const ToolDefinition_MetadataEntry: MessageFns<ToolDefinition_MetadataEntry>;
export declare const SourceTool: MessageFns<SourceTool>;
export declare const SourceTool_DefaultsEntry: MessageFns<SourceTool_DefaultsEntry>;
export declare const PatternSpec: MessageFns<PatternSpec>;
export declare const PipelineSpec: MessageFns<PipelineSpec>;
export declare const PipelineStep: MessageFns<PipelineStep>;
export declare const StepOperation: MessageFns<StepOperation>;
export declare const AgentCall: MessageFns<AgentCall>;
export declare const ToolCall: MessageFns<ToolCall>;
export declare const DataBinding: MessageFns<DataBinding>;
export declare const InputBinding: MessageFns<InputBinding>;
export declare const StepBinding: MessageFns<StepBinding>;
export declare const ConstructBinding: MessageFns<ConstructBinding>;
export declare const ConstructBinding_FieldsEntry: MessageFns<ConstructBinding_FieldsEntry>;
export declare const ScatterGatherSpec: MessageFns<ScatterGatherSpec>;
export declare const ScatterTarget: MessageFns<ScatterTarget>;
export declare const AggregationStrategy: MessageFns<AggregationStrategy>;
export declare const AggregationOp: MessageFns<AggregationOp>;
export declare const SortOp: MessageFns<SortOp>;
export declare const DedupeOp: MessageFns<DedupeOp>;
export declare const LimitOp: MessageFns<LimitOp>;
export declare const FilterSpec: MessageFns<FilterSpec>;
export declare const FieldPredicate: MessageFns<FieldPredicate>;
export declare const PredicateValue: MessageFns<PredicateValue>;
export declare const ValueList: MessageFns<ValueList>;
export declare const SchemaMapSpec: MessageFns<SchemaMapSpec>;
export declare const SchemaMapSpec_MappingsEntry: MessageFns<SchemaMapSpec_MappingsEntry>;
export declare const FieldSource: MessageFns<FieldSource>;
export declare const LiteralValue: MessageFns<LiteralValue>;
export declare const CoalesceSource: MessageFns<CoalesceSource>;
export declare const TemplateSource: MessageFns<TemplateSource>;
export declare const TemplateSource_VarsEntry: MessageFns<TemplateSource_VarsEntry>;
export declare const ConcatSource: MessageFns<ConcatSource>;
export declare const MapEachSpec: MessageFns<MapEachSpec>;
export declare const MapEachInner: MessageFns<MapEachInner>;
export declare const OutputTransform: MessageFns<OutputTransform>;
export declare const OutputTransform_MappingsEntry: MessageFns<OutputTransform_MappingsEntry>;
export declare const RetrySpec: MessageFns<RetrySpec>;
export declare const BackoffStrategy: MessageFns<BackoffStrategy>;
export declare const FixedBackoff: MessageFns<FixedBackoff>;
export declare const ExponentialBackoff: MessageFns<ExponentialBackoff>;
export declare const LinearBackoff: MessageFns<LinearBackoff>;
export declare const TimeoutSpec: MessageFns<TimeoutSpec>;
export declare const CacheSpec: MessageFns<CacheSpec>;
export declare const IdempotentSpec: MessageFns<IdempotentSpec>;
export declare const CircuitBreakerSpec: MessageFns<CircuitBreakerSpec>;
export declare const DeadLetterSpec: MessageFns<DeadLetterSpec>;
export declare const SagaSpec: MessageFns<SagaSpec>;
export declare const SagaStep: MessageFns<SagaStep>;
export declare const ClaimCheckSpec: MessageFns<ClaimCheckSpec>;
type Builtin = Date | Function | Uint8Array | string | number | boolean | undefined;
export type DeepPartial<T> = T extends Builtin ? T : T extends globalThis.Array<infer U> ? globalThis.Array<DeepPartial<U>> : T extends ReadonlyArray<infer U> ? ReadonlyArray<DeepPartial<U>> : T extends {
    $case: string;
} ? {
    [K in keyof Omit<T, "$case">]?: DeepPartial<T[K]>;
} & {
    $case: T["$case"];
} : T extends {} ? {
    [K in keyof T]?: DeepPartial<T[K]>;
} : Partial<T>;
type KeysOfUnion<T> = T extends T ? keyof T : never;
export type Exact<P, I extends P> = P extends Builtin ? P : P & {
    [K in keyof P]: Exact<P[K], I[K]>;
} & {
    [K in Exclude<keyof I, KeysOfUnion<P>>]: never;
};
export interface MessageFns<T> {
    fromJSON(object: any): T;
    toJSON(message: T): unknown;
    create<I extends Exact<DeepPartial<T>, I>>(base?: I): T;
    fromPartial<I extends Exact<DeepPartial<T>, I>>(object: I): T;
}
export {};
//# sourceMappingURL=registry.d.ts.map