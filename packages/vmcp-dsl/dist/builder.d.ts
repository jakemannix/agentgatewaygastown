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
import type { RegistryV2, SchemaDefinition, ServerDefinition, ToolDefinitionV2, ToolSourceV2, ToolProvision, AgentDefinition, AgentSkill, AgentCapabilities, AgentExtension, Dependency, JSONSchema, PatternSpec, OutputTransform } from './types.js';
/**
 * Builder for schema definitions
 */
export declare class SchemaBuilder {
    private def;
    constructor(name: string, version: string);
    description(desc: string): this;
    schema(schema: JSONSchema): this;
    metadata(key: string, value: unknown): this;
    build(): SchemaDefinition;
}
/**
 * Create a schema builder
 */
export declare function schema(name: string, version: string): SchemaBuilder;
/**
 * Builder for server definitions
 */
export declare class ServerBuilder {
    private def;
    constructor(name: string, version: string);
    description(desc: string): this;
    provides(tool: string, version: string): this;
    providesAll(provisions: ToolProvision[]): this;
    deprecated(message?: string): this;
    metadata(key: string, value: unknown): this;
    build(): ServerDefinition;
}
/**
 * Create a server builder
 */
export declare function server(name: string, version: string): ServerBuilder;
/**
 * Builder for tool definitions v2
 */
export declare class ToolV2Builder {
    private def;
    constructor(name: string, version: string);
    description(desc: string): this;
    /**
     * Set the source for a 1:1 tool mapping
     */
    source(server: string, serverVersion: string, tool: string): this;
    /**
     * Set full source configuration
     */
    sourceConfig(config: ToolSourceV2): this;
    /**
     * Set the pattern specification for a composition
     */
    spec(pattern: PatternSpec): this;
    /**
     * Add a tool dependency
     */
    dependsOnTool(name: string, version: string): this;
    /**
     * Add an agent dependency
     */
    dependsOnAgent(name: string, version: string, skill?: string): this;
    /**
     * Add multiple dependencies
     */
    dependsOn(deps: Dependency[]): this;
    /**
     * Set inline input schema
     */
    inputSchema(schema: JSONSchema): this;
    /**
     * Set input schema reference
     */
    inputSchemaRef(ref: string): this;
    /**
     * Set inline output schema
     */
    outputSchema(schema: JSONSchema): this;
    /**
     * Set output schema reference
     */
    outputSchemaRef(ref: string): this;
    /**
     * Set output transform
     */
    outputTransform(transform: OutputTransform): this;
    metadata(key: string, value: unknown): this;
    build(): ToolDefinitionV2;
}
/**
 * Create a tool v2 builder
 */
export declare function toolV2(name: string, version: string): ToolV2Builder;
/**
 * Builder for agent skill definitions
 */
export declare class AgentSkillBuilder {
    private def;
    constructor(id: string, name: string);
    description(desc: string): this;
    tags(...tags: string[]): this;
    examples(...examples: string[]): this;
    inputModes(...modes: string[]): this;
    outputModes(...modes: string[]): this;
    inputSchema(schema: JSONSchema): this;
    inputSchemaRef(ref: string): this;
    outputSchema(schema: JSONSchema): this;
    outputSchemaRef(ref: string): this;
    build(): AgentSkill;
}
/**
 * Create an agent skill builder
 */
export declare function skill(id: string, name: string): AgentSkillBuilder;
/**
 * Builder for agent definitions
 */
export declare class AgentBuilder {
    private def;
    constructor(name: string, version: string);
    description(desc: string): this;
    url(url: string): this;
    protocolVersion(version: string): this;
    defaultInputModes(...modes: string[]): this;
    defaultOutputModes(...modes: string[]): this;
    skill(skill: AgentSkill): this;
    skills(...skills: AgentSkill[]): this;
    streaming(enabled?: boolean): this;
    pushNotifications(enabled?: boolean): this;
    stateTransitionHistory(enabled?: boolean): this;
    extension(ext: AgentExtension): this;
    /**
     * Add SBOM extension with dependencies
     */
    sbom(depends: Dependency[]): this;
    provider(organization: string, url?: string): this;
    capabilities(caps: AgentCapabilities): this;
    build(): AgentDefinition;
}
/**
 * Create an agent builder
 */
export declare function agent(name: string, version: string): AgentBuilder;
/**
 * Builder for Registry v2
 */
export declare class RegistryV2Builder {
    private registry;
    constructor();
    schema(def: SchemaDefinition): this;
    schemas(...defs: SchemaDefinition[]): this;
    server(def: ServerDefinition): this;
    servers(...defs: ServerDefinition[]): this;
    tool(def: ToolDefinitionV2): this;
    tools(...defs: ToolDefinitionV2[]): this;
    agent(def: AgentDefinition): this;
    agents(...defs: AgentDefinition[]): this;
    build(): RegistryV2;
    /**
     * Compile to JSON string
     */
    toJSON(pretty?: boolean): string;
}
/**
 * Create a Registry v2 builder
 */
export declare function registryV2(): RegistryV2Builder;
/**
 * Create a tool dependency
 */
export declare function toolDep(name: string, version: string): Dependency;
/**
 * Create an agent dependency
 */
export declare function agentDep(name: string, version: string, skill?: string): Dependency;
/**
 * Create a schema reference string
 */
export declare function schemaRef(name: string, version: string): string;
//# sourceMappingURL=builder.d.ts.map