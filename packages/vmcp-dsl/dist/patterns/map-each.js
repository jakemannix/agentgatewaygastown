"use strict";
/**
 * Map-each pattern builder
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.MapEachBuilder = void 0;
exports.mapEach = mapEach;
exports.mapEachTool = mapEachTool;
exports.mapEachPattern = mapEachPattern;
/**
 * Builder for map-each patterns
 */
class MapEachBuilder {
    inner;
    /**
     * Apply a tool to each element
     */
    tool(name) {
        this.inner = { tool: name };
        return this;
    }
    /**
     * Apply a pattern to each element
     */
    pattern(spec) {
        this.inner = { pattern: spec };
        return this;
    }
    /**
     * Build the map-each pattern spec
     */
    build() {
        if (!this.inner) {
            throw new Error('Inner operation (tool or pattern) is required');
        }
        return { mapEach: { inner: this.inner } };
    }
    /**
     * Get the raw spec
     */
    spec() {
        if (!this.inner) {
            throw new Error('Inner operation (tool or pattern) is required');
        }
        return { inner: this.inner };
    }
}
exports.MapEachBuilder = MapEachBuilder;
/**
 * Create a map-each pattern
 *
 * @example
 * // Apply a tool to each element
 * const fetchAll = mapEach()
 *   .tool('fetch_document')
 *   .build();
 *
 * // Apply a pattern to each element
 * const normalizeAll = mapEach()
 *   .pattern(schemaMap().field('title', '$.name').build())
 *   .build();
 */
function mapEach() {
    return new MapEachBuilder();
}
/**
 * Shorthand for creating a map-each with a tool
 */
function mapEachTool(toolName) {
    return { mapEach: { inner: { tool: toolName } } };
}
/**
 * Shorthand for creating a map-each with a pattern
 */
function mapEachPattern(spec) {
    return { mapEach: { inner: { pattern: spec } } };
}
//# sourceMappingURL=map-each.js.map