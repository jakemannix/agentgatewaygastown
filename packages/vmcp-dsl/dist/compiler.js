"use strict";
/**
 * Compiler for transforming DSL definitions to JSON IR
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.RegistryBuilder = void 0;
exports.createRegistry = createRegistry;
exports.compile = compile;
exports.parseRegistry = parseRegistry;
/**
 * Registry builder for accumulating tool definitions
 */
class RegistryBuilder {
    tools = [];
    schemaVersion = '1.0';
    /**
     * Set schema version
     */
    version(v) {
        this.schemaVersion = v;
        return this;
    }
    /**
     * Add a tool definition
     */
    add(tool) {
        this.tools.push(tool);
        return this;
    }
    /**
     * Add multiple tool definitions
     */
    addAll(...tools) {
        this.tools.push(...tools);
        return this;
    }
    /**
     * Build the registry
     */
    build() {
        return {
            schemaVersion: this.schemaVersion,
            tools: this.tools,
        };
    }
    /**
     * Compile to JSON string
     */
    toJSON(pretty = true) {
        const registry = this.build();
        return pretty
            ? JSON.stringify(registry, null, 2)
            : JSON.stringify(registry);
    }
    /**
     * Validate the registry
     */
    validate() {
        const errors = [];
        const warnings = [];
        const toolNames = new Set();
        for (const tool of this.tools) {
            // Check for duplicate names
            if (toolNames.has(tool.name)) {
                errors.push({
                    type: 'duplicate_name',
                    message: `Duplicate tool name: ${tool.name}`,
                    toolName: tool.name,
                });
            }
            toolNames.add(tool.name);
            // Validate tool definition
            if (!tool.implementation) {
                errors.push({
                    type: 'missing_implementation',
                    message: `Tool ${tool.name} has no implementation`,
                    toolName: tool.name,
                });
            }
            // Check for missing description
            if (!tool.description) {
                warnings.push({
                    type: 'missing_description',
                    message: `Tool ${tool.name} has no description`,
                    toolName: tool.name,
                });
            }
        }
        return {
            valid: errors.length === 0,
            errors,
            warnings,
        };
    }
}
exports.RegistryBuilder = RegistryBuilder;
/**
 * Create a registry builder
 *
 * @example
 * const registry = createRegistry()
 *   .add(tool('get_weather').source('weather', 'fetch').build())
 *   .add(tool('research').composition(pipeline()...build()).build())
 *   .build();
 */
function createRegistry() {
    return new RegistryBuilder();
}
/**
 * Compile a registry to JSON
 *
 * @example
 * const json = compile(
 *   tool('search').source('backend', 'search').build(),
 *   tool('pipeline').composition(pipeline()...build()).build()
 * );
 */
function compile(...tools) {
    return createRegistry().addAll(...tools).toJSON();
}
/**
 * Parse a JSON registry
 */
function parseRegistry(json) {
    return JSON.parse(json);
}
//# sourceMappingURL=compiler.js.map