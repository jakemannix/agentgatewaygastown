/**
 * Core type definitions for vMCP tool compositions
 * These types correspond to the registry.proto schema
 */
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
//# sourceMappingURL=types.d.ts.map