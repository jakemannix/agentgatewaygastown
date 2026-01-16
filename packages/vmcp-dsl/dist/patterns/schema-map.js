"use strict";
/**
 * Schema map pattern builder
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SchemaMapBuilder = void 0;
exports.schemaMap = schemaMap;
exports.outputTransform = outputTransform;
const path_builder_js_1 = require("../path-builder.js");
/**
 * Builder for schema map patterns
 */
class SchemaMapBuilder {
    mappings = {};
    /**
     * Map a field from a JSONPath
     */
    field(name, pathExpr) {
        this.mappings[name] = { path: (0, path_builder_js_1.path)(pathExpr) };
        return this;
    }
    /**
     * Map a field to a string literal
     */
    literal(name, value) {
        if (typeof value === 'string') {
            this.mappings[name] = { literal: { stringValue: value } };
        }
        else if (typeof value === 'number') {
            this.mappings[name] = { literal: { numberValue: value } };
        }
        else if (typeof value === 'boolean') {
            this.mappings[name] = { literal: { boolValue: value } };
        }
        else {
            this.mappings[name] = { literal: { nullValue: true } };
        }
        return this;
    }
    /**
     * Map a field using coalesce (first non-null value)
     */
    coalesce(name, paths) {
        this.mappings[name] = { coalesce: { paths: paths.map(path_builder_js_1.path) } };
        return this;
    }
    /**
     * Map a field using a template
     */
    template(name, templateStr, vars) {
        const resolvedVars = {};
        for (const [key, value] of Object.entries(vars)) {
            resolvedVars[key] = (0, path_builder_js_1.path)(value);
        }
        this.mappings[name] = { template: { template: templateStr, vars: resolvedVars } };
        return this;
    }
    /**
     * Map a field by concatenating multiple paths
     */
    concat(name, paths, separator) {
        this.mappings[name] = { concat: { paths: paths.map(path_builder_js_1.path), separator } };
        return this;
    }
    /**
     * Map a field to a nested object
     */
    nested(name, nestedMap) {
        const spec = nestedMap instanceof SchemaMapBuilder ? nestedMap.spec() : nestedMap;
        this.mappings[name] = { nested: spec };
        return this;
    }
    /**
     * Add a raw field source
     */
    add(name, source) {
        this.mappings[name] = source;
        return this;
    }
    /**
     * Build the schema map pattern spec
     */
    build() {
        return { schemaMap: { mappings: this.mappings } };
    }
    /**
     * Get the raw spec
     */
    spec() {
        return { mappings: this.mappings };
    }
    /**
     * Convert to output transform
     */
    toOutputTransform() {
        return { mappings: this.mappings };
    }
}
exports.SchemaMapBuilder = SchemaMapBuilder;
/**
 * Create a schema map pattern
 *
 * @example
 * const normalize = schemaMap()
 *   .field('title', '$.paper_title')
 *   .field('author', '$.author_name')
 *   .coalesce('url', ['$.pdf_url', '$.web_url'])
 *   .literal('source', 'arxiv')
 *   .template('citation', '{author} ({year})', { author: 'author', year: 'year' })
 *   .build();
 */
function schemaMap() {
    return new SchemaMapBuilder();
}
/**
 * Create an output transform
 *
 * @example
 * const transform = outputTransform()
 *   .field('temperature', '$.data.temp')
 *   .field('city', '$.location.name')
 *   .toOutputTransform();
 */
function outputTransform() {
    return new SchemaMapBuilder();
}
//# sourceMappingURL=schema-map.js.map