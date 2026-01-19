# Registry v2 Work Packages

**Parent Doc**: [registry-v2.md](./registry-v2.md)  
**Example Registry**: [registry-v2-example.json](../../examples/pattern-demos/configs/registry-v2-example.json)

This document breaks down the Registry v2 implementation into parallelizable work packages with detailed specifications for TDD development.

---

## Implementation Phases

### Phase 1 (MVP): Agent-to-MCP-Tool
Focus on enabling agents to access versioned MCP tools with dependency tracking.

**Included Work Packages:**
- WP1: Proto Schema Update
- WP2: Rust Types and Parsing
- WP3: Rust Validation
- WP4: Runtime Hooks
- WP5: TypeScript DSL Types
- WP6: TypeScript DSL Validation
- WP7: SBOM Export
- WP8: Test Suite
- WP9: Documentation
- WP10: Caller Identity
- WP11: Dependency-Scoped Discovery
- WP14: Discovery Endpoints (MCP only)
- WP16: Version-Aware Server Routing

### Phase 2 (Future): Agent-as-Tool
Extend the gateway to invoke A2A agents within compositions.

**Deferred Work Packages:**
- WP12: Agent Multiplexing (A2A)
- WP13: Agent-as-Tool Executor
- WP15: Agent Discovery Endpoints

---

## Work Package Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           WP1: Proto Schema                                  │
│                     (No dependencies - START HERE)                           │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
┌───────────────────────────────────┐ ┌───────────────────────────────────────┐
│       WP2: Rust Types             │ │       WP5: TypeScript Types           │
│       (Depends: WP1)              │ │       (Depends: WP1)                  │
└───────────────────────────────────┘ └───────────────────────────────────────┘
                    │                               │
          ┌─────────┴─────────┐                     │
          ▼                   ▼                     ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────────────────┐
│ WP3: Rust       │ │ WP7: SBOM       │ │       WP6: TypeScript Validation    │
│ Validation      │ │ Export          │ │       (Depends: WP5)                │
│ (Depends: WP2)  │ │ (Depends: WP2)  │ └─────────────────────────────────────┘
└─────────────────┘ └─────────────────┘
          │
          ▼
┌─────────────────┐
│ WP4: Runtime    │
│ Hooks           │
│ (Depends: WP3)  │
└─────────────────┘

WP8 (Tests) can start immediately with stubs, runs against WP2-WP6
WP9 (Docs) runs last
```

---

## WP1: Proto Schema Update

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 2-3 days  
**Dependencies**: None

### Objective

Update `registry.proto` to define the v2 registry IR with Schema, Server, and Agent messages.

### Files to Modify

- `crates/agentgateway/proto/registry.proto`

### Specification

Add the following message types:

```protobuf
// Schema definition with versioning
message SchemaDefinition {
  string name = 1;
  string version = 2;
  optional string description = 3;
  google.protobuf.Struct schema = 4;  // JSON Schema
  map<string, google.protobuf.Value> metadata = 5;
}

// Server (MCP backend) registration
message ServerDefinition {
  string name = 1;
  string version = 2;
  optional string description = 3;
  repeated ToolProvision provides = 4;
  bool deprecated = 5;
  optional string deprecation_message = 6;
  map<string, google.protobuf.Value> metadata = 7;
}

message ToolProvision {
  string tool = 1;
  string version = 2;
}

// Agent registration (extends A2A AgentCard)
message AgentDefinition {
  string name = 1;
  string version = 2;
  string description = 3;
  string url = 4;
  string protocol_version = 5;
  repeated string default_input_modes = 6;
  repeated string default_output_modes = 7;
  repeated AgentSkillDefinition skills = 8;
  AgentCapabilitiesDefinition capabilities = 9;
  optional AgentProvider provider = 10;
}

message AgentSkillDefinition {
  string id = 1;
  string name = 2;
  string description = 3;
  repeated string tags = 4;
  repeated string examples = 5;
  repeated string input_modes = 6;
  repeated string output_modes = 7;
  optional google.protobuf.Struct input_schema = 8;
  optional google.protobuf.Struct output_schema = 9;
}

message AgentCapabilitiesDefinition {
  optional bool streaming = 1;
  optional bool push_notifications = 2;
  optional bool state_transition_history = 3;
  repeated AgentExtensionDefinition extensions = 4;
}

message AgentExtensionDefinition {
  string uri = 1;
  optional string description = 2;
  optional bool required = 3;
  google.protobuf.Struct params = 4;
}

// Typed dependency reference
message Dependency {
  DependencyType type = 1;
  string name = 2;
  string version = 3;
  optional string skill = 4;  // For agent dependencies
}

enum DependencyType {
  DEPENDENCY_TYPE_UNSPECIFIED = 0;
  DEPENDENCY_TYPE_TOOL = 1;
  DEPENDENCY_TYPE_AGENT = 2;
}

// Updated Registry root
message Registry {
  string schema_version = 1;
  repeated SchemaDefinition schemas = 2;
  repeated ServerDefinition servers = 3;
  repeated ToolDefinition tools = 4;
  repeated AgentDefinition agents = 5;
}

// Updated ToolDefinition with version and serverVersion
message ToolDefinition {
  string name = 1;
  string version = 2;  // NEW
  optional string description = 3;
  
  oneof implementation {
    SourceTool source = 4;
    PatternSpec spec = 5;
  }
  
  repeated Dependency depends = 6;  // NEW
  
  // Schema can be inline or reference
  oneof input_schema_type {
    google.protobuf.Struct input_schema_inline = 7;
    string input_schema_ref = 8;  // Format: "#SchemaName:Version"
  }
  
  oneof output_schema_type {
    google.protobuf.Struct output_schema_inline = 9;
    string output_schema_ref = 10;
  }
  
  optional OutputTransform output_transform = 11;
  map<string, google.protobuf.Value> metadata = 12;
}

// Updated SourceTool with server version
message SourceTool {
  string server = 1;         // Server name
  string server_version = 2; // Server version
  string tool = 3;           // Original tool name
  map<string, google.protobuf.Value> defaults = 4;
  repeated string hide_fields = 5;
}
```

### Acceptance Criteria

- [ ] Proto compiles without errors
- [ ] Generated Rust bindings compile
- [ ] Backward compatible: v1 registries still parse (version field optional)

---

## WP2: Rust Types and Parsing

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 3-4 days  
**Dependencies**: WP1

### Objective

Implement Rust types that deserialize the v2 registry JSON format.

### Files to Create/Modify

- `crates/agentgateway/src/mcp/registry/types.rs` (modify)
- `crates/agentgateway/src/mcp/registry/schema_resolver.rs` (new)
- `crates/agentgateway/src/mcp/registry/v2_types.rs` (new, if cleaner)

### Type Definitions

```rust
/// Schema definition
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDefinition {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub schema: serde_json::Value,  // JSON Schema
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Server (MCP backend) registration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDefinition {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub provides: Vec<ToolProvision>,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub deprecation_message: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolProvision {
    pub tool: String,
    pub version: String,
}

/// Dependency reference
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dependency {
    #[serde(rename = "type")]
    pub dep_type: DependencyType,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub skill: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Tool,
    Agent,
}

/// Updated Registry with v2 fields
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegistryV2 {
    #[serde(default = "default_schema_version_v2")]
    pub schema_version: String,
    #[serde(default)]
    pub schemas: Vec<SchemaDefinition>,
    #[serde(default)]
    pub servers: Vec<ServerDefinition>,
    #[serde(default)]
    pub tools: Vec<ToolDefinitionV2>,
    #[serde(default)]
    pub agents: Vec<AgentDefinition>,
}

/// Schema reference resolver
pub struct SchemaResolver {
    schemas: HashMap<(String, String), serde_json::Value>,
}

impl SchemaResolver {
    pub fn new(schemas: &[SchemaDefinition]) -> Self { ... }
    
    pub fn resolve(&self, ref_str: &str) -> Result<&serde_json::Value, SchemaError> {
        // Parse "#SchemaName:Version" format
        ...
    }
}
```

### Test Stubs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_v2_registry_with_schemas() {
        let json = include_str!("../../../../examples/pattern-demos/configs/registry-v2-example.json");
        let registry: RegistryV2 = serde_json::from_str(json).expect("should parse");
        
        assert_eq!(registry.schema_version, "2.0");
        assert!(!registry.schemas.is_empty());
        assert!(!registry.servers.is_empty());
        assert!(!registry.tools.is_empty());
        assert!(!registry.agents.is_empty());
    }

    #[test]
    fn test_schema_resolution() {
        let schemas = vec![
            SchemaDefinition {
                name: "SearchQuery".into(),
                version: "1.0.0".into(),
                schema: serde_json::json!({"type": "object"}),
                ..Default::default()
            }
        ];
        let resolver = SchemaResolver::new(&schemas);
        
        let result = resolver.resolve("#SearchQuery:1.0.0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_ref_not_found() {
        let resolver = SchemaResolver::new(&[]);
        let result = resolver.resolve("#NonExistent:1.0.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tool_with_source() {
        let json = r#"{
            "name": "search",
            "version": "1.0.0",
            "source": {
                "server": "doc-service",
                "serverVersion": "1.2.0",
                "tool": "search_documents"
            }
        }"#;
        let tool: ToolDefinitionV2 = serde_json::from_str(json).expect("should parse");
        assert_eq!(tool.version, Some("1.0.0".into()));
    }

    #[test]
    fn test_parse_tool_with_dependencies() {
        let json = r#"{
            "name": "pipeline",
            "version": "1.0.0",
            "depends": [
                {"type": "tool", "name": "fetch", "version": "1.2.3"},
                {"type": "agent", "name": "summarizer", "version": "2.0.0", "skill": "summarize"}
            ],
            "spec": {"pipeline": {"steps": []}}
        }"#;
        let tool: ToolDefinitionV2 = serde_json::from_str(json).expect("should parse");
        assert_eq!(tool.depends.len(), 2);
    }

    #[test]
    fn test_backward_compatibility_v1() {
        // v1 registry should still parse (version fields optional)
        let json = r#"{
            "schemaVersion": "1.0",
            "tools": [
                {"name": "search", "source": {"target": "backend", "tool": "search"}}
            ]
        }"#;
        let registry: RegistryV2 = serde_json::from_str(json).expect("v1 should parse");
        assert_eq!(registry.tools.len(), 1);
    }
}
```

### Acceptance Criteria

- [ ] All types deserialize from JSON correctly
- [ ] Schema resolver handles `$ref` format
- [ ] v1 registries still parse (backward compatible)
- [ ] All test stubs pass

---

## WP3: Rust Validation

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 4-5 days  
**Dependencies**: WP2

### Objective

Implement startup validation: dependency resolution, cycle detection, schema validation.

### Files to Create

- `crates/agentgateway/src/mcp/registry/validate.rs`
- `crates/agentgateway/src/mcp/registry/dependency_graph.rs`

### Core Types

```rust
/// Validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub missing_entity: Severity,
    pub deprecated_entity: Severity,
    pub unused_schema: Severity,
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Error,
    Warn,
    Ignore,
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    pub fn is_ok(&self) -> bool { self.errors.is_empty() }
}

#[derive(Debug)]
pub enum ValidationError {
    SchemaNotFound { ref_str: String, used_by: String },
    ServerNotFound { server: String, version: String, used_by: String },
    ToolNotFound { tool: String, version: String, used_by: String },
    AgentNotFound { agent: String, version: String, skill: Option<String>, used_by: String },
    CircularDependency { cycle: Vec<String> },
    ServerDoesNotProvideTool { server: String, tool: String, version: String },
}

#[derive(Debug)]
pub enum ValidationWarning {
    DeprecatedServer { server: String, version: String, used_by: String },
    DeprecatedTool { tool: String, version: String, used_by: String },
    UnusedSchema { schema: String, version: String },
}

/// Dependency graph for cycle detection
pub struct DependencyGraph {
    // node_id -> Vec<dependency_node_ids>
    edges: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    pub fn new() -> Self { ... }
    pub fn add_node(&mut self, id: &str) { ... }
    pub fn add_edge(&mut self, from: &str, to: &str) { ... }
    pub fn find_cycles(&self) -> Vec<Vec<String>> { ... }
}

/// Main validation function
pub fn validate_registry(
    registry: &RegistryV2,
    config: &ValidationConfig,
) -> ValidationResult { ... }
```

### Test Stubs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_registry() {
        let registry = load_v2_example();
        let config = ValidationConfig::strict();
        let result = validate_registry(&registry, &config);
        assert!(result.is_ok(), "example registry should be valid");
    }

    #[test]
    fn test_detect_missing_schema_ref() {
        let registry = RegistryV2 {
            tools: vec![ToolDefinitionV2 {
                name: "test".into(),
                version: Some("1.0.0".into()),
                input_schema: Some(SchemaOrRef::Ref("#NonExistent:1.0.0".into())),
                ..Default::default()
            }],
            ..Default::default()
        };
        let result = validate_registry(&registry, &ValidationConfig::strict());
        assert!(!result.is_ok());
        assert!(matches!(
            &result.errors[0],
            ValidationError::SchemaNotFound { .. }
        ));
    }

    #[test]
    fn test_detect_circular_dependency() {
        // Tool A depends on Tool B, Tool B depends on Tool A
        let registry = RegistryV2 {
            tools: vec![
                ToolDefinitionV2 {
                    name: "a".into(),
                    version: Some("1.0.0".into()),
                    depends: vec![Dependency {
                        dep_type: DependencyType::Tool,
                        name: "b".into(),
                        version: "1.0.0".into(),
                        skill: None,
                    }],
                    ..Default::default()
                },
                ToolDefinitionV2 {
                    name: "b".into(),
                    version: Some("1.0.0".into()),
                    depends: vec![Dependency {
                        dep_type: DependencyType::Tool,
                        name: "a".into(),
                        version: "1.0.0".into(),
                        skill: None,
                    }],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let result = validate_registry(&registry, &ValidationConfig::strict());
        assert!(matches!(
            &result.errors[0],
            ValidationError::CircularDependency { .. }
        ));
    }

    #[test]
    fn test_detect_server_does_not_provide_tool() {
        let registry = RegistryV2 {
            servers: vec![ServerDefinition {
                name: "my-server".into(),
                version: "1.0.0".into(),
                provides: vec![],  // Empty!
                ..Default::default()
            }],
            tools: vec![ToolDefinitionV2 {
                name: "search".into(),
                version: Some("1.0.0".into()),
                source: Some(SourceToolV2 {
                    server: "my-server".into(),
                    server_version: "1.0.0".into(),
                    tool: "search".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        let result = validate_registry(&registry, &ValidationConfig::strict());
        assert!(matches!(
            &result.errors[0],
            ValidationError::ServerDoesNotProvideTool { .. }
        ));
    }

    #[test]
    fn test_warn_deprecated_server() {
        let registry = RegistryV2 {
            servers: vec![ServerDefinition {
                name: "old-server".into(),
                version: "1.0.0".into(),
                deprecated: true,
                provides: vec![ToolProvision {
                    tool: "search".into(),
                    version: "1.0.0".into(),
                }],
                ..Default::default()
            }],
            tools: vec![ToolDefinitionV2 {
                name: "search".into(),
                version: Some("1.0.0".into()),
                source: Some(SourceToolV2 {
                    server: "old-server".into(),
                    server_version: "1.0.0".into(),
                    tool: "search".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        let config = ValidationConfig {
            deprecated_entity: Severity::Warn,
            ..ValidationConfig::strict()
        };
        let result = validate_registry(&registry, &config);
        assert!(result.is_ok());  // Only warnings
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_dependency_graph_cycle_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_node("a");
        graph.add_node("b");
        graph.add_node("c");
        graph.add_edge("a", "b");
        graph.add_edge("b", "c");
        graph.add_edge("c", "a");  // Cycle!
        
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
    }
}
```

### Acceptance Criteria

- [ ] Validates all schema `$ref` resolve
- [ ] Validates all tool/agent dependencies exist
- [ ] Detects circular dependencies
- [ ] Validates servers provide declared tools
- [ ] Warns on deprecated entities
- [ ] All test stubs pass

---

## WP4: Runtime Hooks

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 3-4 days  
**Dependencies**: WP3

### Objective

Add runtime validation hooks for input/output schema validation and caller tracking.

### Files to Create/Modify

- `crates/agentgateway/src/mcp/registry/runtime.rs` (new)
- `crates/agentgateway/src/mcp/router.rs` (modify)

### Core Types

```rust
/// Runtime validation configuration
#[derive(Debug, Clone)]
pub struct RuntimeValidationConfig {
    pub input_validation: Severity,
    pub output_validation: Severity,
    pub unknown_caller: CallerPolicy,
    pub undeclared_dependency: Severity,
}

#[derive(Debug, Clone)]
pub enum CallerPolicy {
    Allow,
    Warn,
    Deny,
}

/// Runtime validator
pub struct RuntimeValidator {
    config: RuntimeValidationConfig,
    registry: Arc<RegistryV2>,
    schema_resolver: Arc<SchemaResolver>,
}

impl RuntimeValidator {
    /// Validate tool call input
    pub fn validate_input(
        &self,
        tool_name: &str,
        tool_version: &str,
        input: &serde_json::Value,
    ) -> ValidationResult { ... }
    
    /// Validate tool call output
    pub fn validate_output(
        &self,
        tool_name: &str,
        tool_version: &str,
        output: &serde_json::Value,
    ) -> ValidationResult { ... }
    
    /// Check if caller agent is allowed to call tool
    pub fn check_caller_dependency(
        &self,
        caller_agent: Option<&AgentIdentity>,
        tool_name: &str,
        tool_version: &str,
    ) -> ValidationResult { ... }
}

/// Agent identity from request context
#[derive(Debug, Clone)]
pub struct AgentIdentity {
    pub name: String,
    pub version: String,
}
```

### Test Stubs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_input_against_schema() {
        let validator = create_test_validator();
        
        let valid_input = serde_json::json!({
            "query": "test search",
            "limit": 10
        });
        let result = validator.validate_input("search_documents", "1.0.0", &valid_input);
        assert!(result.is_ok());
        
        let invalid_input = serde_json::json!({
            "limit": 10  // Missing required "query"
        });
        let result = validator.validate_input("search_documents", "1.0.0", &invalid_input);
        assert!(!result.is_ok());
    }

    #[test]
    fn test_check_caller_has_dependency() {
        let validator = create_test_validator();
        
        // research-agent declares dependency on search_documents
        let caller = AgentIdentity {
            name: "research-agent".into(),
            version: "2.1.0".into(),
        };
        let result = validator.check_caller_dependency(
            Some(&caller),
            "search_documents",
            "1.0.0"
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_caller_undeclared_dependency() {
        let validator = create_test_validator_with_config(RuntimeValidationConfig {
            undeclared_dependency: Severity::Error,
            ..Default::default()
        });
        
        // summarizer-agent does NOT declare dependency on search_documents
        let caller = AgentIdentity {
            name: "summarizer-agent".into(),
            version: "2.0.0".into(),
        };
        let result = validator.check_caller_dependency(
            Some(&caller),
            "search_documents",
            "1.0.0"
        );
        assert!(!result.is_ok());
    }

    #[test]
    fn test_unknown_caller_policy() {
        // Test Allow
        let validator = create_test_validator_with_config(RuntimeValidationConfig {
            unknown_caller: CallerPolicy::Allow,
            ..Default::default()
        });
        let result = validator.check_caller_dependency(None, "search_documents", "1.0.0");
        assert!(result.is_ok());
        
        // Test Deny
        let validator = create_test_validator_with_config(RuntimeValidationConfig {
            unknown_caller: CallerPolicy::Deny,
            ..Default::default()
        });
        let result = validator.check_caller_dependency(None, "search_documents", "1.0.0");
        assert!(!result.is_ok());
    }
}
```

### Acceptance Criteria

- [ ] Input validation against JSON Schema works
- [ ] Output validation against JSON Schema works
- [ ] Caller dependency checking works
- [ ] Unknown caller policy configurable
- [ ] Integrates with MCP router

---

## WP5: TypeScript DSL Types

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 2-3 days  
**Dependencies**: WP1

### Objective

Define TypeScript types and a fluent builder API for registry construction.

### Files to Create

- `packages/registry-dsl/src/types.ts`
- `packages/registry-dsl/src/builder.ts`

### Type Definitions

```typescript
// types.ts

export interface RegistryV2 {
  schemaVersion: "2.0";
  schemas: SchemaDefinition[];
  servers: ServerDefinition[];
  tools: ToolDefinition[];
  agents: AgentDefinition[];
}

export interface SchemaDefinition {
  name: string;
  version: string;
  description?: string;
  schema: JSONSchema;
  metadata?: Record<string, unknown>;
}

export interface ServerDefinition {
  name: string;
  version: string;
  description?: string;
  provides: ToolProvision[];
  deprecated?: boolean;
  deprecationMessage?: string;
  metadata?: Record<string, unknown>;
}

export interface ToolProvision {
  tool: string;
  version: string;
}

export interface Dependency {
  type: "tool" | "agent";
  name: string;
  version: string;
  skill?: string;
}

export interface ToolDefinition {
  name: string;
  version: string;
  description?: string;
  source?: ToolSource;
  spec?: PatternSpec;
  depends?: Dependency[];
  inputSchema?: JSONSchema | SchemaRef;
  outputSchema?: JSONSchema | SchemaRef;
  outputTransform?: OutputTransform;
  metadata?: Record<string, unknown>;
}

export interface ToolSource {
  server: string;
  serverVersion: string;
  tool: string;
  defaults?: Record<string, unknown>;
  hideFields?: string[];
}

export interface SchemaRef {
  $ref: string;  // "#SchemaName:Version"
}

export interface AgentDefinition {
  name: string;
  version: string;
  description: string;
  url: string;
  protocolVersion: string;
  defaultInputModes: string[];
  defaultOutputModes: string[];
  skills: AgentSkill[];
  capabilities: AgentCapabilities;
  provider?: AgentProvider;
}

export interface AgentSkill {
  id: string;
  name: string;
  description: string;
  tags: string[];
  examples?: string[];
  inputModes: string[];
  outputModes: string[];
  inputSchema?: JSONSchema | SchemaRef;
  outputSchema?: JSONSchema | SchemaRef;
}

// ... more types
```

### Builder API

```typescript
// builder.ts

export class RegistryBuilder {
  private registry: RegistryV2;

  constructor() {
    this.registry = {
      schemaVersion: "2.0",
      schemas: [],
      servers: [],
      tools: [],
      agents: [],
    };
  }

  schema(def: SchemaDefinition): this {
    this.registry.schemas.push(def);
    return this;
  }

  server(def: ServerDefinition): this {
    this.registry.servers.push(def);
    return this;
  }

  tool(def: ToolDefinition): this {
    this.registry.tools.push(def);
    return this;
  }

  agent(def: AgentDefinition): this {
    this.registry.agents.push(def);
    return this;
  }

  build(): RegistryV2 {
    return this.registry;
  }
}

// Fluent helpers
export function schema(name: string, version: string): SchemaBuilder { ... }
export function server(name: string, version: string): ServerBuilder { ... }
export function tool(name: string, version: string): ToolBuilder { ... }
export function agent(name: string, version: string): AgentBuilder { ... }
```

### Test Stubs

```typescript
// builder.test.ts

describe("RegistryBuilder", () => {
  it("should build a valid registry", () => {
    const registry = new RegistryBuilder()
      .schema({
        name: "SearchQuery",
        version: "1.0.0",
        schema: { type: "object" },
      })
      .server({
        name: "doc-service",
        version: "1.0.0",
        provides: [{ tool: "search", version: "1.0.0" }],
      })
      .tool({
        name: "search",
        version: "1.0.0",
        source: {
          server: "doc-service",
          serverVersion: "1.0.0",
          tool: "search",
        },
      })
      .build();

    expect(registry.schemaVersion).toBe("2.0");
    expect(registry.schemas).toHaveLength(1);
    expect(registry.servers).toHaveLength(1);
    expect(registry.tools).toHaveLength(1);
  });

  it("should support schema refs", () => {
    const tool = new ToolBuilder("search", "1.0.0")
      .inputSchema({ $ref: "#SearchQuery:1.0.0" })
      .build();

    expect(tool.inputSchema).toEqual({ $ref: "#SearchQuery:1.0.0" });
  });

  it("should support dependencies", () => {
    const tool = new ToolBuilder("pipeline", "1.0.0")
      .dependsOnTool("fetch", "1.2.3")
      .dependsOnAgent("summarizer", "2.0.0", "summarize")
      .build();

    expect(tool.depends).toHaveLength(2);
    expect(tool.depends![0].type).toBe("tool");
    expect(tool.depends![1].type).toBe("agent");
    expect(tool.depends![1].skill).toBe("summarize");
  });
});
```

### Acceptance Criteria

- [ ] All types match proto/Rust definitions
- [ ] Builder API is fluent and type-safe
- [ ] Schema refs work correctly
- [ ] All test stubs pass

---

## WP6: TypeScript DSL Validation

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 2-3 days  
**Dependencies**: WP5

### Objective

Client-side validation matching Rust validation logic.

### Files to Create

- `packages/registry-dsl/src/validate.ts`

### Test Stubs

```typescript
// validate.test.ts

describe("validateRegistry", () => {
  it("should validate a valid registry", () => {
    const registry = loadV2Example();
    const result = validateRegistry(registry);
    expect(result.isValid).toBe(true);
    expect(result.errors).toHaveLength(0);
  });

  it("should detect missing schema ref", () => {
    const registry = new RegistryBuilder()
      .tool({
        name: "test",
        version: "1.0.0",
        inputSchema: { $ref: "#NonExistent:1.0.0" },
      })
      .build();

    const result = validateRegistry(registry);
    expect(result.isValid).toBe(false);
    expect(result.errors[0].type).toBe("SCHEMA_NOT_FOUND");
  });

  it("should detect circular dependencies", () => {
    const registry = new RegistryBuilder()
      .tool({
        name: "a",
        version: "1.0.0",
        depends: [{ type: "tool", name: "b", version: "1.0.0" }],
        spec: { pipeline: { steps: [] } },
      })
      .tool({
        name: "b",
        version: "1.0.0",
        depends: [{ type: "tool", name: "a", version: "1.0.0" }],
        spec: { pipeline: { steps: [] } },
      })
      .build();

    const result = validateRegistry(registry);
    expect(result.isValid).toBe(false);
    expect(result.errors[0].type).toBe("CIRCULAR_DEPENDENCY");
  });
});
```

### Acceptance Criteria

- [ ] Validation logic matches Rust implementation
- [ ] Helpful error messages with context
- [ ] All test stubs pass

---

## WP7: SBOM Export

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 2-3 days  
**Dependencies**: WP2, WP3

### Objective

Export registry as CycloneDX or SPDX SBOM format.

### Files to Create

- `crates/agentgateway/src/mcp/registry/sbom.rs`
- `crates/agentgateway/src/commands/sbom.rs`

### CLI Interface

```bash
# Export full SBOM
agentgateway sbom export --format cyclonedx --output sbom.json

# Export for specific tool
agentgateway sbom export --tool search_documents:1.0.0 --format spdx
```

### Test Stubs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_cyclonedx() {
        let registry = load_v2_example();
        let sbom = export_cyclonedx(&registry).expect("should export");
        
        assert_eq!(sbom["bomFormat"], "CycloneDX");
        assert!(!sbom["components"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_sbom_includes_all_dependencies() {
        let registry = load_v2_example();
        let sbom = export_cyclonedx(&registry).expect("should export");
        
        let components = sbom["components"].as_array().unwrap();
        
        // Should include tools, agents, and their dependencies
        let tool_names: Vec<&str> = components
            .iter()
            .filter(|c| c["type"] == "library")
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        
        assert!(tool_names.contains(&"search_documents"));
        assert!(tool_names.contains(&"research-agent"));
    }
}
```

### Acceptance Criteria

- [ ] CycloneDX export works
- [ ] SPDX export works
- [ ] CLI command implemented
- [ ] All test stubs pass

---

## WP8: Test Suite

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 3-4 days  
**Dependencies**: WP2, WP5

### Objective

Comprehensive test fixtures and integration tests.

### Files to Create

- `crates/agentgateway/tests/registry_v2/mod.rs`
- `crates/agentgateway/tests/registry_v2/parsing.rs`
- `crates/agentgateway/tests/registry_v2/validation.rs`
- `crates/agentgateway/tests/registry_v2/resolution.rs`
- `packages/registry-dsl/tests/integration.test.ts`

### Acceptance Criteria

- [ ] Example registry parses correctly
- [ ] All validation scenarios covered
- [ ] Edge cases documented and tested
- [ ] Performance benchmarks for large registries

---

## WP9: Documentation

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 2-3 days  
**Dependencies**: All

### Objective

User-facing documentation and migration guide.

### Files to Create

- `docs/user-guide/registry-v2.md`
- `docs/migration/v1-to-v2.md`
- `docs/api/registry-schema.md`

### Acceptance Criteria

- [ ] Schema documented with examples
- [ ] Migration guide complete
- [ ] API reference generated

---

---

## WP10: Caller Identity

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 2-3 days  
**Dependencies**: WP2

### Objective

Extract caller identity from requests (MCP and A2A) for dependency enforcement.

### Files to Create

- `crates/agentgateway/src/mcp/identity.rs`
- `crates/agentgateway/src/a2a/identity.rs`

### Core Types

```rust
/// Caller identity extracted from request
#[derive(Debug, Clone)]
pub struct CallerIdentity {
    pub name: String,
    pub version: String,
    pub source: IdentitySource,
}

#[derive(Debug, Clone)]
pub enum IdentitySource {
    Header,           // X-Agent-Name + X-Agent-Version headers
    Jwt(String),      // JWT claim
    McpClientInfo,    // MCP initialize clientInfo
    A2aCallerAgent,   // X-Caller-Agent header (URL to AgentCard)
    Unknown,
}

/// Extract identity from HTTP request
pub fn extract_identity(
    headers: &HeaderMap,
    mcp_client_info: Option<&ClientInfo>,
) -> Option<CallerIdentity> {
    // Try headers first
    if let (Some(name), Some(version)) = (
        headers.get("x-agent-name"),
        headers.get("x-agent-version"),
    ) {
        return Some(CallerIdentity {
            name: name.to_str().ok()?.to_string(),
            version: version.to_str().ok()?.to_string(),
            source: IdentitySource::Header,
        });
    }
    
    // Fall back to MCP clientInfo
    if let Some(info) = mcp_client_info {
        return Some(CallerIdentity {
            name: info.name.clone(),
            version: info.version.clone().unwrap_or_else(|| "unknown".to_string()),
            source: IdentitySource::McpClientInfo,
        });
    }
    
    None
}
```

### Test Stubs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-agent-name", "research-agent".parse().unwrap());
        headers.insert("x-agent-version", "2.1.0".parse().unwrap());
        
        let identity = extract_identity(&headers, None).unwrap();
        assert_eq!(identity.name, "research-agent");
        assert_eq!(identity.version, "2.1.0");
        assert!(matches!(identity.source, IdentitySource::Header));
    }

    #[test]
    fn test_fallback_to_mcp_client_info() {
        let headers = HeaderMap::new();
        let client_info = ClientInfo {
            name: "my-agent".to_string(),
            version: Some("1.0.0".to_string()),
        };
        
        let identity = extract_identity(&headers, Some(&client_info)).unwrap();
        assert_eq!(identity.name, "my-agent");
        assert!(matches!(identity.source, IdentitySource::McpClientInfo));
    }

    #[test]
    fn test_unknown_caller() {
        let headers = HeaderMap::new();
        let identity = extract_identity(&headers, None);
        assert!(identity.is_none());
    }
}
```

### Acceptance Criteria

- [ ] Extract identity from X-Agent-Name/Version headers
- [ ] Fall back to MCP clientInfo
- [ ] Support A2A X-Caller-Agent header
- [ ] Identity available in request context

---

## WP11: Dependency-Scoped Discovery

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 3-4 days  
**Dependencies**: WP10, WP3

### Objective

Filter `tools/list` results based on caller's declared dependencies.

### Files to Modify

- `crates/agentgateway/src/mcp/handler.rs`
- `crates/agentgateway/src/mcp/session.rs`

### Implementation

```rust
// In handler.rs, modify merge_tools():

pub fn merge_tools(&self, cel: CelContext, caller: Option<CallerIdentity>) -> MergeFn {
    let registry = self.registry.clone();
    let config = self.config.clone();

    Box::new(move |streams| {
        let backend_tools = /* ... existing code ... */;
        let transformed_tools = /* ... existing code ... */;

        // NEW: Dependency filtering
        let filtered_tools = match (&caller, &registry) {
            (Some(agent), Some(reg)) => {
                let guard = reg.get();
                if let Some(compiled) = guard.as_ref() {
                    transformed_tools
                        .into_iter()
                        .filter(|(_, tool)| {
                            compiled.agent_can_access_tool(
                                &agent.name,
                                &agent.version,
                                &tool.name,
                            )
                        })
                        .collect()
                } else {
                    transformed_tools
                }
            }
            (None, _) if config.allow_unknown_caller => transformed_tools,
            (None, _) => {
                // Reject or warn based on config
                vec![]
            }
            _ => transformed_tools,
        };

        // Continue with RBAC filtering...
    })
}
```

### Test Stubs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scoped_discovery_returns_only_dependencies() {
        let registry = load_v2_example();
        let handler = create_test_handler(registry);
        
        // research-agent depends on: search_documents, fetch, create_entities
        let caller = CallerIdentity {
            name: "research-agent".into(),
            version: "2.1.0".into(),
            source: IdentitySource::Header,
        };
        
        let tools = handler.list_tools_for_caller(Some(caller)).await;
        
        let tool_names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"search_documents"));
        assert!(tool_names.contains(&"fetch"));
        assert!(!tool_names.contains(&"send_notification")); // Not in depends
    }

    #[tokio::test]
    async fn test_unknown_caller_policy_allow() {
        let config = RuntimeConfig { allow_unknown_caller: true, .. };
        let handler = create_test_handler_with_config(config);
        
        let tools = handler.list_tools_for_caller(None).await;
        assert!(!tools.is_empty()); // Gets all tools
    }

    #[tokio::test]
    async fn test_unknown_caller_policy_deny() {
        let config = RuntimeConfig { allow_unknown_caller: false, .. };
        let handler = create_test_handler_with_config(config);
        
        let tools = handler.list_tools_for_caller(None).await;
        assert!(tools.is_empty()); // Gets nothing
    }
}
```

### Acceptance Criteria

- [ ] `tools/list` filters by caller's depends when caller identified
- [ ] Unknown caller policy configurable (allow/warn/deny)
- [ ] Registered agent only sees declared tools
- [ ] Metrics for dependency mismatches

---

## WP12: Agent Multiplexing (A2A) — *Phase 2*

> **⚠️ Phase 2**: This work package is deferred until Phase 1 is complete and validated.

**Status**: Deferred (Phase 2)  
**Owner**: TBD  
**Estimated Effort**: 4-5 days  
**Dependencies**: WP2, Phase 1 complete

### Objective

Route A2A requests to multiple registered agents (like MCP multiplexing).

### Files to Create/Modify

- `crates/agentgateway/src/a2a/router.rs` (new)
- `crates/agentgateway/src/a2a/mod.rs` (modify)

### Design

```rust
/// A2A router that multiplexes across registered agents
pub struct A2aRouter {
    /// Registry with agent definitions
    registry: Arc<RegistryStoreRef>,
    /// HTTP client for backend calls
    http_client: reqwest::Client,
}

impl A2aRouter {
    /// Route A2A request to appropriate agent
    pub async fn route(&self, req: Request<Body>) -> Result<Response<Body>, A2aError> {
        let path = req.uri().path();
        
        // Path-based routing: /a2a/{agent-name}/...
        if let Some(agent_name) = extract_agent_from_path(path) {
            return self.route_to_agent(&agent_name, req).await;
        }
        
        // Discovery endpoint: /.well-known/agents
        if path == "/.well-known/agents" {
            return self.handle_discovery().await;
        }
        
        Err(A2aError::AgentNotSpecified)
    }
    
    /// Route to specific agent by name
    async fn route_to_agent(
        &self,
        agent_name: &str,
        req: Request<Body>,
    ) -> Result<Response<Body>, A2aError> {
        let agent = self.registry.lookup_agent(agent_name)?;
        
        // Forward to agent's URL
        let backend_url = &agent.url;
        // ... forward request ...
    }
    
    /// Return aggregated list of registered agents
    async fn handle_discovery(&self) -> Result<Response<Body>, A2aError> {
        let agents = self.registry.list_agents();
        let response = agents.iter().map(|a| AgentCardSummary {
            name: a.name.clone(),
            version: a.version.clone(),
            description: a.description.clone(),
            url: format!("/a2a/{}", a.name), // Gateway URL
            skills: a.skills.iter().map(|s| s.id.clone()).collect(),
        }).collect::<Vec<_>>();
        
        Ok(Response::builder()
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&response)?.into())?)
    }
}
```

### Test Stubs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_route_to_agent_by_path() {
        let router = create_test_router();
        
        let req = Request::builder()
            .uri("/a2a/research-agent/message/send")
            .body(Body::empty())
            .unwrap();
        
        let response = router.route(req).await;
        // Should route to research-agent backend
    }

    #[tokio::test]
    async fn test_discovery_endpoint() {
        let router = create_test_router();
        
        let req = Request::builder()
            .uri("/.well-known/agents")
            .body(Body::empty())
            .unwrap();
        
        let response = router.route(req).await.unwrap();
        let agents: Vec<AgentCardSummary> = serde_json::from_slice(
            &hyper::body::to_bytes(response.into_body()).await.unwrap()
        ).unwrap();
        
        assert!(agents.iter().any(|a| a.name == "research-agent"));
        assert!(agents.iter().any(|a| a.name == "summarizer-agent"));
    }

    #[tokio::test]
    async fn test_agent_not_found() {
        let router = create_test_router();
        
        let req = Request::builder()
            .uri("/a2a/nonexistent-agent/message/send")
            .body(Body::empty())
            .unwrap();
        
        let result = router.route(req).await;
        assert!(matches!(result, Err(A2aError::AgentNotFound(_))));
    }
}
```

### Acceptance Criteria

- [ ] Path-based routing: `/a2a/{agent}/...`
- [ ] Discovery endpoint: `/.well-known/agents`
- [ ] Agent lookup from registry
- [ ] AgentCard URL rewriting to gateway URLs

---

## WP13: Agent-as-Tool Executor — *Phase 2*

> **⚠️ Phase 2**: This work package is deferred until Phase 1 is complete and validated.

**Status**: Deferred (Phase 2)  
**Owner**: TBD  
**Estimated Effort**: 4-5 days  
**Dependencies**: WP12, WP4, Phase 1 complete

### Objective

Execute agents as steps in tool compositions (pipelines, scatter-gather).

### Files to Create

- `crates/agentgateway/src/mcp/registry/executor/agent.rs`

### Design

```rust
/// Execute an agent call within a composition
pub async fn execute_agent_call(
    call: &AgentCall,
    input: serde_json::Value,
    registry: &CompiledRegistry,
    a2a_client: &A2aClient,
) -> Result<serde_json::Value, ExecutorError> {
    // 1. Look up agent in registry
    let agent = registry.get_agent(&call.name, &call.version)?;
    
    // 2. Find the skill
    let skill = agent.skills.iter()
        .find(|s| s.id == call.skill)
        .ok_or(ExecutorError::SkillNotFound)?;
    
    // 3. Validate input against skill's inputSchema
    if let Some(schema) = &skill.input_schema {
        validate_json(&input, schema)?;
    }
    
    // 4. Build A2A message with DataPart
    let message = a2a_sdk::Message {
        role: "user".into(),
        parts: vec![
            a2a_sdk::Part::Data(a2a_sdk::DataPart { data: input })
        ],
    };
    
    // 5. Send to agent
    let response = a2a_client.send_message(&agent.url, message).await?;
    
    // 6. Extract result from response
    let result = extract_data_part(&response)?;
    
    // 7. Validate output against skill's outputSchema
    if let Some(schema) = &skill.output_schema {
        validate_json(&result, schema)?;
    }
    
    Ok(result)
}
```

### Test Stubs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_agent_call() {
        let registry = load_v2_example();
        let mock_agent = MockA2aAgent::new();
        mock_agent.expect_skill("summarize")
            .returning(|input| Ok(json!({ "summary": "test summary" })));
        
        let result = execute_agent_call(
            &AgentCall {
                name: "summarizer-agent".into(),
                skill: "summarize".into(),
            },
            json!({ "content": "long text here..." }),
            &registry,
            &mock_agent.client(),
        ).await.unwrap();
        
        assert_eq!(result["summary"], "test summary");
    }

    #[tokio::test]
    async fn test_agent_input_validation() {
        let registry = load_v2_example();
        
        // Missing required field
        let result = execute_agent_call(
            &AgentCall {
                name: "summarizer-agent".into(),
                skill: "summarize".into(),
            },
            json!({ "wrong_field": "value" }),  // Missing "content"
            &registry,
            &mock_client(),
        ).await;
        
        assert!(matches!(result, Err(ExecutorError::InputValidation(_))));
    }

    #[tokio::test]
    async fn test_pipeline_with_agent_steps() {
        let registry = load_v2_example();
        
        // research_and_summarize pipeline:
        // Step 1: research-agent:research_topic
        // Step 2: summarizer-agent:summarize
        
        let result = execute_composition(
            "research_and_summarize",
            json!({ "topic": "quantum computing" }),
            &registry,
        ).await.unwrap();
        
        assert!(result.get("research").is_some());
        assert!(result.get("executiveSummary").is_some());
    }
}
```

### Acceptance Criteria

- [ ] AgentCall execution works in pipelines
- [ ] Input mapped to A2A DataPart
- [ ] Output extracted from A2A response
- [ ] Schema validation for skill I/O
- [ ] Error handling for agent failures

---

## WP14: Discovery Endpoints (Phase 1: MCP Only)

**Status**: Not Started  
**Owner**: TBD  
**Estimated Effort**: 2-3 days  
**Dependencies**: WP2

### Objective

Implement versioned tool discovery for MCP clients.

### Files to Create/Modify

- `crates/agentgateway/src/mcp/handler.rs` (modify for versioned tools)

### Endpoints (Phase 1)

| Endpoint | Description |
|----------|-------------|
| `/mcp` + `tools/list` | Already exists, add version metadata |

### Endpoints (Phase 2 - Deferred)

| Endpoint | Description |
|----------|-------------|
| `/.well-known/agents` | List all registered agents |
| `/.well-known/agents/{name}` | Get specific agent's card |

### Test Stubs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_agents() {
        let server = create_test_server();
        
        let response = server.get("/.well-known/agents").await;
        assert_eq!(response.status(), 200);
        
        let agents: Vec<AgentCardSummary> = response.json().await;
        assert!(agents.iter().any(|a| a.name == "research-agent"));
    }

    #[tokio::test]
    async fn test_get_agent_card() {
        let server = create_test_server();
        
        let response = server.get("/.well-known/agents/research-agent").await;
        assert_eq!(response.status(), 200);
        
        let card: AgentCard = response.json().await;
        assert_eq!(card.name, "research-agent");
        assert_eq!(card.version, "2.1.0");
    }
}
```

### Acceptance Criteria

- [ ] `/.well-known/agents` returns registered agents
- [ ] Individual agent cards accessible
- [ ] URLs rewritten to gateway paths

---

## WP16: Version-Aware Server Routing

**Status**: Not Started
**Owner**: TBD
**Estimated Effort**: 3-4 days
**Dependencies**: WP2

### Objective

Update routing to use versioned server keys for dispatch. YAML targets embed version in name, registry `source.server` + `source.serverVersion` constructs the lookup key.

### Design

**YAML config targets:**
```yaml
targets:
- name: document-service:1.2.0
  stdio:
    cmd: npx
    args: ["@agentgateway/document-service@1.2.0"]
- name: document-service:1.1.0  # Previous version still available
  stdio:
    cmd: npx
    args: ["@agentgateway/document-service@1.1.0"]
```

**Registry tool definition:**
```json
{
  "name": "search_documents",
  "version": "1.0.0",
  "source": {
    "server": "document-service",
    "serverVersion": "1.2.0",
    "tool": "search_documents"
  }
}
```

**Routing logic:**
```rust
// In handler.rs resolve_tool_call()
let target_key = match &source.server_version {
    Some(version) => format!("{}:{}", source.server, version),
    None => source.server.clone(),  // v1 fallback
};

// target_key = "document-service:1.2.0"
let upstream = upstream_group.get(&target_key)?;
```

### Files to Modify

- `crates/agentgateway/src/mcp/handler.rs` (resolve_tool_call routing)
- `crates/agentgateway/src/mcp/registry/types.rs` (add server_version to SourceTool)
- `crates/agentgateway/src/mcp/registry/compiled.rs` (update source key construction)

### Test Stubs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_versioned_target_key_construction() {
        let source = SourceToolV2 {
            server: "doc-service".into(),
            server_version: Some("1.2.0".into()),
            tool: "search".into(),
            ..Default::default()
        };

        let key = build_target_key(&source);
        assert_eq!(key, "doc-service:1.2.0");
    }

    #[test]
    fn test_unversioned_fallback() {
        let source = SourceToolV2 {
            server: "doc-service".into(),
            server_version: None,  // v1 style
            tool: "search".into(),
            ..Default::default()
        };

        let key = build_target_key(&source);
        assert_eq!(key, "doc-service");  // No version suffix
    }

    #[tokio::test]
    async fn test_route_to_versioned_target() {
        let handler = create_test_handler_with_targets(vec![
            ("doc-service:1.2.0", mock_upstream_v1_2()),
            ("doc-service:1.1.0", mock_upstream_v1_1()),
        ]);

        // Tool configured for 1.2.0
        let result = handler.resolve_tool_call("search_documents", json!({})).await;

        assert!(matches!(result, ResolvedToolCall::Backend { target, .. } if target == "doc-service:1.2.0"));
    }

    #[tokio::test]
    async fn test_validate_registry_server_matches_yaml_target() {
        let registry = RegistryV2 {
            servers: vec![ServerDefinition {
                name: "doc-service".into(),
                version: "1.2.0".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let yaml_targets = vec!["doc-service:1.2.0"];

        let result = validate_servers_have_targets(&registry, &yaml_targets);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_missing_target_for_server_version() {
        let registry = RegistryV2 {
            servers: vec![ServerDefinition {
                name: "doc-service".into(),
                version: "2.0.0".into(),  // Version not in YAML
                ..Default::default()
            }],
            ..Default::default()
        };

        let yaml_targets = vec!["doc-service:1.2.0"];  // Only has 1.2.0

        let result = validate_servers_have_targets(&registry, &yaml_targets);
        assert!(result.is_err());  // Missing target for 2.0.0
    }
}
```

### Acceptance Criteria

- [ ] `source.server` + `source.serverVersion` constructs `server:version` key
- [ ] YAML targets with embedded version work
- [ ] v1 tools without serverVersion fall back to unversioned key
- [ ] Startup validation: registry servers have matching YAML targets
- [ ] All test stubs pass

---

## Updated Dependency Graph

### Phase 1 (MVP) Dependencies

```
WP1 (Proto) ─────┬────────────────────────────────────────────────┐
                 │                                                │
                 ▼                                                ▼
        WP2 (Rust Types) ───────────────────────────────  WP5 (TS Types)
                 │                                                │
        ┌────────┼────────┬────────────┬──────────┐              ▼
        ▼        ▼        ▼            ▼          ▼       WP6 (TS Validation)
   WP3 (Rust  WP7 (SBOM) WP10       WP14       WP16
   Validation)           (Identity)  (Discovery) (Routing)
        │                    │
        ▼                    ▼
   WP4 (Runtime) ◄───── WP11 (Scoped Discovery)

WP8 (Tests) can start immediately with stubs
WP9 (Docs) runs last
WP16 (Routing) is runtime critical - version-aware dispatch
```

### Phase 2 (Future) Dependencies

```
Phase 1 Complete
        │
        ▼
   WP12 (A2A Multiplexing)  ← Deferred
        │
        ▼
   WP13 (Agent-as-Tool)     ← Deferred
        │
        ▼
   WP15 (Agent Discovery)   ← Deferred
```

---

## Getting Started (Phase 1)

1. **Read the design doc first**
   ```bash
   cat docs/design/registry-v2.md
   ```

2. **Understand the architecture intent**
   - The registry JSON is a local cache for a future centralized Registry Service
   - Phase 1 focuses on agent-to-MCP-tool (no agent-as-tool yet)
   - Agents are stored in registry for dependency tracking, not for invocation

3. **Review the example registry**
   ```bash
   cat examples/pattern-demos/configs/registry-v2-example.json
   ```

4. **Pick a Phase 1 work package** based on dependencies
   - WP1 (Proto) has no dependencies - start here
   - WP5 (TS Types) can start in parallel with WP2
   - WP10-11 are the **runtime critical path** for a working demo
   - Skip WP12-13 (Phase 2)

5. **Write failing tests first** using the stubs above

6. **Implement until tests pass**

7. **Submit PR** with tests and implementation

### What NOT to Implement (Phase 2)

Do not work on these until Phase 1 is complete and validated:

- WP12: A2A agent multiplexing
- WP13: Agent-as-tool execution in compositions
- WP15: `/.well-known/agents` discovery endpoint

These depend on understanding how Phase 1 actually works in practice.
