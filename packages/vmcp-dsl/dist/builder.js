"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.RegistryV2Builder = exports.AgentBuilder = exports.AgentSkillBuilder = exports.ToolV2Builder = exports.ServerBuilder = exports.SchemaBuilder = void 0;
exports.schema = schema;
exports.server = server;
exports.toolV2 = toolV2;
exports.skill = skill;
exports.agent = agent;
exports.registryV2 = registryV2;
exports.toolDep = toolDep;
exports.agentDep = agentDep;
exports.schemaRef = schemaRef;
// =============================================================================
// Schema Builder
// =============================================================================
/**
 * Builder for schema definitions
 */
class SchemaBuilder {
    def;
    constructor(name, version) {
        this.def = {
            name,
            version,
            schema: {},
        };
    }
    description(desc) {
        this.def.description = desc;
        return this;
    }
    schema(schema) {
        this.def.schema = schema;
        return this;
    }
    metadata(key, value) {
        if (!this.def.metadata)
            this.def.metadata = {};
        this.def.metadata[key] = value;
        return this;
    }
    build() {
        return { ...this.def };
    }
}
exports.SchemaBuilder = SchemaBuilder;
/**
 * Create a schema builder
 */
function schema(name, version) {
    return new SchemaBuilder(name, version);
}
// =============================================================================
// Server Builder
// =============================================================================
/**
 * Builder for server definitions
 */
class ServerBuilder {
    def;
    constructor(name, version) {
        this.def = {
            name,
            version,
            provides: [],
        };
    }
    description(desc) {
        this.def.description = desc;
        return this;
    }
    provides(tool, version) {
        this.def.provides.push({ tool, version });
        return this;
    }
    providesAll(provisions) {
        this.def.provides.push(...provisions);
        return this;
    }
    deprecated(message) {
        this.def.deprecated = true;
        if (message)
            this.def.deprecationMessage = message;
        return this;
    }
    metadata(key, value) {
        if (!this.def.metadata)
            this.def.metadata = {};
        this.def.metadata[key] = value;
        return this;
    }
    build() {
        return { ...this.def };
    }
}
exports.ServerBuilder = ServerBuilder;
/**
 * Create a server builder
 */
function server(name, version) {
    return new ServerBuilder(name, version);
}
// =============================================================================
// Tool v2 Builder
// =============================================================================
/**
 * Builder for tool definitions v2
 */
class ToolV2Builder {
    def;
    constructor(name, version) {
        this.def = {
            name,
            version,
        };
    }
    description(desc) {
        this.def.description = desc;
        return this;
    }
    /**
     * Set the source for a 1:1 tool mapping
     */
    source(server, serverVersion, tool) {
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
    sourceConfig(config) {
        this.def.source = config;
        return this;
    }
    /**
     * Set the pattern specification for a composition
     */
    spec(pattern) {
        this.def.spec = pattern;
        return this;
    }
    /**
     * Add a tool dependency
     */
    dependsOnTool(name, version) {
        if (!this.def.depends)
            this.def.depends = [];
        this.def.depends.push({ type: 'tool', name, version });
        return this;
    }
    /**
     * Add an agent dependency
     */
    dependsOnAgent(name, version, skill) {
        if (!this.def.depends)
            this.def.depends = [];
        this.def.depends.push({ type: 'agent', name, version, skill });
        return this;
    }
    /**
     * Add multiple dependencies
     */
    dependsOn(deps) {
        if (!this.def.depends)
            this.def.depends = [];
        this.def.depends.push(...deps);
        return this;
    }
    /**
     * Set inline input schema
     */
    inputSchema(schema) {
        this.def.inputSchema = schema;
        return this;
    }
    /**
     * Set input schema reference
     */
    inputSchemaRef(ref) {
        this.def.inputSchema = { $ref: ref };
        return this;
    }
    /**
     * Set inline output schema
     */
    outputSchema(schema) {
        this.def.outputSchema = schema;
        return this;
    }
    /**
     * Set output schema reference
     */
    outputSchemaRef(ref) {
        this.def.outputSchema = { $ref: ref };
        return this;
    }
    /**
     * Set output transform
     */
    outputTransform(transform) {
        this.def.outputTransform = transform;
        return this;
    }
    metadata(key, value) {
        if (!this.def.metadata)
            this.def.metadata = {};
        this.def.metadata[key] = value;
        return this;
    }
    build() {
        return { ...this.def };
    }
}
exports.ToolV2Builder = ToolV2Builder;
/**
 * Create a tool v2 builder
 */
function toolV2(name, version) {
    return new ToolV2Builder(name, version);
}
// =============================================================================
// Agent Skill Builder
// =============================================================================
/**
 * Builder for agent skill definitions
 */
class AgentSkillBuilder {
    def;
    constructor(id, name) {
        this.def = {
            id,
            name,
            description: '',
            tags: [],
            inputModes: [],
            outputModes: [],
        };
    }
    description(desc) {
        this.def.description = desc;
        return this;
    }
    tags(...tags) {
        this.def.tags.push(...tags);
        return this;
    }
    examples(...examples) {
        if (!this.def.examples)
            this.def.examples = [];
        this.def.examples.push(...examples);
        return this;
    }
    inputModes(...modes) {
        this.def.inputModes.push(...modes);
        return this;
    }
    outputModes(...modes) {
        this.def.outputModes.push(...modes);
        return this;
    }
    inputSchema(schema) {
        this.def.inputSchema = schema;
        return this;
    }
    inputSchemaRef(ref) {
        this.def.inputSchema = { $ref: ref };
        return this;
    }
    outputSchema(schema) {
        this.def.outputSchema = schema;
        return this;
    }
    outputSchemaRef(ref) {
        this.def.outputSchema = { $ref: ref };
        return this;
    }
    build() {
        return { ...this.def };
    }
}
exports.AgentSkillBuilder = AgentSkillBuilder;
/**
 * Create an agent skill builder
 */
function skill(id, name) {
    return new AgentSkillBuilder(id, name);
}
// =============================================================================
// Agent Builder
// =============================================================================
/**
 * Builder for agent definitions
 */
class AgentBuilder {
    def;
    constructor(name, version) {
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
    description(desc) {
        this.def.description = desc;
        return this;
    }
    url(url) {
        this.def.url = url;
        return this;
    }
    protocolVersion(version) {
        this.def.protocolVersion = version;
        return this;
    }
    defaultInputModes(...modes) {
        this.def.defaultInputModes.push(...modes);
        return this;
    }
    defaultOutputModes(...modes) {
        this.def.defaultOutputModes.push(...modes);
        return this;
    }
    skill(skill) {
        this.def.skills.push(skill);
        return this;
    }
    skills(...skills) {
        this.def.skills.push(...skills);
        return this;
    }
    streaming(enabled = true) {
        this.def.capabilities.streaming = enabled;
        return this;
    }
    pushNotifications(enabled = true) {
        this.def.capabilities.pushNotifications = enabled;
        return this;
    }
    stateTransitionHistory(enabled = true) {
        this.def.capabilities.stateTransitionHistory = enabled;
        return this;
    }
    extension(ext) {
        if (!this.def.capabilities.extensions) {
            this.def.capabilities.extensions = [];
        }
        this.def.capabilities.extensions.push(ext);
        return this;
    }
    /**
     * Add SBOM extension with dependencies
     */
    sbom(depends) {
        return this.extension({
            uri: 'urn:agentgateway:sbom',
            description: 'Software bill of materials - tool and agent dependencies',
            required: true,
            params: { depends },
        });
    }
    provider(organization, url) {
        this.def.provider = { organization, url };
        return this;
    }
    capabilities(caps) {
        this.def.capabilities = { ...this.def.capabilities, ...caps };
        return this;
    }
    build() {
        return { ...this.def };
    }
}
exports.AgentBuilder = AgentBuilder;
/**
 * Create an agent builder
 */
function agent(name, version) {
    return new AgentBuilder(name, version);
}
// =============================================================================
// Registry v2 Builder
// =============================================================================
/**
 * Builder for Registry v2
 */
class RegistryV2Builder {
    registry;
    constructor() {
        this.registry = {
            schemaVersion: '2.0',
            schemas: [],
            servers: [],
            tools: [],
            agents: [],
        };
    }
    schema(def) {
        this.registry.schemas.push(def);
        return this;
    }
    schemas(...defs) {
        this.registry.schemas.push(...defs);
        return this;
    }
    server(def) {
        this.registry.servers.push(def);
        return this;
    }
    servers(...defs) {
        this.registry.servers.push(...defs);
        return this;
    }
    tool(def) {
        this.registry.tools.push(def);
        return this;
    }
    tools(...defs) {
        this.registry.tools.push(...defs);
        return this;
    }
    agent(def) {
        this.registry.agents.push(def);
        return this;
    }
    agents(...defs) {
        this.registry.agents.push(...defs);
        return this;
    }
    build() {
        return { ...this.registry };
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
}
exports.RegistryV2Builder = RegistryV2Builder;
/**
 * Create a Registry v2 builder
 */
function registryV2() {
    return new RegistryV2Builder();
}
// =============================================================================
// Dependency Helpers
// =============================================================================
/**
 * Create a tool dependency
 */
function toolDep(name, version) {
    return { type: 'tool', name, version };
}
/**
 * Create an agent dependency
 */
function agentDep(name, version, skill) {
    return { type: 'agent', name, version, skill };
}
/**
 * Create a schema reference string
 */
function schemaRef(name, version) {
    return `#${name}:${version}`;
}
//# sourceMappingURL=builder.js.map