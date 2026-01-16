/**
 * Schema map pattern builder
 */

import type {
  PatternSpec,
  SchemaMapSpec,
  FieldSource,
  OutputTransform,
} from '../types.js';
import { path } from '../path-builder.js';

/**
 * Builder for schema map patterns
 */
export class SchemaMapBuilder {
  private mappings: Record<string, FieldSource> = {};

  /**
   * Map a field from a JSONPath
   */
  field(name: string, pathExpr: string): this {
    this.mappings[name] = { path: path(pathExpr) };
    return this;
  }

  /**
   * Map a field to a string literal
   */
  literal(name: string, value: string | number | boolean | null): this {
    if (typeof value === 'string') {
      this.mappings[name] = { literal: { stringValue: value } };
    } else if (typeof value === 'number') {
      this.mappings[name] = { literal: { numberValue: value } };
    } else if (typeof value === 'boolean') {
      this.mappings[name] = { literal: { boolValue: value } };
    } else {
      this.mappings[name] = { literal: { nullValue: true } };
    }
    return this;
  }

  /**
   * Map a field using coalesce (first non-null value)
   */
  coalesce(name: string, paths: string[]): this {
    this.mappings[name] = { coalesce: { paths: paths.map(path) } };
    return this;
  }

  /**
   * Map a field using a template
   */
  template(name: string, templateStr: string, vars: Record<string, string>): this {
    const resolvedVars: Record<string, string> = {};
    for (const [key, value] of Object.entries(vars)) {
      resolvedVars[key] = path(value);
    }
    this.mappings[name] = { template: { template: templateStr, vars: resolvedVars } };
    return this;
  }

  /**
   * Map a field by concatenating multiple paths
   */
  concat(name: string, paths: string[], separator?: string): this {
    this.mappings[name] = { concat: { paths: paths.map(path), separator } };
    return this;
  }

  /**
   * Map a field to a nested object
   */
  nested(name: string, nestedMap: SchemaMapBuilder | SchemaMapSpec): this {
    const spec = nestedMap instanceof SchemaMapBuilder ? nestedMap.spec() : nestedMap;
    this.mappings[name] = { nested: spec };
    return this;
  }

  /**
   * Add a raw field source
   */
  add(name: string, source: FieldSource): this {
    this.mappings[name] = source;
    return this;
  }

  /**
   * Build the schema map pattern spec
   */
  build(): PatternSpec {
    return { schemaMap: { mappings: this.mappings } };
  }

  /**
   * Get the raw spec
   */
  spec(): SchemaMapSpec {
    return { mappings: this.mappings };
  }

  /**
   * Convert to output transform
   */
  toOutputTransform(): OutputTransform {
    return { mappings: this.mappings };
  }
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
export function schemaMap(): SchemaMapBuilder {
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
export function outputTransform(): SchemaMapBuilder {
  return new SchemaMapBuilder();
}

