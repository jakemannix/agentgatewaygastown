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
export * from './types.js';
export * from './path-builder.js';
export { tool, ToolBuilder, SourceToolBuilder, CompositionBuilder } from './tool.js';
export { pipeline, step, PipelineBuilder, StepBuilder, scatterGather, agg, ScatterGatherBuilder, AggregationBuilder, filter, filterBy, FilterBuilder, schemaMap, outputTransform, SchemaMapBuilder, mapEach, mapEachTool, mapEachPattern, MapEachBuilder, } from './patterns/index.js';
export { createRegistry, compile, parseRegistry, RegistryBuilder, type ValidationResult, type ValidationError, type ValidationWarning, } from './compiler.js';
//# sourceMappingURL=index.d.ts.map