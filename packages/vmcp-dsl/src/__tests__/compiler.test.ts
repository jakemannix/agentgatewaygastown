import { describe, it, expect } from 'vitest';
import { createRegistry, RegistryBuilder } from '../compiler';
import { tool } from '../tool';
import { pipeline, scatterGather, agg, schemaMap } from '../patterns';

describe('RegistryBuilder', () => {
  it('should create an empty registry', () => {
    const registry = createRegistry().build();

    expect(registry.schemaVersion).toBe('1.0');
    expect(registry.tools).toHaveLength(0);
  });

  it('should add tools to the registry', () => {
    const weatherTool = tool('get_weather')
      .source('weather-backend', 'fetch_weather')
      .build();

    const searchTool = tool('search')
      .source('search-backend', 'web_search')
      .build();

    const registry = createRegistry()
      .add(weatherTool)
      .add(searchTool)
      .build();

    expect(registry.tools).toHaveLength(2);
    expect(registry.tools[0].name).toBe('get_weather');
    expect(registry.tools[1].name).toBe('search');
  });

  it('should add multiple tools at once', () => {
    const tool1 = tool('tool1').source('backend', 'tool1').build();
    const tool2 = tool('tool2').source('backend', 'tool2').build();
    const tool3 = tool('tool3').source('backend', 'tool3').build();

    const registry = createRegistry()
      .addAll(tool1, tool2, tool3)
      .build();

    expect(registry.tools).toHaveLength(3);
  });
});

describe('Registry Validation', () => {
  it('should pass validation for valid registry', () => {
    const weatherTool = tool('get_weather')
      .source('weather-backend', 'fetch_weather')
      .build();

    const result = createRegistry()
      .add(weatherTool)
      .validate();

    expect(result.valid).toBe(true);
    expect(result.errors).toHaveLength(0);
  });

  it('should detect duplicate tool names', () => {
    const tool1 = tool('duplicate_name').source('a', 'a').build();
    const tool2 = tool('duplicate_name').source('b', 'b').build();

    const result = createRegistry()
      .add(tool1)
      .add(tool2)
      .validate();

    expect(result.valid).toBe(false);
    // errors might be objects with message property
    expect(result.errors.some((e: any) => 
      typeof e === 'string' ? e.includes('duplicate') : e?.message?.includes('duplicate')
    )).toBe(true);
  });

  // TODO: Implement stricter validation for missing tool names
  it.skip('should detect missing tool name', () => {
    // Create a malformed tool by casting
    const badTool = { implementation: { source: { target: 'a', tool: 'a' } } } as any;

    const result = createRegistry()
      .add(badTool)
      .validate();

    expect(result.valid).toBe(false);
    expect(result.errors.some((e: any) => 
      typeof e === 'string' ? e.includes('name') : e?.message?.includes('name')
    )).toBe(true);
  });

  // TODO: Implement tool reference tracking for validation warnings
  it.skip('should warn about unresolved tool references', () => {
    const pipelineComp = tool('my_pipeline')
      .composition(
        pipeline()
          .step('search', 'nonexistent_tool')
          .build()
      )
      .build();

    const result = createRegistry()
      .add(pipelineComp)
      .validate();

    // This should generate a warning (not an error) about unresolved reference
    expect(result.warnings.some((w: any) => 
      typeof w === 'string' ? w.includes('nonexistent_tool') : w?.message?.includes('nonexistent_tool')
    )).toBe(true);
  });
});

describe('Complex Registry', () => {
  it('should build a complete registry with mixed tools', () => {
    // Virtual tool
    const weatherTool = tool('get_weather')
      .description('Get weather information')
      .source('weather-backend', 'fetch_weather')
      .default('units', 'metric')
      .build();

    // Scatter-gather composition
    const multiSearch = tool('multi_search')
      .description('Search multiple sources')
      .composition(
        scatterGather()
          .targets('search_web', 'search_arxiv')
          .aggregate(agg().flatten().sortDesc('$.score').limit(20))
          .timeout(5000)
          .build()
      )
      .build();

    // Pipeline composition referencing multi_search
    const researchPipeline = tool('research_pipeline')
      .description('End-to-end research')
      .composition(
        pipeline()
          .step('search', 'multi_search')
          .addStep({
            id: 'normalize',
            operation: {
              pattern: schemaMap()
                .field('title', '$.name')
                .coalesce('url', ['$.pdf_url', '$.web_url'])
                .literal('source', 'research')
                .build(),
            },
          })
          .build()
      )
      .build();

    const registry = createRegistry()
      .addAll(weatherTool, multiSearch, researchPipeline)
      .build();

    expect(registry.tools).toHaveLength(3);
    
    // Verify structure - implementation is nested
    const weather = registry.tools.find(t => t.name === 'get_weather');
    expect(weather?.implementation).toBeDefined();

    const search = registry.tools.find(t => t.name === 'multi_search');
    expect(search?.implementation).toBeDefined();

    const pipelineTool = registry.tools.find(t => t.name === 'research_pipeline');
    expect(pipelineTool?.implementation).toBeDefined();
  });

  it('should produce valid JSON output', () => {
    const weatherTool = tool('get_weather')
      .source('weather', 'fetch')
      .default('key', 'value')
      .build();

    const registry = createRegistry().add(weatherTool).build();

    // Should not throw
    const json = JSON.stringify(registry, null, 2);
    const parsed = JSON.parse(json);

    expect(parsed.schemaVersion).toBe('1.0');
    expect(parsed.tools[0].name).toBe('get_weather');
  });
});
