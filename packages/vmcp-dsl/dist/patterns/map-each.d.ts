/**
 * Map-each pattern builder
 */
import type { PatternSpec, MapEachSpec } from '../types.js';
/**
 * Builder for map-each patterns
 */
export declare class MapEachBuilder {
    private inner?;
    /**
     * Apply a tool to each element
     */
    tool(name: string): this;
    /**
     * Apply a pattern to each element
     */
    pattern(spec: PatternSpec): this;
    /**
     * Build the map-each pattern spec
     */
    build(): PatternSpec;
    /**
     * Get the raw spec
     */
    spec(): MapEachSpec;
}
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
export declare function mapEach(): MapEachBuilder;
/**
 * Shorthand for creating a map-each with a tool
 */
export declare function mapEachTool(toolName: string): PatternSpec;
/**
 * Shorthand for creating a map-each with a pattern
 */
export declare function mapEachPattern(spec: PatternSpec): PatternSpec;
//# sourceMappingURL=map-each.d.ts.map