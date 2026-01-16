"use strict";
/**
 * Tool definition builders
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.CompositionBuilder = exports.SourceToolBuilder = exports.ToolBuilder = void 0;
exports.tool = tool;
/**
 * Builder for creating tool definitions
 */
class ToolBuilder {
    def = {};
    constructor(name) {
        this.def.name = name;
    }
    /**
     * Set description
     */
    description(desc) {
        this.def.description = desc;
        return this;
    }
    /**
     * Set as source-based tool (1:1 mapping)
     */
    source(target, tool) {
        return new SourceToolBuilder(this.def, target, tool);
    }
    /**
     * Set as composition (N:1 mapping)
     */
    composition(spec) {
        return new CompositionBuilder(this.def, spec);
    }
    /**
     * Set input schema
     */
    inputSchema(schema) {
        this.def.inputSchema = schema;
        return this;
    }
    /**
     * Set output transform
     */
    outputTransform(transform) {
        this.def.outputTransform = transform;
        return this;
    }
    /**
     * Set version
     */
    version(v) {
        this.def.version = v;
        return this;
    }
    /**
     * Set metadata
     */
    metadata(meta) {
        this.def.metadata = meta;
        return this;
    }
    /**
     * Build the tool definition
     */
    build() {
        if (!this.def.name) {
            throw new Error('Tool name is required');
        }
        if (!('implementation' in this.def) || !this.def.implementation) {
            throw new Error('Tool implementation (source or composition) is required');
        }
        return this.def;
    }
}
exports.ToolBuilder = ToolBuilder;
/**
 * Builder for source-based tools
 */
class SourceToolBuilder {
    def;
    source;
    constructor(def, target, tool) {
        this.def = def;
        this.source = { target, tool };
        this.def.implementation = { source: this.source };
    }
    /**
     * Add default value
     */
    default(key, value) {
        if (!this.source.defaults) {
            this.source.defaults = {};
        }
        this.source.defaults[key] = value;
        return this;
    }
    /**
     * Add multiple defaults
     */
    defaults(defaults) {
        this.source.defaults = { ...this.source.defaults, ...defaults };
        return this;
    }
    /**
     * Hide fields from schema
     */
    hideFields(fields) {
        this.source.hideFields = fields;
        return this;
    }
    /**
     * Set description
     */
    description(desc) {
        this.def.description = desc;
        return this;
    }
    /**
     * Set output transform
     */
    outputTransform(transform) {
        this.def.outputTransform = transform;
        return this;
    }
    /**
     * Set version
     */
    version(v) {
        this.def.version = v;
        return this;
    }
    /**
     * Set metadata
     */
    metadata(meta) {
        this.def.metadata = meta;
        return this;
    }
    /**
     * Build the tool definition
     */
    build() {
        return this.def;
    }
}
exports.SourceToolBuilder = SourceToolBuilder;
/**
 * Builder for composition tools
 */
class CompositionBuilder {
    def;
    constructor(def, spec) {
        this.def = def;
        this.def.implementation = { spec };
    }
    /**
     * Set description
     */
    description(desc) {
        this.def.description = desc;
        return this;
    }
    /**
     * Set input schema
     */
    inputSchema(schema) {
        this.def.inputSchema = schema;
        return this;
    }
    /**
     * Set output transform
     */
    outputTransform(transform) {
        this.def.outputTransform = transform;
        return this;
    }
    /**
     * Set version
     */
    version(v) {
        this.def.version = v;
        return this;
    }
    /**
     * Set metadata
     */
    metadata(meta) {
        this.def.metadata = meta;
        return this;
    }
    /**
     * Build the tool definition
     */
    build() {
        return this.def;
    }
}
exports.CompositionBuilder = CompositionBuilder;
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
function tool(name) {
    return new ToolBuilder(name);
}
//# sourceMappingURL=tool.js.map