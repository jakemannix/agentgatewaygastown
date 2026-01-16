import { describe, it, expect } from 'vitest';
import { tool, ToolBuilder } from '../tool';
import type { ToolImplementation } from '../types';

describe('ToolBuilder', () => {
  describe('source tools', () => {
    it('should create a basic source tool', () => {
      const weatherTool = tool('get_weather')
        .description('Get weather information')
        .source('weather-backend', 'fetch_weather')
        .build();

      expect(weatherTool.name).toBe('get_weather');
      expect(weatherTool.description).toBe('Get weather information');
      // Source is nested under implementation
      const impl = weatherTool.implementation as { source: { target: string; tool: string } };
      expect(impl.source).toBeDefined();
      expect(impl.source.target).toBe('weather-backend');
      expect(impl.source.tool).toBe('fetch_weather');
    });

    it('should support defaults', () => {
      const weatherTool = tool('get_weather')
        .source('weather-backend', 'fetch_weather')
        .default('units', 'metric')
        .default('api_key', 'secret')
        .build();

      const impl = weatherTool.implementation as { source: { defaults?: Record<string, unknown> } };
      expect(impl.source.defaults).toEqual({
        units: 'metric',
        api_key: 'secret',
      });
    });

    it('should support hiding fields', () => {
      const weatherTool = tool('get_weather')
        .source('weather-backend', 'fetch_weather')
        .hideFields(['debug_mode', 'raw_output'])
        .build();

      const impl = weatherTool.implementation as { source: { hideFields?: string[] } };
      expect(impl.source.hideFields).toEqual(['debug_mode', 'raw_output']);
    });

    it('should support metadata', () => {
      const weatherTool = tool('get_weather')
        .source('weather-backend', 'fetch_weather')
        .metadata({ owner: 'weather-team', priority: 1 })
        .build();

      expect(weatherTool.metadata).toEqual({
        owner: 'weather-team',
        priority: 1,
      });
    });

    it('should support version', () => {
      const weatherTool = tool('get_weather')
        .source('weather-backend', 'fetch_weather')
        .version('1.0.0')
        .build();

      expect(weatherTool.version).toBe('1.0.0');
    });
  });

  describe('composition tools', () => {
    it('should create a tool with a pattern spec', () => {
      const searchTool = tool('multi_search')
        .description('Search multiple sources')
        .composition({
          scatterGather: {
            targets: [{ tool: 'search_web' }, { tool: 'search_arxiv' }],
            aggregation: { ops: [{ flatten: true }] },
          },
        })
        .build();

      expect(searchTool.name).toBe('multi_search');
      // Spec is nested under implementation
      const impl = searchTool.implementation as { spec: { scatterGather?: { targets: unknown[] } } };
      expect(impl.spec).toBeDefined();
      expect(impl.spec.scatterGather).toBeDefined();
      expect(impl.spec.scatterGather?.targets).toHaveLength(2);
    });
  });

  describe('output transform', () => {
    it('should support output transformation', () => {
      const weatherTool = tool('get_weather')
        .source('weather-backend', 'fetch_weather')
        .outputTransform({
          mappings: {
            temperature: { path: '$.data.temp' },
            conditions: { path: '$.data.weather' },
          },
        })
        .build();

      expect(weatherTool.outputTransform).toBeDefined();
      expect(weatherTool.outputTransform?.mappings.temperature).toEqual({
        path: '$.data.temp',
      });
    });
  });
});

