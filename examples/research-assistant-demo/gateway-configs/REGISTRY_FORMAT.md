# Registry JSON Format Reference

## Tool Definition Patterns

### Source-based Virtual Tool
```json
{
  "name": "normalized_search",
  "version": "1.0.0",
  "description": "Search with normalized output",
  "source": {
    "server": "search-service",
    "tool": "raw_search"
  },
  "inputSchema": { ... },
  "outputSchema": { ... },
  "outputTransform": {
    "mappings": [
      { "field": "results", "source": { "path": "$.items" } }
    ]
  }
}
```

### Scatter-Gather Composition
```json
{
  "name": "multi_search",
  "version": "1.0.0",
  "description": "Search multiple sources in parallel",
  "spec": {
    "scatterGather": {
      "targets": [
        {"tool": "exa_search", "server": "search-service"},
        {"tool": "arxiv_search", "server": "search-service"}
      ],
      "aggregation": {
        "ops": [{"merge": true}]
      },
      "timeoutMs": 30000,
      "failFast": false
    }
  },
  "inputSchema": { ... }
}
```

### Pipeline Composition
```json
{
  "name": "fetch_and_process",
  "version": "1.0.0",
  "description": "Fetch URL then process content",
  "spec": {
    "pipeline": {
      "steps": [
        {
          "id": "fetch",
          "operation": { "tool": { "name": "url_fetch", "server": "fetch-service" } },
          "input": { "input": { "path": "$" } }
        },
        {
          "id": "process",
          "operation": { "tool": { "name": "extract_text", "server": "fetch-service" } },
          "input": {
            "construct": {
              "fields": {
                "content": { "step": { "stepId": "fetch", "path": "$.body" } }
              }
            }
          }
        }
      ]
    }
  },
  "inputSchema": { ... }
}
```

## Data Binding Types

### Input Binding (from original input)
```json
{ "input": { "path": "$.query" } }
```

### Step Binding (from previous step output)
```json
{ "step": { "stepId": "fetch", "path": "$.content" } }
```

### Constant Value
```json
{ "constant": "fixed_value" }
```

### Construct (build object from multiple bindings)
```json
{
  "construct": {
    "fields": {
      "name": { "input": { "path": "$.name" } },
      "type": { "constant": "entity" },
      "data": { "step": { "stepId": "prev", "path": "$.result" } }
    }
  }
}
```

## Output Transform (arrayMap)
```json
{
  "outputTransform": {
    "mappings": [
      {
        "field": "results",
        "source": {
          "arrayMap": {
            "sourcePath": "$.items",
            "itemMappings": [
              { "field": "title", "source": { "path": "$.name" } },
              { "field": "url", "source": { "path": "$.link" } },
              { "field": "source", "source": { "literal": { "stringValue": "exa" } } }
            ]
          }
        }
      }
    ]
  }
}
```
