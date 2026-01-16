/**
 * Tool definition builders
 */
import type { ToolDefinition, PatternSpec, OutputTransform, JSONSchema } from './types.js';
/**
 * Builder for creating tool definitions
 */
export declare class ToolBuilder {
    private def;
    constructor(name: string);
    /**
     * Set description
     */
    description(desc: string): this;
    /**
     * Set as source-based tool (1:1 mapping)
     */
    source(target: string, tool: string): SourceToolBuilder;
    /**
     * Set as composition (N:1 mapping)
     */
    composition(spec: PatternSpec): CompositionBuilder;
    /**
     * Set input schema
     */
    inputSchema(schema: JSONSchema): this;
    /**
     * Set output transform
     */
    outputTransform(transform: OutputTransform): this;
    /**
     * Set version
     */
    version(v: string): this;
    /**
     * Set metadata
     */
    metadata(meta: Record<string, unknown>): this;
    /**
     * Build the tool definition
     */
    build(): ToolDefinition;
}
/**
 * Builder for source-based tools
 */
export declare class SourceToolBuilder {
    private def;
    private source;
    constructor(def: ToolDefinition, target: string, tool: string);
    /**
     * Add default value
     */
    default(key: string, value: unknown): this;
    /**
     * Add multiple defaults
     */
    defaults(defaults: Record<string, unknown>): this;
    /**
     * Hide fields from schema
     */
    hideFields(fields: string[]): this;
    /**
     * Set description
     */
    description(desc: string): this;
    /**
     * Set output transform
     */
    outputTransform(transform: OutputTransform): this;
    /**
     * Set version
     */
    version(v: string): this;
    /**
     * Set metadata
     */
    metadata(meta: Record<string, unknown>): this;
    /**
     * Build the tool definition
     */
    build(): ToolDefinition;
}
/**
 * Builder for composition tools
 */
export declare class CompositionBuilder {
    private def;
    constructor(def: ToolDefinition, spec: PatternSpec);
    /**
     * Set description
     */
    description(desc: string): this;
    /**
     * Set input schema
     */
    inputSchema(schema: JSONSchema): this;
    /**
     * Set output transform
     */
    outputTransform(transform: OutputTransform): this;
    /**
     * Set version
     */
    version(v: string): this;
    /**
     * Set metadata
     */
    metadata(meta: Record<string, unknown>): this;
    /**
     * Build the tool definition
     */
    build(): ToolDefinition;
}
/**
 * Create a new tool builder
 *
 * @example
 * const weatherTool = tool('get_weather')
 *   .description('Get weather information')
 *   .source('weather', 'fetch_weather')
 *   .default('units', 'metric')
 *   .build();
 */
export declare function tool(name: string): ToolBuilder;
//# sourceMappingURL=tool.d.ts.map