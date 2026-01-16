import { describe, it, expect } from 'vitest';
import { pipeline, scatterGather, filter, schemaMap, mapEach, agg } from '../patterns';

describe('Pipeline Pattern', () => {
  it('should create a simple pipeline', () => {
    const spec = pipeline()
      .step('search', 'web_search')
      .step('summarize', 'summarize_text')
      .build();

    expect(spec.pipeline).toBeDefined();
    expect(spec.pipeline?.steps).toHaveLength(2);
    expect(spec.pipeline?.steps[0].id).toBe('search');
    expect(spec.pipeline?.steps[0].operation.tool?.name).toBe('web_search');
  });

  it('should support step with input binding', () => {
    const spec = pipeline()
      .step('search', 'web_search')
      .addStep({
        id: 'process',
        operation: { tool: { name: 'process_results' } },
        input: { step: { stepId: 'search', path: '$.results' } },
      })
      .build();

    expect(spec.pipeline?.steps[1].input?.step?.stepId).toBe('search');
    expect(spec.pipeline?.steps[1].input?.step?.path).toBe('$.results');
  });

  it('should support inline pattern operations', () => {
    const spec = pipeline()
      .step('search', 'web_search')
      .addStep({
        id: 'filter',
        operation: {
          pattern: {
            filter: {
              predicate: { field: '$.score', op: 'gt', value: { numberValue: 0.5 } },
            },
          },
        },
      })
      .build();

    expect(spec.pipeline?.steps[1].operation.pattern?.filter).toBeDefined();
  });
});

describe('ScatterGather Pattern', () => {
  it('should create a basic scatter-gather', () => {
    const spec = scatterGather()
      .targets('search_web', 'search_arxiv', 'search_wikipedia')
      .build();

    expect(spec.scatterGather).toBeDefined();
    expect(spec.scatterGather?.targets).toHaveLength(3);
    expect(spec.scatterGather?.targets[0].tool).toBe('search_web');
  });

  it('should support aggregation operations', () => {
    const spec = scatterGather()
      .targets('search_web', 'search_arxiv')
      .aggregate(agg().flatten().sortDesc('$.score').limit(10))
      .build();

    expect(spec.scatterGather?.aggregation?.ops).toHaveLength(3);
    expect(spec.scatterGather?.aggregation?.ops[0].flatten).toBe(true);
    expect(spec.scatterGather?.aggregation?.ops[1].sort?.order).toBe('desc');
    expect(spec.scatterGather?.aggregation?.ops[2].limit?.count).toBe(10);
  });

  it('should support timeout and failFast', () => {
    const spec = scatterGather()
      .targets('search_web')
      .timeout(5000)
      .failFast(true)
      .build();

    expect(spec.scatterGather?.timeoutMs).toBe(5000);
    expect(spec.scatterGather?.failFast).toBe(true);
  });
});

describe('Aggregation Builder', () => {
  it('should chain multiple operations', () => {
    const ops = agg()
      .flatten()
      .merge()
      .sortAsc('$.date')
      .dedupe('$.id')
      .limit(100)
      .build();

    expect(ops.ops).toHaveLength(5);
    expect(ops.ops[0].flatten).toBe(true);
    expect(ops.ops[1].merge).toBe(true);
    expect(ops.ops[2].sort?.field).toBe('$.date');
    expect(ops.ops[2].sort?.order).toBe('asc');
    expect(ops.ops[3].dedupe?.field).toBe('$.id');
    expect(ops.ops[4].limit?.count).toBe(100);
  });
});

describe('Filter Pattern', () => {
  it('should create a filter with predicate', () => {
    const spec = filter()
      .field('$.score')
      .gt(0.5)
      .build();

    expect(spec.filter).toBeDefined();
    expect(spec.filter?.predicate.field).toBe('$.score');
    expect(spec.filter?.predicate.op).toBe('gt');
    expect(spec.filter?.predicate.value).toEqual({ numberValue: 0.5 });
  });

  it('should support various comparison operators', () => {
    const eqSpec = filter().field('$.status').eq('active').build();
    expect(eqSpec.filter?.predicate.op).toBe('eq');
    expect(eqSpec.filter?.predicate.value).toEqual({ stringValue: 'active' });

    const ltSpec = filter().field('$.age').lt(18).build();
    expect(ltSpec.filter?.predicate.op).toBe('lt');

    const containsSpec = filter().field('$.tags').contains('important').build();
    expect(containsSpec.filter?.predicate.op).toBe('contains');
  });

  it('should support in operator', () => {
    const spec = filter().field('$.status').in(['active', 'pending']).build();
    expect(spec.filter?.predicate.op).toBe('in');
    expect(spec.filter?.predicate.value).toEqual({
      listValue: [{ stringValue: 'active' }, { stringValue: 'pending' }],
    });
  });
});

describe('SchemaMap Pattern', () => {
  it('should create path mappings', () => {
    const spec = schemaMap()
      .field('title', '$.name')
      .field('author', '$.metadata.author')
      .build();

    expect(spec.schemaMap).toBeDefined();
    expect(spec.schemaMap?.mappings.title).toEqual({ path: '$.name' });
    expect(spec.schemaMap?.mappings.author).toEqual({ path: '$.metadata.author' });
  });

  it('should support coalesce', () => {
    const spec = schemaMap()
      .coalesce('url', ['$.pdf_url', '$.web_url', '$.fallback_url'])
      .build();

    expect(spec.schemaMap?.mappings.url).toEqual({
      coalesce: { paths: ['$.pdf_url', '$.web_url', '$.fallback_url'] },
    });
  });

  it('should support literal values', () => {
    // Use literal method with different types - check actual API
    const spec = schemaMap()
      .literal('source', 'web')
      .literal('version', 1)
      .literal('verified', true)
      .build();

    expect(spec.schemaMap?.mappings.source).toEqual({
      literal: { stringValue: 'web' },
    });
    expect(spec.schemaMap?.mappings.version).toEqual({
      literal: { numberValue: 1 },
    });
    expect(spec.schemaMap?.mappings.verified).toEqual({
      literal: { boolValue: true },
    });
  });

  it('should support template strings', () => {
    const spec = schemaMap()
      .template('fullName', '${firstName} ${lastName}', {
        firstName: '$.first_name',
        lastName: '$.last_name',
      })
      .build();

    expect(spec.schemaMap?.mappings.fullName).toEqual({
      template: {
        template: '${firstName} ${lastName}',
        vars: {
          firstName: '$.first_name',
          lastName: '$.last_name',
        },
      },
    });
  });

  it('should support concat', () => {
    const spec = schemaMap()
      .concat('tags', ['$.tag1', '$.tag2', '$.tag3'], ', ')
      .build();

    expect(spec.schemaMap?.mappings.tags).toEqual({
      concat: {
        paths: ['$.tag1', '$.tag2', '$.tag3'],
        separator: ', ',
      },
    });
  });

  it('should support nested mappings', () => {
    const spec = schemaMap()
      .nested('author', {
        name: { path: '$.author_name' },
        email: { path: '$.author_email' },
      })
      .build();

    // Actual output format may vary - check the actual structure
    expect(spec.schemaMap?.mappings.author).toHaveProperty('nested');
    expect(spec.schemaMap?.mappings.author.nested).toHaveProperty('name');
    expect(spec.schemaMap?.mappings.author.nested).toHaveProperty('email');
  });
});

describe('MapEach Pattern', () => {
  it('should create mapEach with a tool', () => {
    const spec = mapEach()
      .tool('process_item')
      .build();

    expect(spec.mapEach).toBeDefined();
    expect(spec.mapEach?.inner.tool).toBe('process_item');
  });

  it('should create mapEach with a pattern', () => {
    const innerPattern = schemaMap()
      .field('title', '$.name')
      .build();

    const spec = mapEach()
      .pattern(innerPattern)
      .build();

    expect(spec.mapEach?.inner.pattern?.schemaMap).toBeDefined();
  });
});

