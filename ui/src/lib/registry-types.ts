// Registry types for virtual tool definitions
// These match the Rust types in crates/agentgateway/src/mcp/registry/types.rs

// =============================================================================
// Registry Core Types
// =============================================================================

export interface Registry {
  schemaVersion: string;
  description?: string;
  schemas: Schema[];
  servers: Server[];
  agents: AgentDefinition[];
  tools: ToolDefinition[];
  unknownCallerPolicy?: UnknownCallerPolicy;
  metadata?: Record<string, unknown>;
}

export type UnknownCallerPolicy = "allowAll" | "denyAll" | "allowUnregistered";

// =============================================================================
// Schema Definitions
// =============================================================================

export interface Schema {
  name: string;
  version?: string;
  description?: string;
  schema: JsonSchema;
  metadata?: Record<string, unknown>;
}

export type SchemaRef = { inline: JsonSchema } | { $ref: string };

export interface JsonSchema {
  type?: string;
  properties?: Record<string, JsonSchema>;
  items?: JsonSchema;
  required?: string[];
  description?: string;
  enum?: string[];
  default?: unknown;
  $ref?: string;
  [key: string]: unknown;
}

// =============================================================================
// Server Definitions
// =============================================================================

export interface Server {
  name: string;
  version?: string;
  description?: string;
  provides: ToolProvision[];
  deprecated?: boolean;
  deprecationMessage?: string;
  metadata?: Record<string, unknown>;
}

export interface ToolProvision {
  tool: string;
  version?: string;
}

// =============================================================================
// Agent Definitions (A2A Compatible)
// =============================================================================

export interface AgentDefinition {
  name: string;
  version?: string;
  description?: string;
  url?: string;
  protocolVersion?: string;
  defaultInputModes?: string[];
  defaultOutputModes?: string[];
  skills?: AgentSkillDefinition[];
  capabilities?: AgentCapabilities;
  provider?: AgentProvider;
  metadata?: Record<string, unknown>;
}

export interface AgentSkillDefinition {
  id: string;
  name?: string;
  description?: string;
  tags?: string[];
  examples?: string[];
  inputModes?: string[];
  outputModes?: string[];
  inputSchema?: SchemaRef;
  outputSchema?: SchemaRef;
}

export interface AgentCapabilities {
  streaming?: boolean;
  pushNotifications?: boolean;
  stateTransitionHistory?: boolean;
  extensions?: AgentExtension[];
}

export interface AgentExtension {
  uri: string;
  description?: string;
  required?: boolean;
  params?: unknown;
}

export interface AgentProvider {
  organization: string;
  url?: string;
}

// =============================================================================
// Tool Definitions
// =============================================================================

export interface ToolDefinition {
  name: string;
  description?: string;
  version?: string;
  tags?: string[];
  deprecated?: string;
  depends?: Dependency[];
  inputSchema?: JsonSchema;
  outputSchema?: JsonSchema | SchemaRef;
  outputTransform?: OutputTransform;
  metadata?: Record<string, unknown>;
  // Implementation - one of these should be present
  source?: SourceTool;
  spec?: PatternSpec;
}

export interface SourceTool {
  target?: string;
  server?: string;
  tool: string;
  defaults?: Record<string, unknown>;
  hideFields?: string[];
  serverVersion?: string;
}

export interface OutputTransform {
  mappings: Record<string, FieldSource>;
}

export interface Dependency {
  type: DependencyType;
  name: string;
  version?: string;
  skill?: string;
}

export type DependencyType = "tool" | "agent";

// =============================================================================
// Field Sources (for output transforms)
// =============================================================================

export type FieldSource =
  | { path: string }
  | { literal: LiteralValue }
  | { coalesce: CoalesceSource }
  | { template: TemplateSource }
  | { concat: ConcatSource }
  | { nested: SchemaMapSpec }
  | { arrayMap: ArrayMapSource };

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

export interface ArrayMapSource {
  over: string;
  each: Record<string, FieldSource>;
}

export interface SchemaMapSpec {
  mappings: Record<string, FieldSource>;
}

// =============================================================================
// Pattern Specifications
// =============================================================================

export type PatternSpec =
  | { pipeline: PipelineSpec }
  | { scatterGather: ScatterGatherSpec }
  | { filter: FilterSpec }
  | { schemaMap: SchemaMapSpec }
  | { mapEach: MapEachSpec };

// Pipeline Pattern
export interface PipelineSpec {
  steps: PipelineStep[];
}

export interface PipelineStep {
  id: string;
  operation: StepOperation;
  input?: DataBinding;
}

export type StepOperation = { tool: ToolCall } | { agent: AgentCall } | { pattern: PatternSpec };

export interface ToolCall {
  name: string;
  server?: string;
}

export interface AgentCall {
  name: string;
  skill?: string;
}

export type DataBinding =
  | { input: InputBinding }
  | { step: StepBinding }
  | { constant: unknown }
  | { construct: ConstructBinding };

export interface InputBinding {
  path: string;
}

export interface StepBinding {
  stepId: string;
  path: string;
}

export interface ConstructBinding {
  fields: Record<string, DataBinding>;
}

// Scatter-Gather Pattern
export interface ScatterGatherSpec {
  targets: ScatterTarget[];
  aggregation: AggregationStrategy;
  timeoutMs?: number;
  failFast?: boolean;
}

export type ScatterTarget = { tool: string; server?: string } | { pattern: PatternSpec };

export interface AggregationStrategy {
  ops: AggregationOp[];
}

export type AggregationOp =
  | { flatten: boolean }
  | { sort: SortOp }
  | { dedupe: DedupeOp }
  | { limit: LimitOp }
  | { concat: boolean }
  | { merge: boolean }
  | { wrap: WrapOp }
  | { extract: ExtractOp };

export interface SortOp {
  field: string;
  order: "asc" | "desc";
}

export interface DedupeOp {
  field: string;
}

export interface LimitOp {
  count: number;
}

export interface WrapOp {
  field: string;
}

export interface ExtractOp {
  path: string;
}

// Filter Pattern
export interface FilterSpec {
  predicate: FieldPredicate;
}

export interface FieldPredicate {
  field: string;
  op: PredicateOp;
  value: LiteralValue;
}

export type PredicateOp =
  | "eq"
  | "ne"
  | "gt"
  | "gte"
  | "lt"
  | "lte"
  | "contains"
  | "startsWith"
  | "endsWith";

// MapEach Pattern
export interface MapEachSpec {
  inner: MapEachInner;
}

export type MapEachInner = { tool: string; server?: string } | { pattern: PatternSpec };

// =============================================================================
// Helper Functions
// =============================================================================

export function isSourceTool(tool: ToolDefinition): boolean {
  return tool.source !== undefined;
}

export function isCompositionTool(tool: ToolDefinition): boolean {
  return tool.spec !== undefined;
}

export function getToolType(
  tool: ToolDefinition
): "source" | "pipeline" | "scatterGather" | "filter" | "schemaMap" | "mapEach" | "unknown" {
  if (tool.source) return "source";
  if (tool.spec) {
    if ("pipeline" in tool.spec) return "pipeline";
    if ("scatterGather" in tool.spec) return "scatterGather";
    if ("filter" in tool.spec) return "filter";
    if ("schemaMap" in tool.spec) return "schemaMap";
    if ("mapEach" in tool.spec) return "mapEach";
  }
  return "unknown";
}

export function getToolTypeLabel(type: ReturnType<typeof getToolType>): string {
  const labels: Record<string, string> = {
    source: "Source Tool",
    pipeline: "Pipeline",
    scatterGather: "Scatter-Gather",
    filter: "Filter",
    schemaMap: "Schema Map",
    mapEach: "Map Each",
    unknown: "Unknown",
  };
  return labels[type] || "Unknown";
}

export function getFieldSourceType(source: FieldSource): string {
  if ("path" in source) return "path";
  if ("literal" in source) return "literal";
  if ("coalesce" in source) return "coalesce";
  if ("template" in source) return "template";
  if ("concat" in source) return "concat";
  if ("nested" in source) return "nested";
  if ("arrayMap" in source) return "arrayMap";
  return "unknown";
}

export function createEmptyRegistry(): Registry {
  return {
    schemaVersion: "2.0",
    schemas: [],
    servers: [],
    agents: [],
    tools: [],
  };
}

export function createEmptySourceTool(name: string): ToolDefinition {
  return {
    name,
    source: {
      server: "",
      tool: "",
    },
  };
}

export function createEmptyScatterGatherTool(name: string): ToolDefinition {
  return {
    name,
    spec: {
      scatterGather: {
        targets: [],
        aggregation: {
          ops: [{ flatten: true }],
        },
      },
    },
  };
}

export function createEmptyPipelineTool(name: string): ToolDefinition {
  return {
    name,
    spec: {
      pipeline: {
        steps: [],
      },
    },
  };
}

export function getReferencedTools(tool: ToolDefinition): string[] {
  const refs: string[] = [];

  if (tool.source) {
    // Source tools don't reference other virtual tools
    return [];
  }

  if (tool.spec) {
    if ("pipeline" in tool.spec) {
      for (const step of tool.spec.pipeline.steps) {
        if ("tool" in step.operation) {
          refs.push(step.operation.tool.name);
        }
      }
    } else if ("scatterGather" in tool.spec) {
      for (const target of tool.spec.scatterGather.targets) {
        if ("tool" in target) {
          refs.push(target.tool);
        }
      }
    } else if ("mapEach" in tool.spec) {
      if ("tool" in tool.spec.mapEach.inner) {
        refs.push(tool.spec.mapEach.inner.tool);
      }
    }
  }

  return refs;
}
