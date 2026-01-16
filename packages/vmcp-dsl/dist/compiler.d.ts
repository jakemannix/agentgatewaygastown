/**
 * Compiler for transforming DSL definitions to JSON IR
 */
import type { Registry, ToolDefinition } from './types.js';
/**
 * Registry builder for accumulating tool definitions
 */
export declare class RegistryBuilder {
    private tools;
    private schemaVersion;
    /**
     * Set schema version
     */
    version(v: string): this;
    /**
     * Add a tool definition
     */
    add(tool: ToolDefinition): this;
    /**
     * Add multiple tool definitions
     */
    addAll(...tools: ToolDefinition[]): this;
    /**
     * Build the registry
     */
    build(): Registry;
    /**
     * Compile to JSON string
     */
    toJSON(pretty?: boolean): string;
    /**
     * Validate the registry
     */
    validate(): ValidationResult;
}
export interface ValidationResult {
    valid: boolean;
    errors: ValidationError[];
    warnings: ValidationWarning[];
}
export interface ValidationError {
    type: string;
    message: string;
    toolName?: string;
}
export interface ValidationWarning {
    type: string;
    message: string;
    toolName?: string;
}
/**
 * Create a registry builder
 *
 * @example
 * const registry = createRegistry()
 *   .add(tool('get_weather').source('weather', 'fetch').build())
 *   .add(tool('research').composition(pipeline()...build()).build())
 *   .build();
 */
export declare function createRegistry(): RegistryBuilder;
/**
 * Compile a registry to JSON
 *
 * @example
 * const json = compile(
 *   tool('search').source('backend', 'search').build(),
 *   tool('pipeline').composition(pipeline()...build()).build()
 * );
 */
export declare function compile(...tools: ToolDefinition[]): string;
/**
 * Parse a JSON registry
 */
export declare function parseRegistry(json: string): Registry;
//# sourceMappingURL=compiler.d.ts.map