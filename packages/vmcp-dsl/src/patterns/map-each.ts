/**
 * Map-each pattern builder
 */

import type {
  PatternSpec,
  MapEachSpec,
} from '../types.js';

/**
 * Builder for map-each patterns
 */
export class MapEachBuilder {
  private inner?: { tool: string } | { pattern: PatternSpec };

  /**
   * Apply a tool to each element
   */
  tool(name: string): this {
    this.inner = { tool: name };
    return this;
  }

  /**
   * Apply a pattern to each element
   */
  pattern(spec: PatternSpec): this {
    this.inner = { pattern: spec };
    return this;
  }

  /**
   * Build the map-each pattern spec
   */
  build(): PatternSpec {
    if (!this.inner) {
      throw new Error('Inner operation (tool or pattern) is required');
    }
    return { mapEach: { inner: this.inner } };
  }

  /**
   * Get the raw spec
   */
  spec(): MapEachSpec {
    if (!this.inner) {
      throw new Error('Inner operation (tool or pattern) is required');
    }
    return { inner: this.inner };
  }
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
export function mapEach(): MapEachBuilder {
  return new MapEachBuilder();
}

/**
 * Shorthand for creating a map-each with a tool
 */
export function mapEachTool(toolName: string): PatternSpec {
  return { mapEach: { inner: { tool: toolName } } };
}

/**
 * Shorthand for creating a map-each with a pattern
 */
export function mapEachPattern(spec: PatternSpec): PatternSpec {
  return { mapEach: { inner: { pattern: spec } } };
}

