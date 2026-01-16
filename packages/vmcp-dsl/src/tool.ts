/**
 * Tool definition builders
 */

import type {
  ToolDefinition,
  SourceTool,
  PatternSpec,
  OutputTransform,
  JSONSchema,
} from './types.js';

/**
 * Builder for creating tool definitions
 */
export class ToolBuilder {
  private def: Partial<ToolDefinition> = {};

  constructor(name: string) {
    this.def.name = name;
  }

  /**
   * Set description
   */
  description(desc: string): this {
    this.def.description = desc;
    return this;
  }

  /**
   * Set as source-based tool (1:1 mapping)
   */
  source(target: string, tool: string): SourceToolBuilder {
    return new SourceToolBuilder(this.def as ToolDefinition, target, tool);
  }

  /**
   * Set as composition (N:1 mapping)
   */
  composition(spec: PatternSpec): CompositionBuilder {
    return new CompositionBuilder(this.def as ToolDefinition, spec);
  }

  /**
   * Set input schema
   */
  inputSchema(schema: JSONSchema): this {
    this.def.inputSchema = schema;
    return this;
  }

  /**
   * Set output transform
   */
  outputTransform(transform: OutputTransform): this {
    this.def.outputTransform = transform;
    return this;
  }

  /**
   * Set version
   */
  version(v: string): this {
    this.def.version = v;
    return this;
  }

  /**
   * Set metadata
   */
  metadata(meta: Record<string, unknown>): this {
    this.def.metadata = meta;
    return this;
  }

  /**
   * Build the tool definition
   */
  build(): ToolDefinition {
    if (!this.def.name) {
      throw new Error('Tool name is required');
    }
    if (!('implementation' in this.def) || !this.def.implementation) {
      throw new Error('Tool implementation (source or composition) is required');
    }
    return this.def as ToolDefinition;
  }
}

/**
 * Builder for source-based tools
 */
export class SourceToolBuilder {
  private source: SourceTool;

  constructor(
    private def: ToolDefinition,
    target: string,
    tool: string
  ) {
    this.source = { target, tool };
    this.def.implementation = { source: this.source };
  }

  /**
   * Add default value
   */
  default(key: string, value: unknown): this {
    if (!this.source.defaults) {
      this.source.defaults = {};
    }
    this.source.defaults[key] = value;
    return this;
  }

  /**
   * Add multiple defaults
   */
  defaults(defaults: Record<string, unknown>): this {
    this.source.defaults = { ...this.source.defaults, ...defaults };
    return this;
  }

  /**
   * Hide fields from schema
   */
  hideFields(fields: string[]): this {
    this.source.hideFields = fields;
    return this;
  }

  /**
   * Set description
   */
  description(desc: string): this {
    this.def.description = desc;
    return this;
  }

  /**
   * Set output transform
   */
  outputTransform(transform: OutputTransform): this {
    this.def.outputTransform = transform;
    return this;
  }

  /**
   * Set version
   */
  version(v: string): this {
    this.def.version = v;
    return this;
  }

  /**
   * Set metadata
   */
  metadata(meta: Record<string, unknown>): this {
    this.def.metadata = meta;
    return this;
  }

  /**
   * Build the tool definition
   */
  build(): ToolDefinition {
    return this.def;
  }
}

/**
 * Builder for composition tools
 */
export class CompositionBuilder {
  constructor(
    private def: ToolDefinition,
    spec: PatternSpec
  ) {
    this.def.implementation = { spec };
  }

  /**
   * Set description
   */
  description(desc: string): this {
    this.def.description = desc;
    return this;
  }

  /**
   * Set input schema
   */
  inputSchema(schema: JSONSchema): this {
    this.def.inputSchema = schema;
    return this;
  }

  /**
   * Set output transform
   */
  outputTransform(transform: OutputTransform): this {
    this.def.outputTransform = transform;
    return this;
  }

  /**
   * Set version
   */
  version(v: string): this {
    this.def.version = v;
    return this;
  }

  /**
   * Set metadata
   */
  metadata(meta: Record<string, unknown>): this {
    this.def.metadata = meta;
    return this;
  }

  /**
   * Build the tool definition
   */
  build(): ToolDefinition {
    return this.def;
  }
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
export function tool(name: string): ToolBuilder {
  return new ToolBuilder(name);
}

