# Pattern Demos Gateway Configs

This directory contains agentgateway configurations showcasing all composition patterns
available for building virtual tools from MCP backends.

## Quick Start

```bash
./start.sh
```

Or with custom settings:

```bash
./start.sh --port 8080 --dev
```

## Files

- **`config.yaml`** - Gateway configuration with MCP backend targets and registry loading
- **`patterns_registry.json`** - Virtual tool registry defining all pattern compositions
- **`start.sh`** - Startup script with configuration options

## Patterns Demonstrated

### 1. Pipeline: `search_and_summarize`

Sequential execution pattern that chains operations together, passing output from one step to the next.

```
Input → document_search → combine_results → llm_summarize → Output
```

**Use case**: Search documents and generate an AI summary of the results.

### 2. Saga: `create_project`

Distributed transaction pattern with automatic compensation (rollback) on failure.

```
create_task → assign_user → send_notification
     ↓             ↓
delete_task   unassign_user  (compensation on failure)
```

**Use case**: Create a project task, assign it to a user, and notify them - with full rollback if any step fails.

### 3. ScatterGather: `multi_search`

Parallel fan-out pattern that executes multiple operations concurrently and aggregates results.

```
         ┌─→ search_users ──┐
Input ───┼─→ search_docs ───┼─→ aggregate → Output
         └─→ search_tasks ──┘
```

**Use case**: Search across multiple data sources simultaneously and merge results.

### 4. Cache: `cached_user_lookup`

Read-through caching pattern that stores results for fast subsequent lookups.

```
Request → Check Cache → [Hit] → Return cached
                    ↓
              [Miss] → get_user → Store in cache → Return
```

**Use case**: Cache user profile lookups with 15-minute TTL and stale-while-revalidate.

### 5. Retry: `reliable_notification`

Automatic retry pattern with configurable backoff for transient failures.

```
send_notification → [Success] → Done
        ↓
    [Failure] → Wait (exponential backoff) → Retry (up to 5 times)
```

**Use case**: Ensure notification delivery even when the notification service has intermittent failures.

### 6. CircuitBreaker: `protected_external_call`

Fail-fast pattern that prevents cascading failures from unreliable external services.

```
                    ┌─→ [Circuit Open] → Return fallback
Request → Circuit ──┤
                    └─→ [Circuit Closed] → external_api_call → Track result
```

**Use case**: Protect against slow or failing external APIs by failing fast after threshold failures.

### 7. Timeout: `time_bounded_search`

Deadline enforcement pattern that ensures operations complete within a time limit.

```
document_search ─┬─→ [Complete < 5s] → Return results
                 └─→ [Timeout] → Return fallback response
```

**Use case**: Ensure search operations don't block indefinitely.

## Pattern Spec Reference

### Pipeline

```json
{
  "pipeline": {
    "steps": [
      {
        "id": "step1",
        "operation": {"tool": {"name": "tool_name"}},
        "input": {"input": {"path": "$"}}
      },
      {
        "id": "step2",
        "operation": {"tool": {"name": "another_tool"}},
        "input": {"step": {"stepId": "step1", "path": "$.result"}}
      }
    ]
  }
}
```

### Saga

```json
{
  "saga": {
    "steps": [
      {
        "id": "step1",
        "name": "Human readable name",
        "action": {"tool": {"name": "do_action"}},
        "compensate": {"tool": {"name": "undo_action"}},
        "input": {"input": {"path": "$"}}
      }
    ],
    "sagaIdPath": "$.transactionId",
    "timeoutMs": 30000
  }
}
```

### ScatterGather

```json
{
  "scatterGather": {
    "targets": [
      {"tool": "search_a"},
      {"tool": "search_b"}
    ],
    "aggregation": {
      "ops": [
        {"flatten": true},
        {"sort": {"field": "$.score", "order": "desc"}},
        {"dedupe": {"field": "$.id"}},
        {"limit": {"count": 10}}
      ]
    },
    "timeoutMs": 5000,
    "failFast": false
  }
}
```

### Cache

```json
{
  "cache": {
    "keyPaths": ["$.userId"],
    "inner": {"tool": {"name": "get_user"}},
    "store": "cache_store_name",
    "ttlSeconds": 900,
    "staleWhileRevalidateSeconds": 60
  }
}
```

### Retry

```json
{
  "retry": {
    "inner": {"tool": {"name": "flaky_service"}},
    "maxAttempts": 5,
    "backoff": {
      "exponential": {
        "initialDelayMs": 500,
        "maxDelayMs": 30000,
        "multiplier": 2.0
      }
    },
    "jitter": 0.2,
    "retryIf": {
      "field": "$.error.code",
      "op": "in",
      "value": {"listValue": [{"stringValue": "TIMEOUT"}]}
    }
  }
}
```

### CircuitBreaker

```json
{
  "circuitBreaker": {
    "name": "circuit_name",
    "inner": {"tool": {"name": "external_api"}},
    "store": "circuit_state_store",
    "failureThreshold": 5,
    "failureWindowSeconds": 60,
    "resetTimeoutSeconds": 30,
    "successThreshold": 2,
    "fallback": {
      "schemaMap": {
        "mappings": {
          "error": {"literal": {"stringValue": "Service unavailable"}}
        }
      }
    }
  }
}
```

### Timeout

```json
{
  "timeout": {
    "inner": {"tool": {"name": "slow_operation"}},
    "durationMs": 5000,
    "fallback": {
      "schemaMap": {
        "mappings": {
          "timedOut": {"literal": {"boolValue": true}}
        }
      }
    },
    "message": "Operation timed out"
  }
}
```

## Data Binding Reference

Patterns use data bindings to wire inputs and outputs:

- **`input`**: Bind from original input using JSONPath
  ```json
  {"input": {"path": "$.query"}}
  ```

- **`step`**: Bind from previous step's output
  ```json
  {"step": {"stepId": "search", "path": "$.results[0]"}}
  ```

- **`constant`**: Bind a literal value
  ```json
  {"constant": {"stringValue": "default"}}
  ```

- **`construct`**: Build an object from multiple sources
  ```json
  {
    "construct": {
      "fields": {
        "query": {"input": {"path": "$.q"}},
        "limit": {"constant": {"numberValue": 10}}
      }
    }
  }
  ```

## Testing with Mock Servers

The configuration expects mock MCP servers. For basic testing, modify `config.yaml` to use
the `@modelcontextprotocol/server-everything` server instead.

## Note on Stateful Patterns

Cache, CircuitBreaker, and other stateful patterns require external state stores
which are defined in IR but not yet implemented in the runtime. These patterns
serve as documentation for the intended behavior.
