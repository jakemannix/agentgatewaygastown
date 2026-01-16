/**
 * Schema map pattern builder
 */
import type { PatternSpec, SchemaMapSpec, FieldSource, OutputTransform } from '../types.js';
/**
 * Builder for schema map patterns
 */
export declare class SchemaMapBuilder {
    private mappings;
    /**
     * Map a field from a JSONPath
     */
    field(name: string, pathExpr: string): this;
    /**
     * Map a field to a string literal
     */
    literal(name: string, value: string | number | boolean | null): this;
    /**
     * Map a field using coalesce (first non-null value)
     */
    coalesce(name: string, paths: string[]): this;
    /**
     * Map a field using a template
     */
    template(name: string, templateStr: string, vars: Record<string, string>): this;
    /**
     * Map a field by concatenating multiple paths
     */
    concat(name: string, paths: string[], separator?: string): this;
    /**
     * Map a field to a nested object
     */
    nested(name: string, nestedMap: SchemaMapBuilder | SchemaMapSpec): this;
    /**
     * Add a raw field source
     */
    add(name: string, source: FieldSource): this;
    /**
     * Build the schema map pattern spec
     */
    build(): PatternSpec;
    /**
     * Get the raw spec
     */
    spec(): SchemaMapSpec;
    /**
     * Convert to output transform
     */
    toOutputTransform(): OutputTransform;
}
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
export declare function schemaMap(): SchemaMapBuilder;
/**
 * Create an output transform
 *
 * @example
 * const transform = outputTransform()
 *   .field('temperature', '$.data.temp')
 *   .field('city', '$.location.name')
 *   .toOutputTransform();
 */
export declare function outputTransform(): SchemaMapBuilder;
//# sourceMappingURL=schema-map.d.ts.map