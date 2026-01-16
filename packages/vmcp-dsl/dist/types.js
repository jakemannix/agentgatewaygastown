"use strict";
/**
 * Core type definitions for vMCP tool compositions
 * These types correspond to the registry.proto schema
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.isSourceTool = isSourceTool;
exports.isComposition = isComposition;
exports.isPipeline = isPipeline;
exports.isScatterGather = isScatterGather;
exports.isFilter = isFilter;
exports.isSchemaMap = isSchemaMap;
exports.isMapEach = isMapEach;
// =============================================================================
// Type Guards
// =============================================================================
function isSourceTool(impl) {
    return 'source' in impl;
}
function isComposition(impl) {
    return 'spec' in impl;
}
function isPipeline(spec) {
    return 'pipeline' in spec;
}
function isScatterGather(spec) {
    return 'scatterGather' in spec;
}
function isFilter(spec) {
    return 'filter' in spec;
}
function isSchemaMap(spec) {
    return 'schemaMap' in spec;
}
function isMapEach(spec) {
    return 'mapEach' in spec;
}
//# sourceMappingURL=types.js.map