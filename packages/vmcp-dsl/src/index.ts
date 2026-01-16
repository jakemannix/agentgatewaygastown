/**
 * @vmcp/dsl - TypeScript DSL for vMCP tool compositions
 *
 * This package provides a type-safe DSL for defining tool compositions
 * that compile to the JSON IR format used by agentgateway.
 *
 * @example
 * import { tool, pipeline, scatterGather, filter, schemaMap, mapEach, compile } from '@vmcp/dsl';
 *
 * // Define a virtual tool (1:1 mapping)
 * const weatherTool = tool('get_weather')
 *   .description('Get weather information')
 *   .source('weather', 'fetch_weather')
 *   .default('units', 'metric')
 *   .build();
 *
 * // Define a composition (N:1 mapping)
 * const researchPipeline = tool('research')
 *   .description('Multi-source research pipeline')
 *   .composition(
 *     pipeline()
 *       .step('search', 'web_search')
 *       .step('summarize', 'summarize_text')
 *       .build()
 *   )
 *   .build();
 *
 * // Compile to JSON
 * const json = compile(weatherTool, researchPipeline);
 */

// Core types
export * from './types.js';

// Path builder
export * from './path-builder.js';

// Tool builder
export { tool, ToolBuilder, SourceToolBuilder, CompositionBuilder } from './tool.js';

// Pattern builders
export {
  // Pipeline
  pipeline,
  step,
  PipelineBuilder,
  StepBuilder,
  // Scatter-gather
  scatterGather,
  agg,
  ScatterGatherBuilder,
  AggregationBuilder,
  // Filter
  filter,
  filterBy,
  FilterBuilder,
  // Schema map
  schemaMap,
  outputTransform,
  SchemaMapBuilder,
  // Map each
  mapEach,
  mapEachTool,
  mapEachPattern,
  MapEachBuilder,
} from './patterns/index.js';

// Compiler
export {
  createRegistry,
  compile,
  parseRegistry,
  RegistryBuilder,
  type ValidationResult,
  type ValidationError,
  type ValidationWarning,
} from './compiler.js';

