# Design: TypeScript DSL → JSON IR → Runtime

A concrete design for implementing the tool algebra composition system with a clear pathway from user code to execution.

---

## Overview: The Three Layers

```
┌──────────────────────────────────────────────────────────────┐
│ Layer 1: TypeScript DSL (vmcp-dsl)                          │
│ What developers write                                        │
└──────────────────────────────────────────────────────────────┘
                            │
                            │ compile (tsc + vmcp-compiler)
                            ▼
┌──────────────────────────────────────────────────────────────┐
│ Layer 2: JSON IR (vmcp-ir)                                  │
│ Portable, versionable, schematized                          │
└──────────────────────────────────────────────────────────────┘
                            │
                            │ load + execute
                            ▼
┌──────────────────────────────────────────────────────────────┐
│ Layer 3: Rust Runtime (agentgateway/compositions)           │
│ Fast, safe execution in the gateway                         │
└──────────────────────────────────────────────────────────────┘
```

---

## Layer 1: TypeScript DSL

### Package Structure

```
@vmcp/dsl/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts          # Public API
│   ├── types.ts          # Type definitions
│   ├── patterns/
│   │   ├── pipeline.ts
│   │   ├── router.ts
│   │   ├── scatter-gather.ts
│   │   ├── filter.ts
│   │   ├── schema-map.ts    # Declarative field mapping
│   │   ├── map-each.ts      # Apply pattern to array elements
│   │   └── ... (other patterns)
│   ├── tool.ts           # Tool reference type
│   └── compiler.ts       # Emit JSON IR
└── examples/
    └── research-pipeline.ts
```

### Core Type System

```typescript
// src/types.ts

/** JSON-serializable value types */
export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

/** Tool reference (name only, resolved at runtime) */
export interface ToolRef<I = JsonValue, O = JsonValue> {
  readonly __brand: 'Tool';
  readonly name: string;
  readonly _input?: I;
  readonly _output?: O;
}

/** Composition is a tool that can be invoked */
export interface Composition<I = JsonValue, O = JsonValue> extends ToolRef<I, O> {
  readonly __brand: 'Composition';
}

/** Pattern base - all patterns extend this */
export interface Pattern<I = JsonValue, O = JsonValue> {
  readonly __pattern: string;
  toIR(): PatternIR;
}

/** Type-safe step definition */
export type Step<I, O> = ToolRef<I, O> | Pattern<I, O>;
```

### Pattern: Pipeline

```typescript
// src/patterns/pipeline.ts

import { Pattern, Step, JsonValue } from '../types';
import { PipelineIR } from '../ir-types';

export class Pipeline<I, O> implements Pattern<I, O> {
  readonly __pattern = 'pipeline';

  constructor(private steps: Step<any, any>[]) {}

  toIR(): PipelineIR {
    return {
      pattern: 'pipeline',
      steps: this.steps.map((step, idx) => ({
        id: `step_${idx}`,
        operation: isToolRef(step)
          ? { tool: step.name }
          : { composition: step.toIR() },
        input: idx === 0
          ? { source: 'input' }
          : { source: 'step', stepId: `step_${idx - 1}` }
      }))
    };
  }
}

/** Type-safe pipeline builder */
export function pipeline<I, O>(
  steps: Step<any, any>[]
): Pipeline<I, O> {
  return new Pipeline<I, O>(steps);
}

// Type inference helper
function isToolRef(step: any): step is ToolRef {
  return step.__brand === 'Tool' || step.__brand === 'Composition';
}
```

### Pattern: Router

```typescript
// src/patterns/router.ts

export interface RouteCase<I, O> {
  when: (input: I) => boolean;
  then: Step<I, O>;
}

export class Router<I, O> implements Pattern<I, O> {
  readonly __pattern = 'router';

  constructor(
    private cases: RouteCase<I, O>[],
    private otherwise?: Step<I, O>
  ) {}

  toIR(): RouterIR {
    return {
      pattern: 'router',
      routes: this.cases.map((c, idx) => ({
        id: `route_${idx}`,
        condition: c.when.toString(), // CEL expression or serialized predicate
        target: isToolRef(c.then)
          ? { tool: c.then.name }
          : { composition: c.then.toIR() }
      })),
      default: this.otherwise
        ? isToolRef(this.otherwise)
          ? { tool: this.otherwise.name }
          : { composition: this.otherwise.toIR() }
        : undefined
    };
  }
}

export function router<I, O>(
  cases: RouteCase<I, O>[],
  otherwise?: Step<I, O>
): Router<I, O> {
  return new Router(cases, otherwise);
}
```

### Pattern: Scatter-Gather

```typescript
// src/patterns/scatter-gather.ts

/** Aggregation primitives (composable, parametrized, type-safe via path builders) */
export type AggregationOp<T> =
  | { flatten: true }                                          // Flatten array of arrays
  | { sort: { by: FieldPath<number | string>; order: 'asc' | 'desc' } }  // Sort by field path
  | { dedupe: { by: FieldPath<string | number> } }             // Dedupe by field path
  | { limit: { count: number } }                               // Take first N
  | { concat: true }                                           // Simple concatenation (keep nested)
  | { merge: true };                                           // Merge objects

/** Aggregation strategy is a sequence of operations */
export interface AggregationStrategy<T> {
  ops: AggregationOp<T>[];
}

export interface ScatterGatherConfig<I, O, T = any> {
  targets: (ToolRef<I, T[]> | Pattern<I, T[]>)[];  // Can include composed tools
  aggregate: AggregationStrategy<T>;                // Type-safe sequence of ops
  timeout?: number;
  failFast?: boolean;
}

export class ScatterGather<I, O> implements Pattern<I, O> {
  readonly __pattern = 'scatter_gather';

  constructor(private config: ScatterGatherConfig<I, O>) {}

  toIR(): ScatterGatherIR {
    return {
      pattern: 'scatter_gather',
      targets: this.config.targets.map(t => 
        isToolRef(t) ? { tool: t.name } : { composition: t.toIR() }
      ),
      aggregation: {
        ops: this.config.aggregate.ops.map(op => this.compileAggOp(op))
      },
      timeout: this.config.timeout,
      failFast: this.config.failFast ?? false
    };
  }

  private compileAggOp(op: AggregationOp<any>): AggregationOpIR {
    if ('flatten' in op) return { flatten: true };
    if ('concat' in op) return { concat: true };
    if ('merge' in op) return { merge: true };
    if ('limit' in op) return { limit: { count: op.limit.count } };
    if ('sort' in op) {
      return { 
        sort: { 
          field: getPath(op.sort.by),  // Extract path from FieldPath
          order: op.sort.order 
        } 
      };
    }
    if ('dedupe' in op) {
      return { 
        dedupe: { 
          field: getPath(op.dedupe.by)  // Extract path from FieldPath
        } 
      };
    }
    throw new Error('Unknown aggregation op');
  }
}

export function scatterGather<I, O>(
  config: ScatterGatherConfig<I, O>
): ScatterGather<I, O> {
  return new ScatterGather(config);
}
```

### Pattern: Filter

```typescript
// src/patterns/filter.ts

/**
 * Type-safe filter predicates using path builders.
 * 
 * REJECTED: (item) => item.relevance > 0.7  // Allows arbitrary code!
 * 
 * Instead, use explicit predicate builders:
 *   filter($.relevance, gt(0.7))
 *   filter($.source, eq('arxiv'))
 *   filter($.tags, contains('important'))
 */

/** Comparison operators */
export type ComparisonOp = 'eq' | 'ne' | 'gt' | 'gte' | 'lt' | 'lte' | 'contains' | 'in';

/** Predicate value - the comparison to apply */
export interface Predicate<T> {
  readonly op: ComparisonOp;
  readonly value: T | T[];
}

// Predicate builders - type-safe and explicit
export const eq = <T>(value: T): Predicate<T> => ({ op: 'eq', value });
export const ne = <T>(value: T): Predicate<T> => ({ op: 'ne', value });
export const gt = (value: number): Predicate<number> => ({ op: 'gt', value });
export const gte = (value: number): Predicate<number> => ({ op: 'gte', value });
export const lt = (value: number): Predicate<number> => ({ op: 'lt', value });
export const lte = (value: number): Predicate<number> => ({ op: 'lte', value });
export const contains = (value: string): Predicate<string> => ({ op: 'contains', value });
export const oneOf = <T>(...values: T[]): Predicate<T> => ({ op: 'in', value: values });

export class Filter<T> implements Pattern<T[], T[]> {
  readonly __pattern = 'filter';

  constructor(
    private field: FieldPath<any>,
    private predicate: Predicate<any>
  ) {}

  toIR(): FilterIR {
    return {
      pattern: 'filter',
      predicate: {
        field: getPath(this.field),
        op: this.predicate.op,
        value: this.predicate.value
      }
    };
  }
}

/**
 * Create a type-safe filter.
 * 
 * Usage:
 *   const $ = path<UnifiedSearchResult>();
 *   filter($.relevance, gt(0.7))           // ✓ Explicit, type-safe
 *   filter($.source, eq('arxiv'))          // ✓ Type-safe string comparison
 *   filter($.source, oneOf('arxiv', 'internal'))  // ✓ Multiple values
 */
export function filter<T, V>(
  field: FieldPath<V>,
  predicate: Predicate<V>
): Filter<T> {
  return new Filter<T>(field, predicate);
}
```

### Why Predicate Builders, Not Comparison Expressions?

The problem with `(item) => item.relevance > 0.7`:

```typescript
// All of these are valid TypeScript, but unclear how to compile:

filter((item) => item.relevance > 0.7 && item.source === 'arxiv')  // Compound
filter((item) => item.tags.some(t => t.startsWith('ai')))          // Higher-order
filter((item) => computeScore(item) > threshold)                    // Function call
filter((item) => item.relevance > item.minThreshold)               // Field comparison
```

With predicate builders, **only valid operations are expressible**:

```typescript
const $ = path<UnifiedSearchResult>();

filter($.relevance, gt(0.7))              // ✓ Clear: field > value
filter($.source, eq('arxiv'))             // ✓ Clear: field === value
filter($.source, oneOf('arxiv', 'web'))   // ✓ Clear: field in [values]

// These are NOT expressible (by design):
// - Compound predicates (AND/OR) - use multiple filter steps
// - Field-to-field comparison - not supported
// - Arbitrary functions - not supported
```

The DSL is intentionally **less expressive** than TypeScript. That's the point—you can only write what can be compiled to the IR.

### Pattern: SchemaMap

```typescript
// src/patterns/schema-map.ts

/**
 * Type-safe schema mapping using PATH BUILDERS (not arbitrary functions).
 * 
 * The key insight: we don't accept `(input) => input.field` because that
 * allows arbitrary code like `input.field.toLowerCase()`. Instead, we use
 * a Proxy-based path builder that ONLY allows property navigation.
 * 
 * Write:  $.paper_title           (using path builder)
 * Emits:  "$.paper_title"         (JSONPath string)
 * 
 * REJECTED BY TYPESCRIPT:
 *   $.paper_title.toLowerCase()   // ✗ No methods on FieldPath
 *   $.paper_title + "_suffix"     // ✗ No operators
 */

/** Branded type representing a field path - NOT a value */
declare const FIELD_PATH_BRAND: unique symbol;
export interface FieldPath<T> {
  readonly [FIELD_PATH_BRAND]: T;
  readonly __path: string;
}

/** Path builder - Proxy that accumulates property access into a path */
export type PathBuilder<T> = {
  readonly [K in keyof T]-?: T[K] extends (infer U)[]
    ? PathBuilder<U> & { [index: number]: PathBuilder<U> } & FieldPath<T[K]>
    : T[K] extends object
      ? PathBuilder<T[K]> & FieldPath<T[K]>
      : FieldPath<T[K]>;
};

/**
 * Create a type-safe path builder for a given input type.
 * Returns a Proxy that tracks property access and builds JSONPath.
 */
export function path<T>(): PathBuilder<T> {
  return createPathProxy<T>('$');
}

function createPathProxy<T>(currentPath: string): PathBuilder<T> {
  return new Proxy({} as PathBuilder<T>, {
    get(_, prop) {
      if (prop === '__path') return currentPath;
      if (prop === FIELD_PATH_BRAND) return undefined;
      
      const newPath = typeof prop === 'number' || /^\d+$/.test(String(prop))
        ? `${currentPath}[${prop}]`
        : `${currentPath}.${String(prop)}`;
      
      return createPathProxy(newPath);
    }
  });
}

/** Extract the JSONPath string from a FieldPath */
function getPath<T>(fieldPath: FieldPath<T>): string {
  return (fieldPath as any).__path;
}

// ============================================
// Field source types
// ============================================

/** Field source - what to extract from input */
export type FieldSource<I, O> =
  | FieldPath<O>                                    // Path builder: $.field.subfield
  | { literal: O }                                  // Constant value
  | { coalesce: FieldPath<O | null>[] }             // First non-null from paths
  | { template: string; vars: Record<string, FieldPath<string>> }
  | { concat: FieldPath<string>[]; separator?: string }
  | SchemaMapConfig<I, O>;                          // Nested object

/** Schema mapping configuration - fully type-safe */
export type SchemaMapConfig<I, O> = {
  [K in keyof O]: FieldSource<I, O[K]>;
};

// ============================================
// SchemaMap implementation
// ============================================

export class SchemaMap<I, O> implements Pattern<I, O> {
  readonly __pattern = 'schema_map';

  constructor(
    private inputPath: PathBuilder<I>,  // The path builder for the input type
    private config: SchemaMapConfig<I, O>
  ) {}

  toIR(): SchemaMapIR {
    return {
      pattern: 'schema_map',
      mappings: this.compileConfig(this.config)
    };
  }

  private compileConfig(config: SchemaMapConfig<any, any>): Record<string, FieldSourceIR> {
    const result: Record<string, FieldSourceIR> = {};
    for (const [field, source] of Object.entries(config)) {
      result[field] = this.compileSource(source);
    }
    return result;
  }

  private compileSource(source: FieldSource<any, any>): FieldSourceIR {
    // Check if it's a FieldPath (has __path)
    if (source && typeof source === 'object' && '__path' in source) {
      return { path: (source as any).__path };
    }
    if ('literal' in source) return { literal: source.literal };
    if ('coalesce' in source) {
      return { coalesce: source.coalesce.map(p => getPath(p)) };
    }
    if ('template' in source) {
      const vars: Record<string, string> = {};
      for (const [k, p] of Object.entries(source.vars)) {
        vars[k] = getPath(p as FieldPath<string>);
      }
      return { template: source.template, vars };
    }
    if ('concat' in source) {
      return { 
        concat: source.concat.map(p => getPath(p)),
        separator: source.separator 
      };
    }
    // Nested object
    return { nested: this.compileConfig(source as SchemaMapConfig<any, any>) };
  }
}

/**
 * Create a schema mapping with type-safe path builders.
 * 
 * Usage:
 *   const $ = path<ArxivResult>();
 *   const mapping = schemaMap($, {
 *     title: $.paper_title,           // ✓ Valid path
 *     excerpt: $.abstract,            // ✓ Valid path
 *     // bad: $.abstract.toLowerCase() // ✗ TypeScript error - no methods!
 *   });
 */
export function schemaMap<I, O>(
  inputPath: PathBuilder<I>,
  config: SchemaMapConfig<I, O>
): SchemaMap<I, O> {
  return new SchemaMap<I, O>(inputPath, config);
}
```

### Why Path Builders, Not Functions?

The problem with `(input) => input.field`:

```typescript
// All of these are valid TypeScript, but can't compile to JSONPath:

schemaMap({
  title: (input) => input.paper_title.toLowerCase(),     // Method call
  url: (input) => input.pdf_url || input.arxiv_id,       // Logical operator
  excerpt: (input) => input.abstract.slice(0, 500),      // Method with args
  combined: (input) => `${input.source}: ${input.id}`,   // Template literal
  computed: (input) => input.count * 2,                  // Arithmetic
});
```

With path builders, **TypeScript itself rejects invalid code**:

```typescript
const $ = path<ArxivResult>();

schemaMap($, {
  title: $.paper_title,                    // ✓ Returns FieldPath<string>
  title: $.paper_title.toLowerCase(),      // ✗ Error: Property 'toLowerCase' does not exist on FieldPath<string>
  url: $.pdf_url || $.arxiv_id,            // ✗ Error: Operator '||' cannot be applied
  excerpt: $.abstract.slice(0, 500),       // ✗ Error: Property 'slice' does not exist on FieldPath<string>
});
```

The `FieldPath<T>` type has NO methods from `T`—only property navigation. You literally cannot write arbitrary code because TypeScript won't let you.

### Pattern: MapEach

```typescript
// src/patterns/map-each.ts

/**
 * Apply a pattern to each element of an array.
 * Composition: mapEach(A → B) = (A[] → B[])
 */

export class MapEach<I, O> implements Pattern<I[], O[]> {
  readonly __pattern = 'map_each';

  constructor(private inner: Pattern<I, O> | ToolRef<I, O>) {}

  toIR(): MapEachIR {
    return {
      pattern: 'map_each',
      inner: isToolRef(this.inner)
        ? { tool: this.inner.name }
        : { pattern: this.inner.toIR() }
    };
  }
}

export function mapEach<I, O>(inner: Pattern<I, O> | ToolRef<I, O>): MapEach<I, O> {
  return new MapEach(inner);
}
```

### Tool Reference Helper

```typescript
// src/tool.ts

/** Create a typed tool reference */
export function tool<I = JsonValue, O = JsonValue>(
  name: string
): ToolRef<I, O> {
  return {
    __brand: 'Tool',
    name,
  } as ToolRef<I, O>;
}
```

### Composition Definition

```typescript
// src/index.ts

export function composition<I = JsonValue, O = JsonValue>(
  name: string,
  description: string,
  pattern: Pattern<I, O>
): Composition<I, O> {
  const comp: any = {
    __brand: 'Composition',
    name,
    description,
    pattern,
    toIR: () => ({
      name,
      description,
      spec: pattern.toIR()
    })
  };
  return comp;
}
```

---

## Example: Research Pipeline (Heterogeneous Schema Normalization)

This example demonstrates the **deep research** use case where different search tools return different schemas. We use declarative `schemaMap` patterns to normalize them to a unified format before aggregation.

```typescript
// examples/research-pipeline.ts

import { 
  composition, pipeline, scatterGather, filter, 
  tool, schemaMap, mapEach, path, gt, eq 
} from '@vmcp/dsl';

// ============================================
// Source-specific schemas (what each API actually returns)
// These are heterogeneous and incompatible!
// ============================================

interface WebSearchResult {
  title: string;
  link: string;
  snippet: string;
  displayLink: string;     // e.g., "example.com"
}

interface ArxivResult {
  paper_title: string;
  arxiv_id: string;        // e.g., "2401.12345"
  abstract: string;
  authors: string[];
  published_date: string;
  pdf_url: string;
}

interface InternalDocResult {
  doc_name: string;
  path: string;            // Internal confluence/drive path
  excerpt: string;
  last_modified: string;
  author: string;
}

// ============================================
// Unified schema (what the pipeline works with)
// ============================================

interface UnifiedSearchResult {
  title: string;
  url: string;
  excerpt: string;
  source: 'web' | 'arxiv' | 'internal';
  relevance: number;
  timestamp: string | null;
}

interface Document {
  content: string;
  metadata: any;
}

interface Report {
  summary: string;
  sources: Document[];
}

// ============================================
// Raw tool references (return source-specific schemas)
// ============================================

const webSearchRaw = tool<string, WebSearchResult[]>('web_search');
const arxivSearchRaw = tool<string, ArxivResult[]>('arxiv_search');
const internalDocsRaw = tool<string, InternalDocResult[]>('internal_docs');
const fetchDocument = tool<string, Document>('fetch_document');
const summarize = tool<Document[], Report>('summarize');

// ============================================
// Schema mappings (type-safe path builders)
// Only property navigation allowed—no methods, no operators
// ============================================

// Create path builders for each input type
const $web = path<WebSearchResult>();
const $arxiv = path<ArxivResult>();
const $internal = path<InternalDocResult>();

const webToUnified = schemaMap($web, {
  title: $web.title,
  url: $web.link,
  excerpt: $web.snippet,
  source: { literal: 'web' },
  relevance: { literal: 0.6 },          // Base relevance for web results
  timestamp: { literal: null }
});

const arxivToUnified = schemaMap($arxiv, {
  title: $arxiv.paper_title,
  url: { coalesce: [                    // Prefer PDF URL, fallback to ID
    $arxiv.pdf_url, 
    $arxiv.arxiv_id
  ]},
  excerpt: $arxiv.abstract,
  source: { literal: 'arxiv' },
  relevance: { literal: 0.85 },         // Academic sources weighted higher
  timestamp: $arxiv.published_date
});

const internalToUnified = schemaMap($internal, {
  title: $internal.doc_name,
  url: $internal.path,
  excerpt: $internal.excerpt,
  source: { literal: 'internal' },
  relevance: { literal: 0.9 },          // Internal docs most trusted
  timestamp: $internal.last_modified
});

// ============================================
// Composed normalized tools (raw tool + schema mapping)
// ============================================

const webSearch = pipeline<string, UnifiedSearchResult[]>([
  webSearchRaw,
  mapEach(webToUnified)
]);

const arxivSearch = pipeline<string, UnifiedSearchResult[]>([
  arxivSearchRaw,
  mapEach(arxivToUnified)
]);

const internalDocs = pipeline<string, UnifiedSearchResult[]>([
  internalDocsRaw,
  mapEach(internalToUnified)
]);

// ============================================
// Full pipeline composition
// ============================================

// Path builder for the unified result type (used in aggregation and filtering)
const $result = path<UnifiedSearchResult>();

export const researchPipeline = composition<string, Report>(
  'research_pipeline',
  'Multi-source research with schema normalization, filtering, and summarization',

  pipeline([
    // Step 1: Scatter-gather with pre-normalized sources
    // Each branch normalizes its schema BEFORE aggregation
    scatterGather<string, UnifiedSearchResult[], UnifiedSearchResult>({
      targets: [webSearch, arxivSearch, internalDocs],
      aggregate: {
        ops: [
          { flatten: true },
          { sort: { by: $result.relevance, order: 'desc' } }
        ]
      }
    }),

    // Step 2: Filter to high-relevance results (explicit predicate builder)
    filter($result.relevance, gt(0.7)),

    // Step 3: Fetch full documents (parallel map over URLs)
    mapEach(fetchDocument),

    // Step 4: Summarize into final report
    summarize
  ])
);

// Compiler emits this to JSON IR (see Layer 2)
```

### Key Design Points

1. **Heterogeneous inputs, unified processing**: Each search API returns its own schema. Schema mappings normalize them *before* scatter-gather aggregation.

2. **Path builders, not functions**: Developers write `$.paper_title`, not `(input) => input.paper_title`. The `FieldPath<T>` type has no methods—you *cannot* write `$.paper_title.toLowerCase()` because TypeScript rejects it.

3. **Source-aware relevance**: Different sources get different base relevance scores baked into the mapping. Internal docs (0.9) are trusted more than random web results (0.6).

4. **Composed tools are transparent**: `webSearch`, `arxivSearch`, `internalDocs` look like simple tools to the scatter-gather pattern, but they're actually `pipeline([raw_tool, mapEach(schema_map)])` compositions.

5. **Composable aggregation ops**: Instead of hardcoded strategies, aggregation is a pipeline of ops: `flatten → sort(by: item.relevance, desc)`. Each op is parametrized by a type-safe field selector.

### Type Safety: Path Builders → JSONPath

```typescript
// Path builders make invalid states unrepresentable:
const $ = path<ArxivResult>();

schemaMap($, {
  title: $.paper_title,                     // ✓ Valid path
  url: $.nonexistent,                       // ✗ TypeScript error: Property 'nonexistent' does not exist
  bad: $.paper_title.toLowerCase(),         // ✗ TypeScript error: Property 'toLowerCase' does not exist on FieldPath
  bad: $.paper_title + "_suffix",           // ✗ TypeScript error: Operator '+' cannot be applied
});

const $r = path<UnifiedSearchResult>();
filter($r.relevance, gt(0.7));              // ✓ Explicit field + predicate
filter($r.relevance, gt);                   // ✗ TypeScript error: Expected Predicate<number>
filter($r.source, gt(0.7));                 // ✗ TypeScript error: gt() expects number, not string

// What the compiler emits (JSONPath strings):
{
  "schemaMap": {
    "mappings": {
      "title": { "path": "$.paper_title" }
    }
  },
  "filter": {
    "predicate": { "field": "$.relevance", "op": "gt", "value": 0.7 }
  }
}
```

**Key insight**: You literally *cannot* write arbitrary code because the `FieldPath<T>` type has no methods. TypeScript rejects it before you even run the compiler.

---

## Layer 2: JSON IR Schema

### IR Schema Definition (Protobuf)

```protobuf
// schema/vmcp-ir.proto

syntax = "proto3";
package vmcp.ir.v1;

// Root registry containing tools and compositions
message Registry {
  string schema_version = 1;
  repeated ToolDefinition tools = 2;
  repeated CompositionDefinition compositions = 3;
}

// Tool definition (from agentgateway registry)
message ToolDefinition {
  string name = 1;
  ToolSource source = 2;
  optional string description = 3;
  optional string input_schema = 4;  // JSON Schema
  map<string, string> defaults = 5;
  repeated string hide_fields = 6;
  optional OutputSchema output_schema = 7;
}

message ToolSource {
  string target = 1;
  string tool = 2;
}

message OutputSchema {
  map<string, OutputField> properties = 1;
}

message OutputField {
  string field_type = 1;
  optional string source_field = 2;  // JSONPath
}

// Composition definition
message CompositionDefinition {
  string name = 1;
  string description = 2;
  string input_type = 3;   // JSON Schema reference
  string output_type = 4;  // JSON Schema reference
  PatternSpec spec = 5;
  optional string version = 6;
}

// Pattern specification (oneof for each pattern type)
message PatternSpec {
  oneof pattern {
    PipelineSpec pipeline = 1;
    RouterSpec router = 2;
    ScatterGatherSpec scatter_gather = 3;
    FilterSpec filter = 4;
    EnricherSpec enricher = 5;
    SchemaMapSpec schema_map = 6;
    MapEachSpec map_each = 7;
    // ... other patterns
  }
}

// Pattern: Pipeline
message PipelineSpec {
  repeated Step steps = 1;
}

message Step {
  string id = 1;
  StepOperation operation = 2;
  DataBinding input = 3;
}

message StepOperation {
  oneof op {
    ToolCall tool = 1;
    CompositionCall composition = 2;
  }
}

message ToolCall {
  string name = 1;
}

message CompositionCall {
  string name = 1;
}

message DataBinding {
  oneof source {
    InputSource input = 1;
    StepSource step = 2;
    ConstantSource constant = 3;
  }
}

message InputSource {
  string path = 1;  // JSONPath into composition input
}

message StepSource {
  string step_id = 1;
  string path = 2;  // JSONPath into step output
}

message ConstantSource {
  string value = 1;  // JSON literal
}

// Pattern: Router
message RouterSpec {
  repeated Route routes = 1;
  optional StepOperation default = 2;
}

message Route {
  string id = 1;
  string condition = 2;  // CEL expression
  StepOperation target = 3;
}

// Pattern: Scatter-Gather
message ScatterGatherSpec {
  repeated ScatterTarget targets = 1;
  AggregationStrategy aggregation = 2;
  optional uint32 timeout_ms = 3;
  bool fail_fast = 4;
}

message ScatterTarget {
  oneof target {
    string tool = 1;                    // Tool name
    PatternSpec composition = 2;        // Inline composition (e.g., pipeline with schema_map)
  }
}

message AggregationStrategy {
  repeated AggregationOp ops = 1;       // Sequence of operations applied in order
}

message AggregationOp {
  oneof op {
    bool flatten = 1;                   // Flatten array of arrays
    SortOp sort = 2;                    // Sort by field
    DedupeOp dedupe = 3;                // Dedupe by field
    LimitOp limit = 4;                  // Take first N
    bool concat = 5;                    // Keep arrays nested
    bool merge = 6;                     // Merge objects
  }
}

message SortOp {
  string field = 1;                     // JSONPath to sort field
  string order = 2;                     // "asc" or "desc"
}

message DedupeOp {
  string field = 1;                     // JSONPath to dedupe key
}

message LimitOp {
  uint32 count = 1;
}

// Pattern: Filter (declarative predicate)
message FilterSpec {
  FieldPredicate predicate = 1;
}

message FieldPredicate {
  string field = 1;                     // JSONPath to field: "$.relevance"
  string op = 2;                        // "eq", "ne", "gt", "gte", "lt", "lte", "contains", "in"
  Value value = 3;                      // Comparison value
}

message Value {
  oneof value {
    string string_value = 1;
    double number_value = 2;
    bool bool_value = 3;
    bool null_value = 4;                // If true, represents null
    ValueList list_value = 5;           // For "in" operator
  }
}

message ValueList {
  repeated Value values = 1;
}

// Pattern: SchemaMap (declarative field mapping)
message SchemaMapSpec {
  map<string, FieldSource> mappings = 1;
}

message FieldSource {
  oneof source {
    string path = 1;                    // JSONPath extraction
    LiteralValue literal = 2;           // Constant value
    CoalesceSource coalesce = 3;        // First non-null from paths
    TemplateSource template = 4;        // String interpolation
    ConcatSource concat = 5;            // Concatenate fields
    SchemaMapSpec nested = 6;           // Nested object mapping
  }
}

message LiteralValue {
  oneof value {
    string string_value = 1;
    double number_value = 2;
    bool bool_value = 3;
    bool null_value = 4;                // If true, value is null
  }
}

message CoalesceSource {
  repeated string paths = 1;            // Try each JSONPath, return first non-null
}

message TemplateSource {
  string template = 1;                  // e.g., "{source}:{id}"
  map<string, string> vars = 2;         // Variable name -> JSONPath binding
}

message ConcatSource {
  repeated string paths = 1;
  optional string separator = 2;
}

// Pattern: MapEach (apply operation to array elements)
message MapEachSpec {
  MapEachInner inner = 1;
}

message MapEachInner {
  oneof inner {
    string tool = 1;                    // Tool name
    PatternSpec pattern = 2;            // Nested pattern (e.g., schema_map)
  }
}

// Pattern: Enricher
message EnricherSpec {
  repeated Enrichment enrichments = 1;
  string merge_strategy = 2;
}

message Enrichment {
  string field = 1;
  StepOperation source = 2;
}
```

### Example: Research Pipeline IR (JSON)

This JSON IR demonstrates heterogeneous schema normalization. Note how each search source has its own `pipeline` composition that includes a `schemaMap` to normalize to `UnifiedSearchResult`.

```json
{
  "schemaVersion": "1.0",
  "compositions": [
    {
      "name": "research_pipeline",
      "description": "Multi-source research with schema normalization, filtering, and summarization",
      "inputType": "string",
      "outputType": "Report",
      "spec": {
        "pipeline": {
          "steps": [
            {
              "id": "step_0",
              "operation": {
                "scatterGather": {
                  "targets": [
                    { "composition": "__web_normalized" },
                    { "composition": "__arxiv_normalized" },
                    { "composition": "__internal_normalized" }
                  ],
                  "aggregation": {
                    "ops": [
                      { "flatten": true },
                      { "sort": { "field": "$.relevance", "order": "desc" } }
                    ]
                  },
                  "timeoutMs": 15000,
                  "failFast": false
                }
              },
              "input": { "input": { "path": "$" } }
            },
            {
              "id": "step_1",
              "operation": {
                "filter": {
                  "predicate": {
                    "field": "$.relevance",
                    "op": "gt",
                    "value": { "numberValue": 0.7 }
                  }
                }
              },
              "input": { "step": { "stepId": "step_0", "path": "$" } }
            },
            {
              "id": "step_2",
              "operation": {
                "mapEach": {
                  "inner": { "tool": "fetch_document" }
                }
              },
              "input": { "step": { "stepId": "step_1", "path": "$" } }
            },
            {
              "id": "step_3",
              "operation": { "tool": { "name": "summarize" } },
              "input": { "step": { "stepId": "step_2", "path": "$" } }
            }
          ]
        }
      }
    },
    {
      "name": "__web_normalized",
      "description": "Web search with schema normalization to UnifiedSearchResult",
      "inputType": "string",
      "outputType": "UnifiedSearchResult[]",
      "spec": {
        "pipeline": {
          "steps": [
            {
              "id": "step_0",
              "operation": { "tool": { "name": "web_search" } },
              "input": { "input": { "path": "$" } }
            },
            {
              "id": "step_1",
              "operation": {
                "mapEach": {
                  "inner": {
                    "pattern": {
                      "schemaMap": {
                        "mappings": {
                          "title": { "path": "$.title" },
                          "url": { "path": "$.link" },
                          "excerpt": { "path": "$.snippet" },
                          "source": { "literal": { "stringValue": "web" } },
                          "relevance": { "literal": { "numberValue": 0.6 } },
                          "timestamp": { "literal": { "nullValue": true } }
                        }
                      }
                    }
                  }
                }
              },
              "input": { "step": { "stepId": "step_0", "path": "$" } }
            }
          ]
        }
      }
    },
    {
      "name": "__arxiv_normalized",
      "description": "arXiv search with schema normalization to UnifiedSearchResult",
      "inputType": "string",
      "outputType": "UnifiedSearchResult[]",
      "spec": {
        "pipeline": {
          "steps": [
            {
              "id": "step_0",
              "operation": { "tool": { "name": "arxiv_search" } },
              "input": { "input": { "path": "$" } }
            },
            {
              "id": "step_1",
              "operation": {
                "mapEach": {
                  "inner": {
                    "pattern": {
                      "schemaMap": {
                        "mappings": {
                          "title": { "path": "$.paper_title" },
                          "url": { "coalesce": { "paths": ["$.pdf_url", "$.arxiv_id"] } },
                          "excerpt": { "path": "$.abstract" },
                          "source": { "literal": { "stringValue": "arxiv" } },
                          "relevance": { "literal": { "numberValue": 0.85 } },
                          "timestamp": { "path": "$.published_date" }
                        }
                      }
                    }
                  }
                }
              },
              "input": { "step": { "stepId": "step_0", "path": "$" } }
            }
          ]
        }
      }
    },
    {
      "name": "__internal_normalized",
      "description": "Internal docs search with schema normalization to UnifiedSearchResult",
      "inputType": "string",
      "outputType": "UnifiedSearchResult[]",
      "spec": {
        "pipeline": {
          "steps": [
            {
              "id": "step_0",
              "operation": { "tool": { "name": "internal_docs" } },
              "input": { "input": { "path": "$" } }
            },
            {
              "id": "step_1",
              "operation": {
                "mapEach": {
                  "inner": {
                    "pattern": {
                      "schemaMap": {
                        "mappings": {
                          "title": { "path": "$.doc_name" },
                          "url": { "path": "$.path" },
                          "excerpt": { "path": "$.excerpt" },
                          "source": { "literal": { "stringValue": "internal" } },
                          "relevance": { "literal": { "numberValue": 0.9 } },
                          "timestamp": { "path": "$.last_modified" }
                        }
                      }
                    }
                  }
                }
              },
              "input": { "step": { "stepId": "step_0", "path": "$" } }
            }
          ]
        }
      }
    }
  ]
}
```

---

## Layer 3: Rust Runtime (Agentgateway)

### Module Structure

```
crates/agentgateway/src/mcp/compositions/
├── mod.rs
├── ir.rs              # IR types (from proto)
├── executor.rs        # Main execution engine
├── patterns/
│   ├── mod.rs
│   ├── pipeline.rs
│   ├── router.rs
│   ├── scatter_gather.rs
│   ├── filter.rs
│   ├── schema_map.rs  # Declarative field mapping
│   ├── map_each.rs    # Apply pattern to array elements
│   └── ... (other patterns)
├── context.rs         # Execution context
├── data_flow.rs       # Data binding resolution
└── registry.rs        # Composition registry
```

### IR Types (Rust)

```rust
// src/mcp/compositions/ir.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionRegistry {
    pub schema_version: String,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
    #[serde(default)]
    pub compositions: Vec<CompositionDefinition>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionDefinition {
    pub name: String,
    pub description: String,
    pub input_type: String,
    pub output_type: String,
    pub spec: PatternSpec,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PatternSpec {
    Pipeline(PipelineSpec),
    Router(RouterSpec),
    ScatterGather(ScatterGatherSpec),
    Filter(FilterSpec),
    Enricher(EnricherSpec),
    SchemaMap(SchemaMapSpec),
    MapEach(MapEachSpec),
    // ... other patterns
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PipelineSpec {
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Step {
    pub id: String,
    pub operation: StepOperation,
    pub input: DataBinding,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StepOperation {
    Tool { name: String },
    Composition { name: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DataBinding {
    Input { path: String },
    Step { step_id: String, path: String },
    Constant { value: serde_json::Value },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScatterGatherSpec {
    pub targets: Vec<ScatterTarget>,
    pub aggregation: AggregationStrategy,
    pub timeout_ms: Option<u32>,
    pub fail_fast: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScatterTarget {
    Tool(String),
    Composition(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationStrategy {
    pub ops: Vec<AggregationOp>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AggregationOp {
    Flatten(bool),
    Sort(SortOp),
    Dedupe(DedupeOp),
    Limit(LimitOp),
    Concat(bool),
    Merge(bool),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SortOp {
    pub field: String,    // JSONPath
    pub order: String,    // "asc" or "desc"
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DedupeOp {
    pub field: String,    // JSONPath to dedupe key
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitOp {
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouterSpec {
    pub routes: Vec<Route>,
    pub default: Option<StepOperation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Route {
    pub id: String,
    pub condition: String, // CEL expression
    pub target: StepOperation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterSpec {
    pub predicate: FieldPredicate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldPredicate {
    pub field: String,       // JSONPath to field
    pub op: String,          // "eq", "ne", "gt", "gte", "lt", "lte", "contains", "in"
    pub value: serde_json::Value,
}

// ============================================
// Schema Mapping Types
// ============================================

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaMapSpec {
    pub mappings: HashMap<String, FieldSource>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldSource {
    Path(String),                       // JSONPath extraction
    Literal(LiteralValue),              // Constant value
    Coalesce(CoalesceSource),           // First non-null from paths
    Template(TemplateSource),           // String interpolation
    Concat(ConcatSource),               // Concatenate fields
    Nested(SchemaMapSpec),              // Nested object mapping
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LiteralValue {
    StringValue(String),
    NumberValue(f64),
    BoolValue(bool),
    NullValue,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoalesceSource {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemplateSource {
    pub template: String,
    pub vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConcatSource {
    pub paths: Vec<String>,
    pub separator: Option<String>,
}

// ============================================
// MapEach Types
// ============================================

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapEachSpec {
    pub inner: MapEachInner,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MapEachInner {
    Tool(String),
    Pattern(Box<PatternSpec>),
}
```

### Executor

```rust
// src/mcp/compositions/executor.rs

use super::ir::*;
use super::patterns::*;
use super::context::ExecutionContext;
use crate::mcp::registry::RegistryStoreRef;
use anyhow::Result;
use std::sync::Arc;

pub struct CompositionExecutor {
    registry: RegistryStoreRef,
    patterns: Arc<PatternLibrary>,
}

impl CompositionExecutor {
    pub fn new(registry: RegistryStoreRef) -> Self {
        Self {
            registry,
            patterns: Arc::new(PatternLibrary::default()),
        }
    }

    /// Execute a composition with given inputs
    pub async fn execute(
        &self,
        composition_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Load composition definition
        let registry = self.registry.read().await;
        let composition = registry
            .get_composition(composition_name)
            .ok_or_else(|| anyhow::anyhow!("Composition not found: {}", composition_name))?;

        // Create execution context
        let mut ctx = ExecutionContext::new(input, composition.clone());

        // Execute based on pattern
        match &composition.spec {
            PatternSpec::Pipeline(spec) => {
                self.patterns.pipeline.execute(spec, &mut ctx, self).await
            }
            PatternSpec::Router(spec) => {
                self.patterns.router.execute(spec, &mut ctx, self).await
            }
            PatternSpec::ScatterGather(spec) => {
                self.patterns.scatter_gather.execute(spec, &mut ctx, self).await
            }
            PatternSpec::Filter(spec) => {
                self.patterns.filter.execute(spec, &mut ctx, self).await
            }
            PatternSpec::Enricher(spec) => {
                self.patterns.enricher.execute(spec, &mut ctx, self).await
            }
            PatternSpec::SchemaMap(spec) => {
                self.patterns.schema_map.execute(spec, &mut ctx, self).await
            }
            PatternSpec::MapEach(spec) => {
                self.patterns.map_each.execute(spec, &mut ctx, self).await
            }
        }
    }

    /// Execute a single step (tool or nested composition)
    pub(crate) async fn execute_step(
        &self,
        operation: &StepOperation,
        input: serde_json::Value,
        ctx: &mut ExecutionContext,
    ) -> Result<serde_json::Value> {
        match operation {
            StepOperation::Tool { name } => {
                // Invoke tool through existing MCP handler
                self.invoke_tool(name, input, ctx).await
            }
            StepOperation::Composition { name } => {
                // Recursively execute nested composition
                self.execute(name, input).await
            }
        }
    }

    async fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        ctx: &mut ExecutionContext,
    ) -> Result<serde_json::Value> {
        // Use existing agentgateway MCP handler
        let registry = self.registry.read().await;

        // Apply virtual tool transformations if applicable
        let (target, actual_tool, transformed_input) = registry
            .prepare_tool_call(tool_name, input)?;

        // Invoke backend (this connects to existing agentgateway backend pool)
        let response = ctx
            .backend_pool
            .invoke(&target, &actual_tool, transformed_input)
            .await?;

        // Apply output transformation
        let transformed_output = registry.transform_output(tool_name, response)?;

        Ok(transformed_output)
    }
}
```

### Pattern: Pipeline

```rust
// src/mcp/compositions/patterns/pipeline.rs

use super::*;

pub struct PipelinePattern;

#[async_trait::async_trait]
impl PatternExecutor for PipelinePattern {
    async fn execute(
        &self,
        spec: &PipelineSpec,
        ctx: &mut ExecutionContext,
        executor: &CompositionExecutor,
    ) -> Result<serde_json::Value> {
        let mut current_value = ctx.input.clone();

        for step in &spec.steps {
            // Resolve input for this step
            let step_input = ctx.resolve_binding(&step.input, &current_value)?;

            // Execute step
            current_value = executor.execute_step(&step.operation, step_input, ctx).await?;

            // Store result in context for future step references
            ctx.set_step_result(&step.id, current_value.clone());

            // Emit trace event
            ctx.trace_step(&step.id, &step.operation, &current_value);
        }

        Ok(current_value)
    }
}
```

### Pattern: Scatter-Gather

```rust
// src/mcp/compositions/patterns/scatter_gather.rs

use futures::future::join_all;
use tokio::time::{timeout, Duration};

pub struct ScatterGatherPattern;

#[async_trait::async_trait]
impl PatternExecutor for ScatterGatherPattern {
    async fn execute(
        &self,
        spec: &ScatterGatherSpec,
        ctx: &mut ExecutionContext,
        executor: &CompositionExecutor,
    ) -> Result<serde_json::Value> {
        let input = ctx.input.clone();

        // Create futures for all targets (tools or compositions)
        let futures: Vec<_> = spec
            .targets
            .iter()
            .map(|target| {
                let op = match target {
                    ScatterTarget::Tool(name) => StepOperation::Tool { name: name.clone() },
                    ScatterTarget::Composition(name) => StepOperation::Composition { name: name.clone() },
                };
                let input = input.clone();
                let mut ctx_clone = ctx.clone();

                async move {
                    executor.execute_step(&op, input, &mut ctx_clone).await
                }
            })
            .collect();

        // Execute in parallel with optional timeout
        let results = if let Some(timeout_ms) = spec.timeout_ms {
            let duration = Duration::from_millis(timeout_ms as u64);
            timeout(duration, join_all(futures))
                .await
                .map_err(|_| anyhow::anyhow!("Scatter-gather timeout"))?
        } else {
            join_all(futures).await
        };

        // Collect successful results (or fail fast)
        let mut collected = Vec::new();
        for (idx, result) in results.into_iter().enumerate() {
            match result {
                Ok(value) => collected.push(value),
                Err(e) if spec.fail_fast => return Err(e),
                Err(e) => {
                    tracing::warn!("Target {:?} failed: {}", spec.targets[idx], e);
                }
            }
        }

        // Apply aggregation strategy (named builtins only)
        let aggregated = self.aggregate(&spec.aggregation, collected)?;

        Ok(aggregated)
    }
}

impl ScatterGatherPattern {
    fn aggregate(
        &self,
        strategy: &AggregationStrategy,
        results: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // Start with results as the working value
        let mut current = serde_json::json!(results);

        // Apply each operation in sequence
        for op in &strategy.ops {
            current = self.apply_op(op, current)?;
        }

        Ok(current)
    }

    fn apply_op(
        &self,
        op: &AggregationOp,
        value: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match op {
            AggregationOp::Flatten(_) => {
                // Flatten array of arrays into single array
                let arr = value.as_array()
                    .ok_or_else(|| anyhow::anyhow!("Flatten requires array input"))?;
                let mut flattened = Vec::new();
                for item in arr {
                    if let Some(inner) = item.as_array() {
                        flattened.extend(inner.clone());
                    } else {
                        flattened.push(item.clone());
                    }
                }
                Ok(serde_json::json!(flattened))
            }

            AggregationOp::Sort(sort_op) => {
                let mut arr = value.as_array()
                    .ok_or_else(|| anyhow::anyhow!("Sort requires array input"))?
                    .clone();
                
                arr.sort_by(|a, b| {
                    let a_val = jsonpath_lib::select(a, &sort_op.field)
                        .ok()
                        .and_then(|v| v.into_iter().next().cloned())
                        .and_then(|v| v.as_f64());
                    let b_val = jsonpath_lib::select(b, &sort_op.field)
                        .ok()
                        .and_then(|v| v.into_iter().next().cloned())
                        .and_then(|v| v.as_f64());
                    
                    match (a_val, b_val) {
                        (Some(a), Some(b)) => {
                            if sort_op.order == "desc" {
                                b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
                            } else {
                                a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
                            }
                        }
                        _ => std::cmp::Ordering::Equal,
                    }
                });
                Ok(serde_json::json!(arr))
            }

            AggregationOp::Dedupe(dedupe_op) => {
                let arr = value.as_array()
                    .ok_or_else(|| anyhow::anyhow!("Dedupe requires array input"))?;
                
                let mut seen = std::collections::HashSet::new();
                let mut deduped = Vec::new();
                
                for item in arr {
                    let key = jsonpath_lib::select(item, &dedupe_op.field)
                        .ok()
                        .and_then(|v| v.into_iter().next().cloned())
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    
                    if seen.insert(key) {
                        deduped.push(item.clone());
                    }
                }
                Ok(serde_json::json!(deduped))
            }

            AggregationOp::Limit(limit_op) => {
                let arr = value.as_array()
                    .ok_or_else(|| anyhow::anyhow!("Limit requires array input"))?;
                let limited: Vec<_> = arr.iter()
                    .take(limit_op.count as usize)
                    .cloned()
                    .collect();
                Ok(serde_json::json!(limited))
            }

            AggregationOp::Concat(_) => {
                // Keep as-is (array of arrays)
                Ok(value)
            }

            AggregationOp::Merge(_) => {
                let arr = value.as_array()
                    .ok_or_else(|| anyhow::anyhow!("Merge requires array input"))?;
                let mut merged = serde_json::Map::new();
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        merged.extend(obj.clone());
                    }
                }
                Ok(serde_json::Value::Object(merged))
            }
        }
    }
}
```

### Pattern: Router

```rust
// src/mcp/compositions/patterns/router.rs

pub struct RouterPattern;

#[async_trait::async_trait]
impl PatternExecutor for RouterPattern {
    async fn execute(
        &self,
        spec: &RouterSpec,
        ctx: &mut ExecutionContext,
        executor: &CompositionExecutor,
    ) -> Result<serde_json::Value> {
        let input = ctx.input.clone();

        // Evaluate conditions in order
        for route in &spec.routes {
            // Evaluate CEL condition
            let cel_ctx = serde_json::json!({ "input": input });
            let matches = ctx.evaluate_cel_bool(&route.condition, &cel_ctx)?;

            if matches {
                // Execute target
                return executor.execute_step(&route.target, input, ctx).await;
            }
        }

        // No route matched, use default or error
        if let Some(default_op) = &spec.default {
            executor.execute_step(default_op, input, ctx).await
        } else {
            Err(anyhow::anyhow!("No matching route and no default specified"))
        }
    }
}
```

### Pattern: Filter

```rust
// src/mcp/compositions/patterns/filter.rs

pub struct FilterPattern;

#[async_trait::async_trait]
impl PatternExecutor for FilterPattern {
    async fn execute(
        &self,
        spec: &FilterSpec,
        ctx: &mut ExecutionContext,
        _executor: &CompositionExecutor,
    ) -> Result<serde_json::Value> {
        let input = ctx.input.clone();

        // Input must be an array
        let items = input
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Filter requires array input"))?;

        // Filter using declarative field predicate
        let mut filtered = Vec::new();
        for item in items {
            if self.evaluate_predicate(&spec.predicate, item)? {
                filtered.push(item.clone());
            }
        }

        Ok(serde_json::json!(filtered))
    }
}

impl FilterPattern {
    fn evaluate_predicate(
        &self,
        pred: &FieldPredicate,
        item: &serde_json::Value,
    ) -> Result<bool> {
        // Extract field value using JSONPath
        let field_value = jsonpath_lib::select(item, &pred.field)?
            .into_iter()
            .next()
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // Compare based on operator
        let result = match pred.op.as_str() {
            "eq" => field_value == pred.value,
            "ne" => field_value != pred.value,
            "gt" => self.compare_numeric(&field_value, &pred.value, |a, b| a > b),
            "gte" => self.compare_numeric(&field_value, &pred.value, |a, b| a >= b),
            "lt" => self.compare_numeric(&field_value, &pred.value, |a, b| a < b),
            "lte" => self.compare_numeric(&field_value, &pred.value, |a, b| a <= b),
            "contains" => {
                if let (Some(s), Some(needle)) = (field_value.as_str(), pred.value.as_str()) {
                    s.contains(needle)
                } else {
                    false
                }
            }
            "in" => {
                if let Some(arr) = pred.value.as_array() {
                    arr.contains(&field_value)
                } else {
                    false
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown predicate operator: {}", pred.op)),
        };

        Ok(result)
    }

    fn compare_numeric<F>(&self, a: &serde_json::Value, b: &serde_json::Value, cmp: F) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        match (a.as_f64(), b.as_f64()) {
            (Some(a_num), Some(b_num)) => cmp(a_num, b_num),
            _ => false,
        }
    }
}
```

### Pattern: SchemaMap

```rust
// src/mcp/compositions/patterns/schema_map.rs

use jsonpath_lib as jsonpath;
use serde_json::{Value, Map};

pub struct SchemaMapPattern;

#[async_trait::async_trait]
impl PatternExecutor for SchemaMapPattern {
    async fn execute(
        &self,
        spec: &SchemaMapSpec,
        ctx: &mut ExecutionContext,
        _executor: &CompositionExecutor,
    ) -> Result<serde_json::Value> {
        let input = ctx.input.clone();
        self.apply_mapping(&input, spec)
    }
}

impl SchemaMapPattern {
    fn apply_mapping(
        &self,
        input: &Value,
        spec: &SchemaMapSpec,
    ) -> Result<Value> {
        let mut output = Map::new();

        for (target_field, source) in &spec.mappings {
            let value = self.extract_field(input, source)?;
            output.insert(target_field.clone(), value);
        }

        Ok(Value::Object(output))
    }

    fn extract_field(&self, input: &Value, source: &FieldSource) -> Result<Value> {
        match source {
            FieldSource::Path(path) => {
                // JSONPath extraction
                jsonpath::select(input, path)?
                    .into_iter()
                    .next()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Path {} not found", path))
            }

            FieldSource::Literal(lit) => Ok(match lit {
                LiteralValue::StringValue(s) => Value::String(s.clone()),
                LiteralValue::NumberValue(n) => serde_json::json!(n),
                LiteralValue::BoolValue(b) => Value::Bool(*b),
                LiteralValue::NullValue => Value::Null,
            }),

            FieldSource::Coalesce(coalesce) => {
                // Return first non-null value from paths
                for path in &coalesce.paths {
                    if let Ok(Some(val)) = jsonpath::select(input, path)
                        .map(|v| v.into_iter().next().cloned())
                    {
                        if !val.is_null() {
                            return Ok(val);
                        }
                    }
                }
                Ok(Value::Null)
            }

            FieldSource::Template(tmpl) => {
                // String interpolation
                let mut result = tmpl.template.clone();
                for (var, path) in &tmpl.vars {
                    let val = jsonpath::select(input, path)?
                        .into_iter()
                        .next()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default();
                    result = result.replace(&format!("{{{}}}", var), &val);
                }
                Ok(Value::String(result))
            }

            FieldSource::Concat(concat) => {
                // Concatenate multiple fields
                let parts: Vec<String> = concat
                    .paths
                    .iter()
                    .filter_map(|p| {
                        jsonpath::select(input, p).ok()?
                            .into_iter()
                            .next()?
                            .as_str()
                            .map(String::from)
                    })
                    .collect();
                let separator = concat.separator.as_deref().unwrap_or("");
                Ok(Value::String(parts.join(separator)))
            }

            FieldSource::Nested(nested_spec) => {
                // Recursively apply nested mapping
                self.apply_mapping(input, nested_spec)
            }
        }
    }
}
```

### Pattern: MapEach

```rust
// src/mcp/compositions/patterns/map_each.rs

pub struct MapEachPattern;

#[async_trait::async_trait]
impl PatternExecutor for MapEachPattern {
    async fn execute(
        &self,
        spec: &MapEachSpec,
        ctx: &mut ExecutionContext,
        executor: &CompositionExecutor,
    ) -> Result<serde_json::Value> {
        let input = ctx.input.clone();

        // Input must be an array
        let items = input
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("MapEach requires array input"))?;

        // Apply inner operation to each element
        let mut results = Vec::with_capacity(items.len());
        
        for item in items {
            let result = match &spec.inner {
                MapEachInner::Tool(name) => {
                    let op = StepOperation::Tool { name: name.clone() };
                    executor.execute_step(&op, item.clone(), ctx).await?
                }
                MapEachInner::Pattern(pattern) => {
                    // Execute nested pattern with item as input
                    let mut item_ctx = ctx.clone();
                    item_ctx.input = item.clone();
                    executor.execute_pattern(pattern, &mut item_ctx).await?
                }
            };
            results.push(result);
        }

        Ok(serde_json::json!(results))
    }
}
```

### Execution Context

```rust
// src/mcp/compositions/context.rs

use std::collections::HashMap;

pub struct ExecutionContext {
    pub input: serde_json::Value,
    pub composition: CompositionDefinition,
    pub step_results: HashMap<String, serde_json::Value>,
    pub backend_pool: Arc<BackendPool>, // From agentgateway
    pub cel_engine: Arc<CelEngine>,     // From agentgateway
    pub tracer: Arc<Tracer>,            // From agentgateway
}

impl ExecutionContext {
    pub fn new(input: serde_json::Value, composition: CompositionDefinition) -> Self {
        Self {
            input,
            composition,
            step_results: HashMap::new(),
            // ... injected from agentgateway
        }
    }

    /// Resolve data binding to actual value
    pub fn resolve_binding(
        &self,
        binding: &DataBinding,
        current_value: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        match binding {
            DataBinding::Input { path } => {
                // JSONPath into original input
                jsonpath::select(&self.input, path)
            }
            DataBinding::Step { step_id, path } => {
                // JSONPath into previous step result
                let step_result = self
                    .step_results
                    .get(step_id)
                    .ok_or_else(|| anyhow::anyhow!("Step not found: {}", step_id))?;
                jsonpath::select(step_result, path)
            }
            DataBinding::Constant { value } => Ok(value.clone()),
        }
    }

    pub fn set_step_result(&mut self, step_id: &str, value: serde_json::Value) {
        self.step_results.insert(step_id.to_string(), value);
    }

    pub fn evaluate_cel_bool(&self, expr: &str, ctx: &serde_json::Value) -> Result<bool> {
        self.cel_engine.evaluate_bool(expr, ctx)
    }

    pub fn trace_step(&self, step_id: &str, op: &StepOperation, result: &serde_json::Value) {
        self.tracer.record_step(step_id, op, result);
    }
}
```

---

## Integration with Agentgateway

### Configuration

```yaml
# config.yaml (extended)
registry:
  source: "file:///etc/agentgateway/registry.json"
  refreshInterval: "5m"
```

### Registry JSON (Extended)

```json
{
  "schemaVersion": "1.0",
  "tools": [
    {
      "name": "web_search",
      "source": { "target": "search-backend", "tool": "search" }
    },
    {
      "name": "arxiv_search",
      "source": { "target": "arxiv-backend", "tool": "search" }
    },
    {
      "name": "fetch_document",
      "source": { "target": "web-backend", "tool": "fetch" }
    },
    {
      "name": "summarize",
      "source": { "target": "llm-backend", "tool": "summarize" }
    }
  ],
  "compositions": [
    {
      "name": "research_pipeline",
      "description": "Multi-source research with filtering",
      "inputType": "string",
      "outputType": "Report",
      "spec": {
        "pipeline": {
          "steps": [
            {
              "id": "step_0",
              "operation": { "composition": { "name": "__scatter_gather_0" } },
              "input": { "input": { "path": "$" } }
            },
            {
              "id": "step_1",
              "operation": { "composition": { "name": "__filter_1" } },
              "input": { "step": { "stepId": "step_0", "path": "$" } }
            },
            {
              "id": "step_2",
              "operation": { "tool": { "name": "fetch_document" } },
              "input": { "step": { "stepId": "step_1", "path": "$[*].url" } }
            },
            {
              "id": "step_3",
              "operation": { "tool": { "name": "summarize" } },
              "input": { "step": { "stepId": "step_2", "path": "$" } }
            }
          ]
        }
      }
    },
    {
      "name": "__scatter_gather_0",
      "spec": {
        "scatterGather": {
          "targets": ["web_search", "arxiv_search", "internal_docs"],
          "aggregation": { "function": "flatten_and_sort_by_relevance" },
          "timeoutMs": 10000,
          "failFast": false
        }
      }
    },
    {
      "name": "__filter_1",
      "spec": {
        "filter": {
          "predicate": "item.relevance > 0.7"
        }
      }
    }
  ]
}
```

### MCP Handler Extension

```rust
// src/mcp/handler.rs (modified)

impl McpHandler {
    pub async fn handle_tool_call(&self, tool_name: &str, args: JsonValue) -> Result<JsonValue> {
        // Check if this is a composition or a tool
        if self.composition_executor.has_composition(tool_name) {
            // Execute as composition
            self.composition_executor.execute(tool_name, args).await
        } else {
            // Execute as regular tool (existing logic)
            self.execute_tool(tool_name, args).await
        }
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let mut tools = Vec::new();

        // Add regular tools from registry
        tools.extend(self.registry.list_tools());

        // Add compositions as tools (they're invokable like any tool)
        tools.extend(self.composition_executor.list_compositions());

        Ok(tools)
    }
}
```

---

## Developer Workflow

### 1. Write Composition (TypeScript)

```bash
# Create new composition
mkdir my-compositions
cd my-compositions
npm init -y
npm install @vmcp/dsl

# Write composition
cat > src/research-pipeline.ts << 'EOF'
import { composition, pipeline, scatterGather, filter, tool } from '@vmcp/dsl';
// ... (code from example above)
EOF
```

### 2. Compile to JSON IR

```bash
# Compile TypeScript to JSON IR
npx vmcp-compile src/research-pipeline.ts -o dist/registry.json

# Output:
# ✓ Compiled research_pipeline
# ✓ Generated 3 compositions (1 top-level, 2 internal)
# ✓ Written to dist/registry.json
```

### 3. Deploy to Agentgateway

```bash
# Copy to agentgateway config directory
cp dist/registry.json /etc/agentgateway/registry.json

# Restart agentgateway (or wait for hot-reload)
# Registry refreshes every 5m by default
```

### 4. Invoke

```bash
# Agent can now discover and invoke
curl -X POST http://localhost:15000/mcp/tools/call \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "research_pipeline",
    "arguments": { "topic": "quantum computing" }
  }'

# Agentgateway orchestrates:
# 1. Scatter-gather to web_search, arxiv_search, internal_docs (parallel)
#    - Each branch normalizes its source-specific schema to UnifiedSearchResult
# 2. Aggregation pipeline: flatten → sort($.relevance, desc)
# 3. Filter where $.relevance > 0.7 (declarative predicate)
# 4. Fetch documents (parallel map via mapEach)
# 5. Summarize into report
# 6. Return to agent
```

---

## Summary: The Complete Pathway

### Layer 1: TypeScript DSL
- Type-safe composition API
- Pattern builders: `pipeline`, `router`, `scatterGather`, `filter`, `schemaMap`, `mapEach`
- **Path builders**: Write `$.field`, not `(x) => x.field`—`FieldPath<T>` has no methods, so you *cannot* write arbitrary code
- Compiles to JSON IR

### Layer 2: JSON IR
- Protobuf schema (language-agnostic)
- Versionable, portable
- Human-readable, machine-parseable
- All transformations are data (field mappings, not code)

### Layer 3: Rust Runtime
- Embedded in agentgateway
- Pattern executors for each algebra operator
- **Safe execution**: JSONPath extraction, builtin functions—no embedded interpreters
- Leverages existing infrastructure (backends, tracing)

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Path builders, not lambdas** | `$.field` instead of `(x) => x.field`—`FieldPath<T>` has no methods, so arbitrary code is impossible |
| **Composable aggregation ops** | `flatten`, `sort`, `dedupe`, `limit`—parametrized by `FieldPath`, not hardcoded combos |
| **Explicit predicate builders** | `filter($.relevance, gt(0.7))` not `filter(x => x.relevance > 0.7)`—no parsing, no ambiguity |
| **Schema mappings as data** | Transforms are config, not code—portable across runtimes |
| **Composition targets in scatter-gather** | Normalized branches look like simple tools |

### Why This Works

1. **Separation of concerns**: Write logic in TS, execute in Rust
2. **Type safety**: TS compiler + JSON Schema validation
3. **Performance**: Rust execution, parallel by default
4. **Portability**: JSON IR can be consumed by any runtime
5. **Safety**: No embedded interpreters—all logic is declarative config
6. **Schema normalization**: Handle heterogeneous tool outputs at the composition layer, not in LLM context

This design gives you the **TypeScript → JSON IR → Rust runtime** pathway while staying true to the tool algebra concept—with a **fully declarative** approach that avoids the complexity of embedded code execution.
