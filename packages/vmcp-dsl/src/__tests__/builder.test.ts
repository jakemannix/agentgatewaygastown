import { describe, it, expect } from 'vitest';
import {
  RegistryV2Builder,
  registryV2,
  SchemaBuilder,
  schema,
  ServerBuilder,
  server,
  ToolV2Builder,
  toolV2,
  AgentBuilder,
  agent,
  AgentSkillBuilder,
  skill,
  toolDep,
  agentDep,
  schemaRef,
} from '../builder';
import {
  isSchemaRef,
  hasSourceV2,
  hasSpecV2,
  isToolDependency,
  isAgentDependency,
  parseSchemaRef,
} from '../types';

describe('RegistryV2Builder', () => {
  it('should create an empty registry v2', () => {
    const registry = registryV2().build();

    expect(registry.schemaVersion).toBe('2.0');
    expect(registry.schemas).toHaveLength(0);
    expect(registry.servers).toHaveLength(0);
    expect(registry.tools).toHaveLength(0);
    expect(registry.agents).toHaveLength(0);
  });

  it('should build a valid registry', () => {
    const registry = new RegistryV2Builder()
      .schema({
        name: 'SearchQuery',
        version: '1.0.0',
        schema: { type: 'object' },
      })
      .server({
        name: 'doc-service',
        version: '1.0.0',
        provides: [{ tool: 'search', version: '1.0.0' }],
      })
      .tool({
        name: 'search',
        version: '1.0.0',
        source: {
          server: 'doc-service',
          serverVersion: '1.0.0',
          tool: 'search',
        },
      })
      .build();

    expect(registry.schemaVersion).toBe('2.0');
    expect(registry.schemas).toHaveLength(1);
    expect(registry.servers).toHaveLength(1);
    expect(registry.tools).toHaveLength(1);
  });

  it('should support schema refs', () => {
    const tool = toolV2('search', '1.0.0')
      .inputSchemaRef('#SearchQuery:1.0.0')
      .build();

    expect(tool.inputSchema).toEqual({ $ref: '#SearchQuery:1.0.0' });
    expect(isSchemaRef(tool.inputSchema!)).toBe(true);
  });

  it('should support dependencies', () => {
    const tool = toolV2('pipeline', '1.0.0')
      .dependsOnTool('fetch', '1.2.3')
      .dependsOnAgent('summarizer', '2.0.0', 'summarize')
      .build();

    expect(tool.depends).toHaveLength(2);
    expect(tool.depends![0].type).toBe('tool');
    expect(tool.depends![1].type).toBe('agent');
    expect(tool.depends![1].skill).toBe('summarize');
  });
});

describe('SchemaBuilder', () => {
  it('should build a schema definition', () => {
    const schemaDef = schema('SearchQuery', '1.0.0')
      .description('Standard search query input')
      .schema({
        type: 'object',
        properties: {
          query: { type: 'string' },
          limit: { type: 'integer', default: 10 },
        },
        required: ['query'],
      })
      .metadata('owner', 'platform-team')
      .build();

    expect(schemaDef.name).toBe('SearchQuery');
    expect(schemaDef.version).toBe('1.0.0');
    expect(schemaDef.description).toBe('Standard search query input');
    expect(schemaDef.schema.type).toBe('object');
    expect(schemaDef.metadata?.owner).toBe('platform-team');
  });
});

describe('ServerBuilder', () => {
  it('should build a server definition', () => {
    const serverDef = server('document-service', '1.2.0')
      .description('SQLite-backed document service')
      .provides('search_documents', '1.0.0')
      .provides('create_document', '1.0.0')
      .metadata('repo', 'github.com/example/doc-service')
      .build();

    expect(serverDef.name).toBe('document-service');
    expect(serverDef.version).toBe('1.2.0');
    expect(serverDef.provides).toHaveLength(2);
    expect(serverDef.provides[0]).toEqual({ tool: 'search_documents', version: '1.0.0' });
  });

  it('should support deprecated servers', () => {
    const serverDef = server('old-service', '0.9.0')
      .deprecated('Use new-service instead')
      .build();

    expect(serverDef.deprecated).toBe(true);
    expect(serverDef.deprecationMessage).toBe('Use new-service instead');
  });
});

describe('ToolV2Builder', () => {
  it('should build a source tool', () => {
    const toolDef = toolV2('search_documents', '1.0.0')
      .description('Semantic search across documents')
      .source('document-service', '1.2.0', 'search_documents')
      .inputSchemaRef('#SearchQuery:1.0.0')
      .outputSchemaRef('#SearchResults:1.0.0')
      .build();

    expect(toolDef.name).toBe('search_documents');
    expect(toolDef.version).toBe('1.0.0');
    expect(hasSourceV2(toolDef)).toBe(true);
    expect(toolDef.source?.server).toBe('document-service');
    expect(toolDef.source?.serverVersion).toBe('1.2.0');
  });

  it('should build a composition tool with dependencies', () => {
    const toolDef = toolV2('fetch_and_store', '1.0.0')
      .description('Pipeline: fetch URL and store as document')
      .dependsOnTool('fetch', '1.2.3')
      .dependsOnTool('create_document', '1.0.0')
      .spec({
        pipeline: {
          steps: [
            { id: 'fetch', operation: { tool: { name: 'fetch' } } },
            { id: 'store', operation: { tool: { name: 'create_document' } } },
          ],
        },
      })
      .build();

    expect(toolDef.name).toBe('fetch_and_store');
    expect(hasSpecV2(toolDef)).toBe(true);
    expect(toolDef.depends).toHaveLength(2);
  });

  it('should support inline schemas', () => {
    const toolDef = toolV2('get_time', '1.0.0')
      .inputSchema({
        type: 'object',
        properties: {
          timezone: { type: 'string' },
        },
        required: ['timezone'],
      })
      .build();

    expect(isSchemaRef(toolDef.inputSchema!)).toBe(false);
    expect((toolDef.inputSchema as any).type).toBe('object');
  });
});

describe('AgentBuilder', () => {
  it('should build an agent definition', () => {
    const agentDef = agent('research-agent', '2.1.0')
      .description('Autonomous research assistant')
      .url('http://research-agent.internal:9000')
      .protocolVersion('0.2.1')
      .defaultInputModes('text', 'application/json')
      .defaultOutputModes('text', 'application/json')
      .streaming()
      .stateTransitionHistory()
      .provider('AI Platform Team', 'https://platform.internal')
      .build();

    expect(agentDef.name).toBe('research-agent');
    expect(agentDef.version).toBe('2.1.0');
    expect(agentDef.url).toBe('http://research-agent.internal:9000');
    expect(agentDef.defaultInputModes).toContain('text');
    expect(agentDef.capabilities.streaming).toBe(true);
    expect(agentDef.capabilities.stateTransitionHistory).toBe(true);
    expect(agentDef.provider?.organization).toBe('AI Platform Team');
  });

  it('should add skills to agent', () => {
    const researchSkill = skill('research_topic', 'Research Topic')
      .description('Deep research on a topic with citations')
      .tags('research', 'analysis', 'citations')
      .examples('Research quantum computing', 'Analyze market trends')
      .inputModes('text', 'application/json')
      .outputModes('text', 'application/json')
      .inputSchemaRef('#ResearchRequest:1.0.0')
      .outputSchemaRef('#ResearchReport:1.0.0')
      .build();

    const agentDef = agent('research-agent', '2.1.0')
      .description('Autonomous research assistant')
      .url('http://research-agent.internal:9000')
      .skill(researchSkill)
      .build();

    expect(agentDef.skills).toHaveLength(1);
    expect(agentDef.skills[0].id).toBe('research_topic');
    expect(agentDef.skills[0].tags).toContain('research');
  });

  it('should add SBOM extension with dependencies', () => {
    const agentDef = agent('research-agent', '2.1.0')
      .description('Autonomous research assistant')
      .url('http://research-agent.internal:9000')
      .sbom([
        toolDep('search_documents', '1.0.0'),
        toolDep('fetch', '1.2.3'),
        agentDep('summarizer-agent', '2.0.0', 'summarize'),
      ])
      .build();

    expect(agentDef.capabilities.extensions).toHaveLength(1);
    expect(agentDef.capabilities.extensions![0].uri).toBe('urn:agentgateway:sbom');

    const params = agentDef.capabilities.extensions![0].params as { depends: any[] };
    expect(params.depends).toHaveLength(3);
    expect(params.depends[0].type).toBe('tool');
    expect(params.depends[2].skill).toBe('summarize');
  });
});

describe('Type Guards', () => {
  it('isSchemaRef should correctly identify refs', () => {
    expect(isSchemaRef({ $ref: '#SearchQuery:1.0.0' })).toBe(true);
    expect(isSchemaRef({ type: 'object' })).toBe(false);
  });

  it('isToolDependency should identify tool deps', () => {
    expect(isToolDependency(toolDep('fetch', '1.0.0'))).toBe(true);
    expect(isToolDependency(agentDep('agent', '1.0.0'))).toBe(false);
  });

  it('isAgentDependency should identify agent deps', () => {
    expect(isAgentDependency(agentDep('agent', '1.0.0'))).toBe(true);
    expect(isAgentDependency(toolDep('fetch', '1.0.0'))).toBe(false);
  });

  it('parseSchemaRef should parse valid refs', () => {
    const result = parseSchemaRef('#SearchQuery:1.0.0');
    expect(result).toEqual({ name: 'SearchQuery', version: '1.0.0' });
  });

  it('parseSchemaRef should return null for invalid refs', () => {
    expect(parseSchemaRef('invalid')).toBeNull();
    expect(parseSchemaRef('#NoVersion')).toBeNull();
    expect(parseSchemaRef('NoHash:1.0.0')).toBeNull();
  });
});

describe('Dependency Helpers', () => {
  it('toolDep should create tool dependency', () => {
    const dep = toolDep('fetch', '1.2.3');
    expect(dep.type).toBe('tool');
    expect(dep.name).toBe('fetch');
    expect(dep.version).toBe('1.2.3');
    expect(dep.skill).toBeUndefined();
  });

  it('agentDep should create agent dependency', () => {
    const dep = agentDep('summarizer', '2.0.0', 'summarize');
    expect(dep.type).toBe('agent');
    expect(dep.name).toBe('summarizer');
    expect(dep.version).toBe('2.0.0');
    expect(dep.skill).toBe('summarize');
  });

  it('schemaRef should create ref string', () => {
    expect(schemaRef('SearchQuery', '1.0.0')).toBe('#SearchQuery:1.0.0');
  });
});

describe('Complex Registry', () => {
  it('should build a complete registry v2 with mixed content', () => {
    const registry = registryV2()
      // Schemas
      .schema(
        schema('SearchQuery', '1.0.0')
          .description('Standard search input')
          .schema({
            type: 'object',
            properties: { query: { type: 'string' } },
            required: ['query'],
          })
          .build()
      )
      .schema(
        schema('SearchResult', '1.0.0')
          .schema({
            type: 'object',
            properties: {
              id: { type: 'string' },
              title: { type: 'string' },
            },
          })
          .build()
      )
      // Servers
      .server(
        server('doc-service', '1.2.0')
          .description('Document service')
          .provides('search_documents', '1.0.0')
          .provides('create_document', '1.0.0')
          .build()
      )
      // Tools
      .tool(
        toolV2('search_documents', '1.0.0')
          .description('Semantic search')
          .source('doc-service', '1.2.0', 'search_documents')
          .inputSchemaRef('#SearchQuery:1.0.0')
          .build()
      )
      .tool(
        toolV2('fetch_and_store', '1.0.0')
          .description('Pipeline tool')
          .dependsOnTool('fetch', '1.2.3')
          .spec({ pipeline: { steps: [] } })
          .build()
      )
      // Agents
      .agent(
        agent('research-agent', '2.1.0')
          .description('Research assistant')
          .url('http://localhost:9000')
          .skill(
            skill('research', 'Research')
              .description('Research a topic')
              .tags('research')
              .inputModes('text')
              .outputModes('text')
              .build()
          )
          .streaming()
          .sbom([toolDep('search_documents', '1.0.0')])
          .build()
      )
      .build();

    expect(registry.schemaVersion).toBe('2.0');
    expect(registry.schemas).toHaveLength(2);
    expect(registry.servers).toHaveLength(1);
    expect(registry.tools).toHaveLength(2);
    expect(registry.agents).toHaveLength(1);

    // Verify tool lookup
    const searchTool = registry.tools.find((t) => t.name === 'search_documents');
    expect(searchTool?.source?.server).toBe('doc-service');

    // Verify agent
    const researchAgent = registry.agents.find((a) => a.name === 'research-agent');
    expect(researchAgent?.skills).toHaveLength(1);
    expect(researchAgent?.capabilities.streaming).toBe(true);
  });

  it('should produce valid JSON output', () => {
    const registry = registryV2()
      .schema(schema('Test', '1.0.0').schema({ type: 'string' }).build())
      .server(server('test-server', '1.0.0').provides('test', '1.0.0').build())
      .tool(toolV2('test', '1.0.0').source('test-server', '1.0.0', 'test').build())
      .build();

    const json = JSON.stringify(registry, null, 2);
    const parsed = JSON.parse(json);

    expect(parsed.schemaVersion).toBe('2.0');
    expect(parsed.schemas[0].name).toBe('Test');
    expect(parsed.servers[0].name).toBe('test-server');
    expect(parsed.tools[0].name).toBe('test');
  });
});
