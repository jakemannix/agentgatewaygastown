/**
 * Compiler for transforming DSL definitions to JSON IR
 *
 * This module provides functions for building and serializing registry definitions.
 * For canonical JSON output, use the proto-generated types from './generated/registry.js'.
 */

import type {
  Registry,
  ToolDefinition,
} from './types.js';

// Import generated types for canonical serialization
import {
  Registry as ProtoRegistry,
} from './generated/registry.js';

/**
 * Registry builder for accumulating tool definitions
 */
export class RegistryBuilder {
  private tools: ToolDefinition[] = [];
  private schemaVersion: string = '1.0';

  /**
   * Set schema version
   */
  version(v: string): this {
    this.schemaVersion = v;
    return this;
  }

  /**
   * Add a tool definition
   */
  add(tool: ToolDefinition): this {
    this.tools.push(tool);
    return this;
  }

  /**
   * Add multiple tool definitions
   */
  addAll(...tools: ToolDefinition[]): this {
    this.tools.push(...tools);
    return this;
  }

  /**
   * Build the registry
   */
  build(): Registry {
    return {
      schemaVersion: this.schemaVersion,
      tools: this.tools,
    };
  }

  /**
   * Compile to JSON string
   */
  toJSON(pretty: boolean = true): string {
    const registry = this.build();
    return pretty
      ? JSON.stringify(registry, null, 2)
      : JSON.stringify(registry);
  }

  /**
   * Validate the registry
   */
  validate(): ValidationResult {
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];
    const toolNames = new Set<string>();

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
export function createRegistry(): RegistryBuilder {
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
export function compile(...tools: ToolDefinition[]): string {
  return createRegistry().addAll(...tools).toJSON();
}

/**
 * Parse a JSON registry (v1 format)
 */
export function parseRegistry(json: string): Registry {
  return JSON.parse(json) as Registry;
}

// =============================================================================
// Proto-based Serialization (Canonical)
// =============================================================================

/**
 * Parse a JSON string into a proto Registry using the generated types.
 * This uses the canonical proto3 JSON format.
 *
 * @example
 * const registry = parseProtoRegistry(jsonString);
 * console.log(registry.schemaVersion);
 */
export function parseProtoRegistry(json: string): ProtoRegistry {
  const parsed = JSON.parse(json);
  return ProtoRegistry.fromJSON(parsed);
}

/**
 * Serialize a proto Registry to canonical JSON string.
 *
 * @example
 * const json = serializeProtoRegistry(registry);
 */
export function serializeProtoRegistry(registry: ProtoRegistry, pretty: boolean = true): string {
  const obj = ProtoRegistry.toJSON(registry);
  return pretty ? JSON.stringify(obj, null, 2) : JSON.stringify(obj);
}

/**
 * Validate and re-serialize a JSON registry through proto types.
 * This ensures the output is in canonical proto3 JSON format.
 *
 * @example
 * // Normalize a registry to canonical format
 * const canonical = canonicalizeRegistry(existingJson);
 */
export function canonicalizeRegistry(json: string): string {
  const registry = parseProtoRegistry(json);
  return serializeProtoRegistry(registry);
}

