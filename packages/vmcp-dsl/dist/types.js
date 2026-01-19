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
exports.isRetry = isRetry;
exports.isTimeout = isTimeout;
exports.isCache = isCache;
exports.isIdempotent = isIdempotent;
exports.isCircuitBreaker = isCircuitBreaker;
exports.isDeadLetter = isDeadLetter;
exports.isSaga = isSaga;
exports.isClaimCheck = isClaimCheck;
exports.isThrottle = isThrottle;
exports.isSchemaRef = isSchemaRef;
exports.hasSourceV2 = hasSourceV2;
exports.hasSpecV2 = hasSpecV2;
exports.isToolOperation = isToolOperation;
exports.isPatternOperation = isPatternOperation;
exports.isAgentOperation = isAgentOperation;
exports.isToolDependency = isToolDependency;
exports.isAgentDependency = isAgentDependency;
exports.parseSchemaRef = parseSchemaRef;
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
// Stateful pattern type guards
function isRetry(spec) {
    return 'retry' in spec;
}
function isTimeout(spec) {
    return 'timeout' in spec;
}
function isCache(spec) {
    return 'cache' in spec;
}
function isIdempotent(spec) {
    return 'idempotent' in spec;
}
function isCircuitBreaker(spec) {
    return 'circuitBreaker' in spec;
}
function isDeadLetter(spec) {
    return 'deadLetter' in spec;
}
function isSaga(spec) {
    return 'saga' in spec;
}
function isClaimCheck(spec) {
    return 'claimCheck' in spec;
}
function isThrottle(spec) {
    return 'throttle' in spec;
}
// =============================================================================
// Registry v2 Type Guards
// =============================================================================
/** Check if schema or inline schema is a reference */
function isSchemaRef(schema) {
    return '$ref' in schema && typeof schema.$ref === 'string';
}
/** Check if a tool definition v2 has a source (vs composition) */
function hasSourceV2(tool) {
    return tool.source !== undefined;
}
/** Check if a tool definition v2 has a spec (composition) */
function hasSpecV2(tool) {
    return tool.spec !== undefined;
}
/** Check if a step operation is a tool call */
function isToolOperation(op) {
    return 'tool' in op;
}
/** Check if a step operation is a pattern */
function isPatternOperation(op) {
    return 'pattern' in op;
}
/** Check if a step operation is an agent call */
function isAgentOperation(op) {
    return 'agent' in op;
}
/** Check if a dependency is a tool dependency */
function isToolDependency(dep) {
    return dep.type === 'tool';
}
/** Check if a dependency is an agent dependency */
function isAgentDependency(dep) {
    return dep.type === 'agent';
}
/** Parse a schema reference string into name and version */
function parseSchemaRef(ref) {
    // Format: "#SchemaName:Version"
    const match = ref.match(/^#([^:]+):(.+)$/);
    if (!match)
        return null;
    return { name: match[1], version: match[2] };
}
//# sourceMappingURL=types.js.map