/**
 * Example: Tool definitions using the @vmcp/dsl TypeScript DSL
 * 
 * Run with:
 *   npx tsx examples/test-tools.ts
 * 
 * Or compile to JSON:
 *   node dist/bin/vmcp-compile.js examples/test-tools.ts -o registry.json
 */
import { 
  tool, 
  pipeline, 
  scatterGather, 
  agg, 
  filter, 
  schemaMap, 
  mapEach,
  createRegistry 
} from '../dist/index.js';

// =============================================================================
// Virtual Tool (1:1 mapping to backend)
// =============================================================================

const weatherTool = tool('get_weather')
  .description('Get weather information')
  .source('weather-backend', 'fetch_weather')
  .default('units', 'metric')
  .default('api_key', '${WEATHER_API_KEY}')  // Environment variable substitution
  .build();

// =============================================================================
// Scatter-Gather Composition (parallel execution + aggregation)
// =============================================================================

const multiSearch = tool('multi_search')
  .description('Search multiple sources in parallel')
  .composition(
    scatterGather()
      .targets('search_web', 'search_arxiv', 'search_wikipedia')
      .aggregate(
        agg()
          .flatten()           // Combine all results into single array
          .sortDesc('$.score') // Sort by score descending
          .dedupe('$.id')      // Remove duplicates by ID
          .limit(20)           // Keep top 20
      )
      .timeout(5000)           // 5 second timeout
      .build()
  )
  .build();

// =============================================================================
// Pipeline Composition (sequential steps with data flow)
// =============================================================================

const researchPipeline = tool('research_pipeline')
  .description('End-to-end research pipeline: search → filter → normalize')
  .composition(
    pipeline()
      // Step 1: Call multi_search
      .step('search', 'multi_search')
      
      // Step 2: Filter results with relevance > 0.5
      .addStep({
        id: 'filter',
        operation: { 
          pattern: filter()
            .field('$.relevance')
            .gt(0.5)
            .build() 
        },
      })
      
      // Step 3: Normalize each result to a common schema
      .addStep({
        id: 'normalize',
        operation: {
          pattern: mapEach()
            .pattern(
              schemaMap()
                .field('title', '$.name')
                .coalesce('url', ['$.pdf_url', '$.web_url'])
                .literal('source', 'research')
                .template('citation', '${author} (${year})', {
                  author: '$.metadata.author',
                  year: '$.metadata.year'
                })
                .build()
            )
            .build(),
        },
      })
      .build()
  )
  .build();

// =============================================================================
// Build and Validate Registry
// =============================================================================

export const registry = createRegistry()
  .add(weatherTool)
  .add(multiSearch)
  .add(researchPipeline)
  .build();

// Validate
const builder = createRegistry()
  .add(weatherTool)
  .add(multiSearch)
  .add(researchPipeline);

const result = builder.validate();
console.log('Valid:', result.valid);
if (result.errors.length > 0) {
  console.log('Errors:', result.errors);
}
if (result.warnings.length > 0) {
  console.log('Warnings:', result.warnings);
}

// Output JSON
console.log('\n--- Registry JSON ---\n');
console.log(JSON.stringify(registry, null, 2));

