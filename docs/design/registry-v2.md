# Registry v2: Full Implementation Design

## Overview

Registry v2 extends the existing tool registry IR to support:

1. **Schema Definitions** - Named, reusable JSON Schema definitions that can be referenced by `$ref`
2. **Server Definitions** - MCP server definitions with versioning for version-aware routing
3. **Agent Definitions** - Agent definitions for A2A routing and agent-as-tool execution

This document describes the proto schema changes and their intended usage.

## Motivation

### Current Limitations (v1)

The v1 registry only supports tool definitions with basic source mappings:

```protobuf
message Registry {
  string schema_version = 1;
  repeated ToolDefinition tools = 2;
}
```

This creates several limitations:

1. **No schema reuse**: Input/output schemas are defined inline, leading to duplication
2. **No server versioning**: Tools reference servers by name only, no version control
3. **No agent support**: Cannot register agents for A2A routing or agent-as-tool execution
4. **No dependency tracking**: Cannot declare what tools/agents a component depends on

### v2 Capabilities

Registry v2 addresses these limitations:

```protobuf
message Registry {
  string schema_version = 1;           // "2.0"
  repeated ToolDefinition tools = 2;   // Existing
  repeated SchemaDefinition schemas = 3;  // NEW: Reusable schemas
  repeated ServerDefinition servers = 4;  // NEW: Versioned servers
  repeated AgentDefinition agents = 5;    // NEW: A2A agents
}
```

---

## Schema Definitions

### Purpose

Named JSON Schema definitions that can be referenced by tools, agents, and other schemas using JSON Schema `$ref` syntax.

### Proto Definition

```protobuf
message SchemaDefinition {
  string name = 1;                    // Unique schema name
  optional string description = 2;
  google.protobuf.Struct schema = 3;  // JSON Schema definition
  optional string version = 4;        // Semantic version
  map<string, google.protobuf.Value> metadata = 5;
}
```

### Usage Example

```json
{
  "schemaVersion": "2.0",
  "schemas": [
    {
      "name": "WeatherInput",
      "description": "Input schema for weather queries",
      "version": "1.0.0",
      "schema": {
        "type": "object",
        "properties": {
          "city": { "type": "string", "description": "City name" },
          "units": { "type": "string", "enum": ["metric", "imperial"] }
        },
        "required": ["city"]
      }
    },
    {
      "name": "WeatherOutput",
      "description": "Weather response format",
      "version": "1.0.0",
      "schema": {
        "type": "object",
        "properties": {
          "temperature": { "type": "number" },
          "conditions": { "type": "string" },
          "humidity": { "type": "number" }
        }
      }
    }
  ],
  "tools": [
    {
      "name": "get_weather",
      "source": {
        "server": "weather-service",
        "tool": "fetch_weather",
        "serverVersion": ">=1.0.0"
      },
      "inputSchema": { "$ref": "#/schemas/WeatherInput" },
      "outputTransform": {
        "mappings": {
          "temperature": { "path": "$.data.temp" },
          "conditions": { "path": "$.data.weather" }
        }
      }
    }
  ]
}
```

### Schema Resolution

Schemas are resolved using JSON Pointer syntax:

- `#/schemas/WeatherInput` - Reference to named schema
- `#/schemas/WeatherInput/properties/city` - Nested reference

The `SchemaResolver` (WP2) handles:
- Circular reference detection
- Missing schema errors
- Version compatibility checking

---

## Server Definitions

### Purpose

Declare MCP servers with versioning to enable:

1. **Version-aware routing**: Route `server:version` keys to correct backend
2. **Capability validation**: Verify server supports required features
3. **Tool validation**: Ensure registry tools match server's actual tools

### Proto Definition

```protobuf
message ServerDefinition {
  string name = 1;              // Server name (e.g., "doc-service")
  string version = 2;           // Server version (e.g., "1.2.0")
  optional string description = 3;
  ServerCapabilities capabilities = 4;
  repeated ServerTool provided_tools = 5;
  map<string, google.protobuf.Value> metadata = 6;
}

message ServerCapabilities {
  bool streamable_http = 1;
  bool stdio = 2;
  bool sse = 3;
  bool tools = 4;
  bool prompts = 5;
  bool resources = 6;
  bool sampling = 7;
}

message ServerTool {
  string name = 1;
  optional string input_schema_ref = 2;
  optional string output_schema_ref = 3;
}
```

### Usage Example

```json
{
  "schemaVersion": "2.0",
  "servers": [
    {
      "name": "doc-service",
      "version": "1.2.0",
      "description": "Document processing service",
      "capabilities": {
        "streamableHttp": true,
        "tools": true,
        "prompts": false,
        "resources": true
      },
      "providedTools": [
        {
          "name": "search_documents",
          "inputSchemaRef": "#/schemas/SearchInput",
          "outputSchemaRef": "#/schemas/SearchResults"
        },
        {
          "name": "get_document",
          "inputSchemaRef": "#/schemas/DocIdInput"
        }
      ],
      "metadata": {
        "owner": "docs-team",
        "healthEndpoint": "/health"
      }
    }
  ]
}
```

### Routing Key Construction

The `server:version` key is constructed from:

1. `SourceTool.server` + `SourceTool.server_version` for tool sources
2. `ToolCall.server` + `ToolCall.server_version` for direct calls

**YAML target matching** (WP16):
```yaml
backends:
  - mcp:
      targets:
        - name: doc-service:1.2.0  # Embeds version in name
          mcp:
            host: docs.internal
```

The registry `source.server` + `source.serverVersion` constructs the lookup key.

### Version Fallback (v1 Compatibility)

If `server_version` is not specified:
1. First, try exact match on server name
2. Fall back to "latest" version of that server
3. Log warning for version-unaware calls

---

## Agent Definitions

### Purpose

Register agents for:

1. **A2A Routing** (WP12): Route A2A requests to multiple registered agents
2. **Agent-as-Tool** (WP13): Execute agents as steps in tool compositions
3. **Dependency-Scoped Discovery** (WP11): Filter tools based on caller's declared dependencies

### Proto Definition

```protobuf
message AgentDefinition {
  string name = 1;
  string version = 2;
  optional string description = 3;
  AgentEndpoint endpoint = 4;
  repeated AgentSkill skills = 5;
  repeated AgentDependency dependencies = 6;
  map<string, google.protobuf.Value> metadata = 7;
}

message AgentEndpoint {
  oneof transport {
    A2AEndpoint a2a = 1;
    MCPEndpoint mcp = 2;
  }
}

message AgentSkill {
  string name = 1;
  optional string description = 2;
  optional SchemaRef input_schema = 3;
  optional SchemaRef output_schema = 4;
  repeated SkillExample examples = 5;
}

message AgentDependency {
  oneof dependency {
    string tool = 1;
    string agent = 2;
    ServerDependency server = 3;
  }
}
```

### Usage Example

```json
{
  "schemaVersion": "2.0",
  "agents": [
    {
      "name": "research-agent",
      "version": "1.0.0",
      "description": "Agent that performs research tasks",
      "endpoint": {
        "a2a": {
          "url": "https://agents.internal/research",
          "auth": {
            "bearer": { "token": "${RESEARCH_AGENT_TOKEN}" }
          }
        }
      },
      "skills": [
        {
          "name": "research",
          "description": "Research a topic and provide summary",
          "inputSchema": { "$ref": "#/schemas/ResearchInput" },
          "outputSchema": { "$ref": "#/schemas/ResearchOutput" },
          "examples": [
            {
              "input": { "topic": "quantum computing", "depth": "overview" },
              "description": "Quick overview research"
            }
          ]
        }
      ],
      "dependencies": [
        { "tool": "web_search" },
        { "tool": "arxiv_search" },
        { "server": { "name": "citation-service", "versionConstraint": ">=1.0.0" } }
      ],
      "metadata": {
        "owner": "research-team",
        "costTier": "high"
      }
    }
  ]
}
```

### Agent-as-Tool Execution

Agents can be invoked as steps in compositions:

```json
{
  "name": "comprehensive_research",
  "spec": {
    "pipeline": {
      "steps": [
        {
          "id": "gather",
          "operation": {
            "agent": {
              "name": "research-agent",
              "skill": "research",
              "version": ">=1.0.0"
            }
          },
          "input": { "input": { "path": "$" } }
        },
        {
          "id": "summarize",
          "operation": { "tool": { "name": "summarize" } },
          "input": { "step": { "stepId": "gather", "path": "$.documents" } }
        }
      ]
    }
  }
}
```

### Dependency-Scoped Discovery

When an agent calls `tools/list`, the gateway filters results based on declared dependencies:

1. Extract caller identity (WP10)
2. Look up agent's declared dependencies
3. Return only tools/servers the agent depends on
4. Log access attempts to undeclared dependencies

---

## Updated Message Types

### SourceTool (Updated)

```protobuf
message SourceTool {
  string server = 1;                    // Renamed from 'target' for clarity
  string tool = 2;
  map<string, google.protobuf.Value> defaults = 3;
  repeated string hide_fields = 4;
  optional string server_version = 5;   // NEW: Version constraint
}
```

### StepOperation (Updated)

```protobuf
message StepOperation {
  oneof op {
    ToolCall tool = 1;
    PatternSpec pattern = 2;
    AgentCall agent = 3;  // NEW: Agent invocation
  }
}

message AgentCall {
  string name = 1;
  optional string skill = 2;
  optional string version = 3;
}
```

### ToolCall (Updated)

```protobuf
message ToolCall {
  string name = 1;
  optional string server = 2;           // NEW: Direct server override
  optional string server_version = 3;   // NEW: Version constraint
}
```

---

## Migration Guide

### From v1 to v2

1. **Schema version**: Update `schema_version` from `"1.0"` to `"2.0"`

2. **SourceTool.target → SourceTool.server**: Rename field (protobuf field number unchanged)

3. **Add server definitions** (optional): If using version-aware routing, add `servers` array

4. **Add schema definitions** (optional): Extract common schemas for reuse

5. **Add agent definitions** (optional): Register agents for A2A

### Backward Compatibility

- v1 registries work unchanged (empty schemas/servers/agents arrays)
- v1 tool definitions work unchanged (no server_version means "latest")
- Runtime falls back gracefully when v2 features aren't used

---

## Implementation Work Packages

| WP | Title | Description | Dependencies |
|----|-------|-------------|--------------|
| WP1 | Proto Schema Update | This document - update registry.proto | None |
| WP2 | Rust Types and Parsing | Deserialize v2 JSON, SchemaResolver | WP1 |
| WP3 | Rust Validation | Dependency resolution, cycle detection | WP2 |
| WP10 | Caller Identity | Extract caller from requests | WP2 |
| WP11 | Dependency-Scoped Discovery | Filter tools/list by caller deps | WP10, WP3 |
| WP12 | Agent Multiplexing | Route A2A to multiple agents | WP2 |
| WP13 | Agent-as-Tool Executor | Execute agents in compositions | WP12 |
| WP16 | Version-Aware Server Routing | server:version dispatch | WP2 |

---

## References

- [registry-integration.md](./registry-integration.md) - Original registry design
- [ARCHITECTURE_DISCONNECTS.md](../ARCHITECTURE_DISCONNECTS.md) - Gap analysis
- [DESIGN_TS_TO_RUNTIME.md](../DESIGN_TS_TO_RUNTIME.md) - TypeScript DSL design
- [mcp-algebra-ala-camel.md](../mcp-algebra-ala-camel.md) - Pattern catalog
