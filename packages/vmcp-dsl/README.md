# @vmcp/dsl

TypeScript DSL for defining vMCP tool compositions that compile to JSON IR for agentgateway.

## Installation

```bash
npm install @vmcp/dsl
```

## Quick Start

```typescript
import { tool, pipeline, scatterGather, agg, filter, schemaMap, compile } from '@vmcp/dsl';

// Define a virtual tool (1:1 mapping to backend)
const weatherTool = tool('get_weather')
  .description('Get weather information')
  .source('weather', 'fetch_weather')
  .default('units', 'metric')
  .hideFields(['debug_mode'])
  .build();

// Define a composition (N:1 - multiple tools orchestrated)
const researchPipeline = tool('research')
  .description('Multi-source research pipeline')
  .composition(
    pipeline()
      .step('search', 'web_search')
      .step('filter', filter().field('$.score').gt(0.5).build())
      .step('summarize', 'summarize_text')
      .build()
  )
  .build();

// Compile to JSON IR
const json = compile(weatherTool, researchPipeline);
console.log(json);
```

## Pattern Builders

### Pipeline

Sequential execution of steps:

```typescript
const searchPipeline = pipeline()
  .add('search').tool('web_search').fromInput('$.query').then()
  .add('process').tool('process_results').fromStep('search', '$.results').then()
  .add('summarize').tool('summarize').then()
  .build();
```

### Scatter-Gather

Parallel execution with aggregation:

```typescript
const multiSearch = scatterGather()
  .targets('search_web', 'search_arxiv', 'search_wikipedia')
  .aggregate(
    agg()
      .flatten()
      .sortDesc('$.relevance')
      .dedupe('$.id')
      .limit(20)
  )
  .timeout(5000)
  .failOnError()
  .build();
```

### Filter

Predicate-based array filtering:

```typescript
const highQuality = filter()
  .field('$.score')
  .gt(0.7)
  .build();

const typeFilter = filter()
  .field('$.type')
  .in(['pdf', 'html'])
  .build();
```

### Schema Map

Field transformation and mapping:

```typescript
const normalize = schemaMap()
  .field('title', '$.paper_title')
  .field('author', '$.author_name')
  .coalesce('url', ['$.pdf_url', '$.web_url', '$.fallback_url'])
  .literal('source', 'arxiv')
  .template('citation', '{author} ({year})', { 
    author: 'author_name', 
    year: 'publication_year' 
  })
  .concat('fullName', ['first_name', 'last_name'], ' ')
  .build();
```

### Map Each

Apply operation to each array element:

```typescript
const fetchAll = mapEach()
  .tool('fetch_document')
  .build();

const normalizeAll = mapEach()
  .pattern(schemaMap().field('name', '$.title').build())
  .build();
```

## CLI Usage

The `vmcp-compile` CLI compiles TypeScript definitions to JSON:

```bash
# Compile to stdout
vmcp-compile tools.ts

# Compile to file
vmcp-compile tools.ts -o registry.json

# Validate before output
vmcp-compile tools.ts --validate -o registry.json
```

Your TypeScript file should export `registry`, `tools`, or a default:

```typescript
// tools.ts
import { tool, pipeline, createRegistry } from '@vmcp/dsl';

export const registry = createRegistry()
  .add(tool('my_tool').source('backend', 'tool').build())
  .build();

// OR
export const tools = [
  tool('my_tool').source('backend', 'tool').build(),
];

// OR
export default [
  tool('my_tool').source('backend', 'tool').build(),
];
```

## Type Safety with Path Builders

For type-safe JSONPath expressions:

```typescript
import { createPathBuilder, getPath } from '@vmcp/dsl';

interface SearchResult {
  data: {
    items: { id: string; score: number; title: string }[];
  };
}

const $ = createPathBuilder<SearchResult>();

// TypeScript validates the path
const scorePath = getPath($.data.items);  // "$.data.items"
```

## License

Apache-2.0

