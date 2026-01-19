"use strict";
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
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports, p)) __createBinding(exports, m, p);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.RegistryBuilder = exports.parseRegistry = exports.compile = exports.createRegistry = exports.MapEachBuilder = exports.mapEachPattern = exports.mapEachTool = exports.mapEach = exports.SchemaMapBuilder = exports.outputTransform = exports.schemaMap = exports.FilterBuilder = exports.filterBy = exports.filter = exports.AggregationBuilder = exports.ScatterGatherBuilder = exports.agg = exports.scatterGather = exports.StepBuilder = exports.PipelineBuilder = exports.step = exports.pipeline = exports.CompositionBuilder = exports.SourceToolBuilder = exports.ToolBuilder = exports.tool = void 0;
// Core types
__exportStar(require("./types.js"), exports);
// Registry v2 builders
__exportStar(require("./builder.js"), exports);
// Path builder
__exportStar(require("./path-builder.js"), exports);
// Tool builder
var tool_js_1 = require("./tool.js");
Object.defineProperty(exports, "tool", { enumerable: true, get: function () { return tool_js_1.tool; } });
Object.defineProperty(exports, "ToolBuilder", { enumerable: true, get: function () { return tool_js_1.ToolBuilder; } });
Object.defineProperty(exports, "SourceToolBuilder", { enumerable: true, get: function () { return tool_js_1.SourceToolBuilder; } });
Object.defineProperty(exports, "CompositionBuilder", { enumerable: true, get: function () { return tool_js_1.CompositionBuilder; } });
// Pattern builders
var index_js_1 = require("./patterns/index.js");
// Pipeline
Object.defineProperty(exports, "pipeline", { enumerable: true, get: function () { return index_js_1.pipeline; } });
Object.defineProperty(exports, "step", { enumerable: true, get: function () { return index_js_1.step; } });
Object.defineProperty(exports, "PipelineBuilder", { enumerable: true, get: function () { return index_js_1.PipelineBuilder; } });
Object.defineProperty(exports, "StepBuilder", { enumerable: true, get: function () { return index_js_1.StepBuilder; } });
// Scatter-gather
Object.defineProperty(exports, "scatterGather", { enumerable: true, get: function () { return index_js_1.scatterGather; } });
Object.defineProperty(exports, "agg", { enumerable: true, get: function () { return index_js_1.agg; } });
Object.defineProperty(exports, "ScatterGatherBuilder", { enumerable: true, get: function () { return index_js_1.ScatterGatherBuilder; } });
Object.defineProperty(exports, "AggregationBuilder", { enumerable: true, get: function () { return index_js_1.AggregationBuilder; } });
// Filter
Object.defineProperty(exports, "filter", { enumerable: true, get: function () { return index_js_1.filter; } });
Object.defineProperty(exports, "filterBy", { enumerable: true, get: function () { return index_js_1.filterBy; } });
Object.defineProperty(exports, "FilterBuilder", { enumerable: true, get: function () { return index_js_1.FilterBuilder; } });
// Schema map
Object.defineProperty(exports, "schemaMap", { enumerable: true, get: function () { return index_js_1.schemaMap; } });
Object.defineProperty(exports, "outputTransform", { enumerable: true, get: function () { return index_js_1.outputTransform; } });
Object.defineProperty(exports, "SchemaMapBuilder", { enumerable: true, get: function () { return index_js_1.SchemaMapBuilder; } });
// Map each
Object.defineProperty(exports, "mapEach", { enumerable: true, get: function () { return index_js_1.mapEach; } });
Object.defineProperty(exports, "mapEachTool", { enumerable: true, get: function () { return index_js_1.mapEachTool; } });
Object.defineProperty(exports, "mapEachPattern", { enumerable: true, get: function () { return index_js_1.mapEachPattern; } });
Object.defineProperty(exports, "MapEachBuilder", { enumerable: true, get: function () { return index_js_1.MapEachBuilder; } });
// Compiler
var compiler_js_1 = require("./compiler.js");
Object.defineProperty(exports, "createRegistry", { enumerable: true, get: function () { return compiler_js_1.createRegistry; } });
Object.defineProperty(exports, "compile", { enumerable: true, get: function () { return compiler_js_1.compile; } });
Object.defineProperty(exports, "parseRegistry", { enumerable: true, get: function () { return compiler_js_1.parseRegistry; } });
Object.defineProperty(exports, "RegistryBuilder", { enumerable: true, get: function () { return compiler_js_1.RegistryBuilder; } });
//# sourceMappingURL=index.js.map