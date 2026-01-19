/**
 * Fluent builder API for Registry v2
 *
 * Provides type-safe builders for constructing registry schemas, servers,
 * tools, and agents with a chainable API.
 *
 * @example
 * ```ts
 * const registry = new RegistryV2Builder()
 *   .schema(schema('SearchQuery', '1.0.0')
 *     .description('Standard search input')
 *     .schema({ type: 'object', properties: { query: { type: 'string' } } })
 *     .build())
 *   .server(server('doc-service', '1.2.0')
 *     .description('Document service')
 *     .provides('search_documents', '1.0.0')
 *     .build())
 *   .tool(toolV2('search_documents', '1.0.0')
 *     .description('Semantic search')
 *     .source('doc-service', '1.2.0', 'search_documents')
 *     .inputSchemaRef('#SearchQuery:1.0.0')
 *     .build())
 *   .build();
 * ```
 */

import type {
  RegistryV2,
  SchemaDefinition,
  ServerDefinition,
  ToolDefinitionV2,
  ToolSourceV2,
  ToolProvision,
  AgentDefinition,
  AgentSkill,
  AgentCapabilities,
  AgentExtension,
  AgentProvider,
  Dependency,
  JSONSchema,
  SchemaRef,
  PatternSpec,
  OutputTransform,
} from './types.js';

// =============================================================================
// Schema Builder
// =============================================================================

/**
 * Builder for schema definitions
 */
export class SchemaBuilder {
  private def: SchemaDefinition;

  constructor(name: string, version: string) {
    this.def = {
      name,
      version,
      schema: {},
    };
  }

  description(desc: string): this {
    this.def.description = desc;
    return this;
  }

  schema(schema: JSONSchema): this {
    this.def.schema = schema;
    return this;
  }

  metadata(key: string, value: unknown): this {
    if (!this.def.metadata) this.def.metadata = {};
    this.def.metadata[key] = value;
    return this;
  }

  build(): SchemaDefinition {
    return { ...this.def };
  }
}

/**
 * Create a schema builder
 */
export function schema(name: string, version: string): SchemaBuilder {
  return new SchemaBuilder(name, version);
}

// =============================================================================
// Server Builder
// =============================================================================

/**
 * Builder for server definitions
 */
export class ServerBuilder {
  private def: ServerDefinition;

  constructor(name: string, version: string) {
    this.def = {
      name,
      version,
      provides: [],
    };
  }

  description(desc: string): this {
    this.def.description = desc;
    return this;
  }

  provides(tool: string, version: string): this {
    this.def.provides.push({ tool, version });
    return this;
  }

  providesAll(provisions: ToolProvision[]): this {
    this.def.provides.push(...provisions);
    return this;
  }

  deprecated(message?: string): this {
    this.def.deprecated = true;
    if (message) this.def.deprecationMessage = message;
    return this;
  }

  metadata(key: string, value: unknown): this {
    if (!this.def.metadata) this.def.metadata = {};
    this.def.metadata[key] = value;
    return this;
  }

  build(): ServerDefinition {
    return { ...this.def };
  }
}

/**
 * Create a server builder
 */
export function server(name: string, version: string): ServerBuilder {
  return new ServerBuilder(name, version);
}

// =============================================================================
// Tool v2 Builder
// =============================================================================

/**
 * Builder for tool definitions v2
 */
export class ToolV2Builder {
  private def: ToolDefinitionV2;

  constructor(name: string, version: string) {
    this.def = {
      name,
      version,
    };
  }

  description(desc: string): this {
    this.def.description = desc;
    return this;
  }

  /**
   * Set the source for a 1:1 tool mapping
   */
  source(server: string, serverVersion: string, tool: string): this {
    this.def.source = {
      server,
      serverVersion,
      tool,
    };
    return this;
  }

  /**
   * Set full source configuration
   */
  sourceConfig(config: ToolSourceV2): this {
    this.def.source = config;
    return this;
  }

  /**
   * Set the pattern specification for a composition
   */
  spec(pattern: PatternSpec): this {
    this.def.spec = pattern;
    return this;
  }

  /**
   * Add a tool dependency
   */
  dependsOnTool(name: string, version: string): this {
    if (!this.def.depends) this.def.depends = [];
    this.def.depends.push({ type: 'tool', name, version });
    return this;
  }

  /**
   * Add an agent dependency
   */
  dependsOnAgent(name: string, version: string, skill?: string): this {
    if (!this.def.depends) this.def.depends = [];
    this.def.depends.push({ type: 'agent', name, version, skill });
    return this;
  }

  /**
   * Add multiple dependencies
   */
  dependsOn(deps: Dependency[]): this {
    if (!this.def.depends) this.def.depends = [];
    this.def.depends.push(...deps);
    return this;
  }

  /**
   * Set inline input schema
   */
  inputSchema(schema: JSONSchema): this {
    this.def.inputSchema = schema;
    return this;
  }

  /**
   * Set input schema reference
   */
  inputSchemaRef(ref: string): this {
    this.def.inputSchema = { $ref: ref };
    return this;
  }

  /**
   * Set inline output schema
   */
  outputSchema(schema: JSONSchema): this {
    this.def.outputSchema = schema;
    return this;
  }

  /**
   * Set output schema reference
   */
  outputSchemaRef(ref: string): this {
    this.def.outputSchema = { $ref: ref };
    return this;
  }

  /**
   * Set output transform
   */
  outputTransform(transform: OutputTransform): this {
    this.def.outputTransform = transform;
    return this;
  }

  metadata(key: string, value: unknown): this {
    if (!this.def.metadata) this.def.metadata = {};
    this.def.metadata[key] = value;
    return this;
  }

  build(): ToolDefinitionV2 {
    return { ...this.def };
  }
}

/**
 * Create a tool v2 builder
 */
export function toolV2(name: string, version: string): ToolV2Builder {
  return new ToolV2Builder(name, version);
}

// =============================================================================
// Agent Skill Builder
// =============================================================================

/**
 * Builder for agent skill definitions
 */
export class AgentSkillBuilder {
  private def: AgentSkill;

  constructor(id: string, name: string) {
    this.def = {
      id,
      name,
      description: '',
      tags: [],
      inputModes: [],
      outputModes: [],
    };
  }

  description(desc: string): this {
    this.def.description = desc;
    return this;
  }

  tags(...tags: string[]): this {
    this.def.tags.push(...tags);
    return this;
  }

  examples(...examples: string[]): this {
    if (!this.def.examples) this.def.examples = [];
    this.def.examples.push(...examples);
    return this;
  }

  inputModes(...modes: string[]): this {
    this.def.inputModes.push(...modes);
    return this;
  }

  outputModes(...modes: string[]): this {
    this.def.outputModes.push(...modes);
    return this;
  }

  inputSchema(schema: JSONSchema): this {
    this.def.inputSchema = schema;
    return this;
  }

  inputSchemaRef(ref: string): this {
    this.def.inputSchema = { $ref: ref };
    return this;
  }

  outputSchema(schema: JSONSchema): this {
    this.def.outputSchema = schema;
    return this;
  }

  outputSchemaRef(ref: string): this {
    this.def.outputSchema = { $ref: ref };
    return this;
  }

  build(): AgentSkill {
    return { ...this.def };
  }
}

/**
 * Create an agent skill builder
 */
export function skill(id: string, name: string): AgentSkillBuilder {
  return new AgentSkillBuilder(id, name);
}

// =============================================================================
// Agent Builder
// =============================================================================

/**
 * Builder for agent definitions
 */
export class AgentBuilder {
  private def: AgentDefinition;

  constructor(name: string, version: string) {
    this.def = {
      name,
      version,
      description: '',
      url: '',
      protocolVersion: '0.2.1',
      defaultInputModes: [],
      defaultOutputModes: [],
      skills: [],
      capabilities: {},
    };
  }

  description(desc: string): this {
    this.def.description = desc;
    return this;
  }

  url(url: string): this {
    this.def.url = url;
    return this;
  }

  protocolVersion(version: string): this {
    this.def.protocolVersion = version;
    return this;
  }

  defaultInputModes(...modes: string[]): this {
    this.def.defaultInputModes.push(...modes);
    return this;
  }

  defaultOutputModes(...modes: string[]): this {
    this.def.defaultOutputModes.push(...modes);
    return this;
  }

  skill(skill: AgentSkill): this {
    this.def.skills.push(skill);
    return this;
  }

  skills(...skills: AgentSkill[]): this {
    this.def.skills.push(...skills);
    return this;
  }

  streaming(enabled: boolean = true): this {
    this.def.capabilities.streaming = enabled;
    return this;
  }

  pushNotifications(enabled: boolean = true): this {
    this.def.capabilities.pushNotifications = enabled;
    return this;
  }

  stateTransitionHistory(enabled: boolean = true): this {
    this.def.capabilities.stateTransitionHistory = enabled;
    return this;
  }

  extension(ext: AgentExtension): this {
    if (!this.def.capabilities.extensions) {
      this.def.capabilities.extensions = [];
    }
    this.def.capabilities.extensions.push(ext);
    return this;
  }

  /**
   * Add SBOM extension with dependencies
   */
  sbom(depends: Dependency[]): this {
    return this.extension({
      uri: 'urn:agentgateway:sbom',
      description: 'Software bill of materials - tool and agent dependencies',
      required: true,
      params: { depends },
    });
  }

  provider(organization: string, url?: string): this {
    this.def.provider = { organization, url };
    return this;
  }

  capabilities(caps: AgentCapabilities): this {
    this.def.capabilities = { ...this.def.capabilities, ...caps };
    return this;
  }

  build(): AgentDefinition {
    return { ...this.def };
  }
}

/**
 * Create an agent builder
 */
export function agent(name: string, version: string): AgentBuilder {
  return new AgentBuilder(name, version);
}

// =============================================================================
// Registry v2 Builder
// =============================================================================

/**
 * Builder for Registry v2
 */
export class RegistryV2Builder {
  private registry: RegistryV2;

  constructor() {
    this.registry = {
      schemaVersion: '2.0',
      schemas: [],
      servers: [],
      tools: [],
      agents: [],
    };
  }

  schema(def: SchemaDefinition): this {
    this.registry.schemas.push(def);
    return this;
  }

  schemas(...defs: SchemaDefinition[]): this {
    this.registry.schemas.push(...defs);
    return this;
  }

  server(def: ServerDefinition): this {
    this.registry.servers.push(def);
    return this;
  }

  servers(...defs: ServerDefinition[]): this {
    this.registry.servers.push(...defs);
    return this;
  }

  tool(def: ToolDefinitionV2): this {
    this.registry.tools.push(def);
    return this;
  }

  tools(...defs: ToolDefinitionV2[]): this {
    this.registry.tools.push(...defs);
    return this;
  }

  agent(def: AgentDefinition): this {
    this.registry.agents.push(def);
    return this;
  }

  agents(...defs: AgentDefinition[]): this {
    this.registry.agents.push(...defs);
    return this;
  }

  build(): RegistryV2 {
    return { ...this.registry };
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
}

/**
 * Create a Registry v2 builder
 */
export function registryV2(): RegistryV2Builder {
  return new RegistryV2Builder();
}

// =============================================================================
// Dependency Helpers
// =============================================================================

/**
 * Create a tool dependency
 */
export function toolDep(name: string, version: string): Dependency {
  return { type: 'tool', name, version };
}

/**
 * Create an agent dependency
 */
export function agentDep(name: string, version: string, skill?: string): Dependency {
  return { type: 'agent', name, version, skill };
}

/**
 * Create a schema reference string
 */
export function schemaRef(name: string, version: string): string {
  return `#${name}:${version}`;
}
