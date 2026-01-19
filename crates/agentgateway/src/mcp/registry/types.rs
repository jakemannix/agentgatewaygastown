// Registry types for tool definitions
//
// Supports both virtual tools (1:1 mapping) and compositions (N:1 orchestration).
// These types correspond to the registry.proto schema.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::patterns::{FieldSource, PatternSpec, SchemaMapSpec};

/// Parsed registry from JSON
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
	/// Schema version for compatibility (e.g., "2.0")
	#[serde(default = "default_schema_version")]
	pub schema_version: String,

	/// List of tool definitions (virtual tools and compositions)
	#[serde(default)]
	pub tools: Vec<ToolDefinition>,

	/// Reusable JSON Schema definitions that can be referenced by $ref (v2)
	#[serde(default)]
	pub schemas: Vec<Schema>,

	/// Backend server declarations with version information (v2)
	#[serde(default)]
	pub servers: Vec<Server>,

	/// A2A agent definitions with capabilities and dependencies (v2)
	#[serde(default)]
	pub agents: Vec<AgentDefinition>,

	/// Registry-level metadata (owner, classification, etc.)
	#[serde(default)]
	pub metadata: HashMap<String, serde_json::Value>,
}

fn default_schema_version() -> String {
	"1.0".to_string()
}

// =============================================================================
// Schema Definitions (v2)
// =============================================================================

/// Schema defines a reusable, versioned JSON Schema that tools can reference.
/// Reference format: "#/schemas/<name>" or "#<name>:<version>"
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
	/// Schema name (used in $ref, e.g., "SearchQuery")
	pub name: String,

	/// Semantic version of this schema (e.g., "1.0.0")
	#[serde(default)]
	pub version: Option<String>,

	/// Human-readable description
	#[serde(default)]
	pub description: Option<String>,

	/// The JSON Schema definition
	pub schema: serde_json::Value,

	/// Schema metadata (owner, data classification, etc.)
	#[serde(default)]
	pub metadata: HashMap<String, serde_json::Value>,
}

/// SchemaRef allows referencing either an inline schema or a $ref to a named schema
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SchemaRef {
	/// Inline JSON Schema definition
	Inline(serde_json::Value),

	/// Reference to a named schema (e.g., "#/schemas/SearchQuery" or "#SearchQuery:1.0.0")
	#[serde(rename = "$ref")]
	Ref(String),
}

// =============================================================================
// Server Definitions (v2)
// =============================================================================

/// Server declares a backend MCP server with version information for routing.
/// The server name + version forms the lookup key (e.g., "doc-service:1.2.0").
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
	/// Server name (must match YAML target name without version suffix)
	pub name: String,

	/// Semantic version of this server (e.g., "1.2.0")
	#[serde(default)]
	pub version: Option<String>,

	/// Human-readable description
	#[serde(default)]
	pub description: Option<String>,

	/// Tools provided by this server (tool name -> version)
	#[serde(default)]
	pub provides: Vec<ToolProvision>,

	/// Whether this server is deprecated
	#[serde(default)]
	pub deprecated: bool,

	/// Deprecation message explaining migration path
	#[serde(default)]
	pub deprecation_message: Option<String>,

	/// Server metadata (owner, SLA, health endpoint, etc.)
	#[serde(default)]
	pub metadata: HashMap<String, serde_json::Value>,
}

/// ToolProvision declares a tool provided by a server
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolProvision {
	/// Tool name as exposed by the server
	pub tool: String,

	/// Tool version
	#[serde(default)]
	pub version: Option<String>,
}

// =============================================================================
// Agent Definitions (v2) - A2A AgentCard Compatible
// =============================================================================

/// AgentDefinition matches the A2A AgentCard structure for interoperability.
/// Agents can be invoked as tools in compositions or delegate to other agents.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
	/// Unique agent name/identifier
	pub name: String,

	/// Semantic version of this agent
	#[serde(default)]
	pub version: Option<String>,

	/// Human-readable description of the agent's purpose
	#[serde(default)]
	pub description: Option<String>,

	/// A2A endpoint URL (direct connection)
	#[serde(default)]
	pub url: Option<String>,

	/// A2A protocol version supported (e.g., "0.2.1")
	#[serde(default)]
	pub protocol_version: Option<String>,

	/// Default input content types accepted (e.g., "text", "application/json")
	#[serde(default)]
	pub default_input_modes: Vec<String>,

	/// Default output content types produced
	#[serde(default)]
	pub default_output_modes: Vec<String>,

	/// Skills/capabilities this agent exposes
	#[serde(default)]
	pub skills: Vec<AgentSkillDefinition>,

	/// Agent capabilities
	#[serde(default)]
	pub capabilities: Option<AgentCapabilities>,

	/// Provider information (optional)
	#[serde(default)]
	pub provider: Option<AgentProvider>,

	/// Agent metadata (owner, model, cost tier, etc.)
	#[serde(default)]
	pub metadata: HashMap<String, serde_json::Value>,
}

/// AgentSkillDefinition describes a capability that an agent exposes
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkillDefinition {
	/// Skill identifier (unique within agent, e.g., "research_topic")
	pub id: String,

	/// Human-readable skill name
	#[serde(default)]
	pub name: Option<String>,

	/// Skill description
	#[serde(default)]
	pub description: Option<String>,

	/// Tags for categorization/discovery
	#[serde(default)]
	pub tags: Vec<String>,

	/// Example prompts demonstrating usage
	#[serde(default)]
	pub examples: Vec<String>,

	/// Input content types for this skill
	#[serde(default)]
	pub input_modes: Vec<String>,

	/// Output content types for this skill
	#[serde(default)]
	pub output_modes: Vec<String>,

	/// Input schema (inline or reference)
	#[serde(default)]
	pub input_schema: Option<SchemaRef>,

	/// Output schema (inline or reference)
	#[serde(default)]
	pub output_schema: Option<SchemaRef>,
}

/// AgentCapabilities describes what an agent supports
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
	/// Supports streaming responses
	#[serde(default)]
	pub streaming: Option<bool>,

	/// Supports push notifications
	#[serde(default)]
	pub push_notifications: Option<bool>,

	/// Supports state transition history
	#[serde(default)]
	pub state_transition_history: Option<bool>,

	/// Protocol extensions (e.g., SBOM for dependency declaration)
	#[serde(default)]
	pub extensions: Vec<AgentExtension>,
}

/// AgentExtension describes a protocol extension
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExtension {
	/// Extension URI (e.g., "urn:agentgateway:sbom")
	pub uri: String,

	/// Extension description
	#[serde(default)]
	pub description: Option<String>,

	/// Whether this extension is required
	#[serde(default)]
	pub required: Option<bool>,

	/// Extension parameters (e.g., depends array for SBOM)
	#[serde(default)]
	pub params: Option<serde_json::Value>,
}

/// AgentProvider describes who provides an agent
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvider {
	/// Organization name
	pub organization: String,

	/// Organization URL
	#[serde(default)]
	pub url: Option<String>,
}

// =============================================================================
// Dependency Declarations (v2)
// =============================================================================

/// Dependency declares a versioned dependency on a tool or agent.
/// Used by tools (compositions) and agents to declare requirements.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
	/// Type of dependency (tool or agent)
	#[serde(rename = "type")]
	pub dep_type: DependencyType,

	/// Name of the required tool or agent
	pub name: String,

	/// Semantic version requirement (e.g., "1.2.3")
	#[serde(default)]
	pub version: Option<String>,

	/// For agent dependencies: specific skill to invoke (optional)
	#[serde(default)]
	pub skill: Option<String>,
}

/// DependencyType identifies whether a dependency is a tool or agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
	Tool,
	Agent,
}

impl Default for DependencyType {
	fn default() -> Self {
		DependencyType::Tool
	}
}

// =============================================================================
// Tool Definitions
// =============================================================================

/// Unified tool definition - either a virtual tool or a composition
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
	/// Name exposed to agents (unique identifier)
	pub name: String,

	/// Optional description
	#[serde(default)]
	pub description: Option<String>,

	/// Tool implementation - either source-based or composition
	#[serde(flatten)]
	pub implementation: ToolImplementation,

	/// Input schema override (JSON Schema)
	#[serde(default)]
	pub input_schema: Option<serde_json::Value>,

	/// Output transformation (HOW to generate structured output - internal)
	#[serde(default)]
	pub output_transform: Option<OutputTransform>,

	/// Output schema (WHAT the output looks like - JSON Schema, sent to MCP clients)
	#[serde(default)]
	pub output_schema: Option<serde_json::Value>,

	/// Semantic version of this tool definition
	#[serde(default)]
	pub version: Option<String>,

	/// Arbitrary metadata (owner, classification, etc.)
	#[serde(default)]
	pub metadata: HashMap<String, serde_json::Value>,

	/// Tags for categorization and discovery (v2)
	#[serde(default)]
	pub tags: Vec<String>,

	/// Deprecation notice - if set, tool is deprecated (v2)
	#[serde(default)]
	pub deprecated: Option<String>,

	/// Dependencies this tool requires - for compositions and tracking (v2)
	#[serde(default)]
	pub depends: Vec<Dependency>,
}

/// Tool implementation - either source-based (1:1) or composition (N:1)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolImplementation {
	/// Virtual tool: adapts a single backend tool (1:1)
	Source(SourceTool),

	/// Composition: orchestrates multiple tools (N:1)
	Spec(PatternSpec),
}

/// Source tool definition - maps to a single backend tool
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTool {
	/// Target name (MCP server/backend name)
	/// Note: Proto uses "server" but we keep "target" for v1 compatibility
	pub target: String,

	/// Original tool name on that target
	pub tool: String,

	/// Fields to inject at call time (supports ${ENV_VAR} substitution)
	#[serde(default)]
	pub defaults: HashMap<String, serde_json::Value>,

	/// Fields to remove from schema (hidden from agents)
	#[serde(default)]
	pub hide_fields: Vec<String>,

	/// Server version for versioned routing (e.g., "1.2.0") (v2)
	/// Combined with target name forms lookup key: "target:server_version"
	/// If not set, routes to unversioned target or latest version
	#[serde(default)]
	pub server_version: Option<String>,
}

/// Output transformation - enhanced version supporting all mapping features
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputTransform {
	/// Field name -> source mapping
	pub mappings: HashMap<String, FieldSource>,
}

impl OutputTransform {
	/// Create from a SchemaMapSpec
	pub fn from_schema_map(schema_map: SchemaMapSpec) -> Self {
		Self {
			mappings: schema_map.mappings,
		}
	}

	/// Create an empty output transform
	pub fn empty() -> Self {
		Self {
			mappings: HashMap::new(),
		}
	}

	/// Check if this transform has any mappings
	pub fn is_empty(&self) -> bool {
		self.mappings.is_empty()
	}
}

// =============================================================================
// Legacy compatibility: VirtualToolDef alias
// =============================================================================

/// Virtual tool definition from registry (legacy alias)
///
/// This type is provided for backward compatibility. New code should use
/// `ToolDefinition` with `ToolImplementation::Source`.
pub type VirtualToolDef = LegacyVirtualToolDef;

/// Legacy virtual tool definition (for backward compatibility)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyVirtualToolDef {
	/// Name exposed to agents (the virtual/renamed tool name)
	pub name: String,

	/// Source backend tool mapping
	pub source: ToolSource,

	/// Override description (optional, inherits from source if not set)
	#[serde(default)]
	pub description: Option<String>,

	/// Override input schema (optional, inherits from source if not set)
	#[serde(default)]
	pub input_schema: Option<serde_json::Value>,

	/// Fields to inject at call time (supports ${ENV_VAR} substitution)
	#[serde(default)]
	pub defaults: HashMap<String, serde_json::Value>,

	/// Fields to remove from schema (hidden from agents)
	#[serde(default)]
	pub hide_fields: Vec<String>,

	/// Output transformation schema with JSONPath mappings (legacy format)
	#[serde(default)]
	pub output_schema: Option<OutputSchema>,

	/// Semantic version of this tool definition
	#[serde(default)]
	pub version: Option<String>,

	/// Arbitrary metadata (owner, classification, etc.)
	#[serde(default)]
	pub metadata: HashMap<String, serde_json::Value>,
}

/// Source backend tool reference (legacy)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolSource {
	/// Target name (MCP server/backend name)
	pub target: String,

	/// Original tool name on that target
	pub tool: String,
}

/// Output transformation schema (legacy format)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSchema {
	/// Schema type (typically "object")
	#[serde(rename = "type", default = "default_object_type")]
	pub schema_type: String,

	/// Field definitions with JSONPath mappings
	#[serde(default)]
	pub properties: HashMap<String, OutputField>,
}

fn default_object_type() -> String {
	"object".to_string()
}

/// Output field definition (legacy format)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputField {
	/// JSON Schema type (string, number, boolean, object, array)
	#[serde(rename = "type")]
	pub field_type: String,

	/// JSONPath expression to extract value from backend response
	#[serde(default)]
	pub source_field: Option<String>,

	/// Optional description for the field
	#[serde(default)]
	pub description: Option<String>,
}

// =============================================================================
// Implementations
// =============================================================================

impl Registry {
	/// Create an empty registry
	pub fn new() -> Self {
		Self::default()
	}

	/// Create a registry with the given tools (unified format)
	pub fn with_tool_definitions(tools: Vec<ToolDefinition>) -> Self {
		Self {
			schema_version: default_schema_version(),
			tools,
			schemas: Vec::new(),
			servers: Vec::new(),
			agents: Vec::new(),
			metadata: HashMap::new(),
		}
	}

	/// Create a registry with legacy virtual tool definitions
	pub fn with_tools(tools: Vec<VirtualToolDef>) -> Self {
		Self {
			schema_version: default_schema_version(),
			tools: tools.into_iter().map(ToolDefinition::from_legacy).collect(),
			schemas: Vec::new(),
			servers: Vec::new(),
			agents: Vec::new(),
			metadata: HashMap::new(),
		}
	}

	/// Check if registry has any tools
	pub fn is_empty(&self) -> bool {
		self.tools.is_empty()
	}

	/// Get number of tools
	pub fn len(&self) -> usize {
		self.tools.len()
	}
}

impl ToolDefinition {
	/// Create a source-based tool (virtual tool)
	pub fn source(
		name: impl Into<String>,
		target: impl Into<String>,
		tool: impl Into<String>,
	) -> Self {
		Self {
			name: name.into(),
			description: None,
			implementation: ToolImplementation::Source(SourceTool {
				target: target.into(),
				tool: tool.into(),
				defaults: HashMap::new(),
				hide_fields: Vec::new(),
				server_version: None,
			}),
			input_schema: None,
			output_transform: None,
			output_schema: None,
			version: None,
			metadata: HashMap::new(),
			tags: Vec::new(),
			deprecated: None,
			depends: Vec::new(),
		}
	}

	/// Create a composition-based tool
	pub fn composition(name: impl Into<String>, spec: PatternSpec) -> Self {
		Self {
			name: name.into(),
			description: None,
			implementation: ToolImplementation::Spec(spec),
			input_schema: None,
			output_transform: None,
			output_schema: None,
			version: None,
			metadata: HashMap::new(),
			tags: Vec::new(),
			deprecated: None,
			depends: Vec::new(),
		}
	}

	/// Convert from legacy VirtualToolDef
	pub fn from_legacy(legacy: VirtualToolDef) -> Self {
		let output_transform = legacy.output_schema.map(|os| {
			let mappings = os
				.properties
				.into_iter()
				.map(|(name, field)| {
					let source = if let Some(path) = field.source_field {
						FieldSource::Path(path)
					} else {
						// Passthrough: use field name as path
						FieldSource::Path(format!("$.{}", name))
					};
					(name, source)
				})
				.collect();
			OutputTransform { mappings }
		});

		Self {
			name: legacy.name,
			description: legacy.description,
			implementation: ToolImplementation::Source(SourceTool {
				target: legacy.source.target,
				tool: legacy.source.tool,
				defaults: legacy.defaults,
				hide_fields: legacy.hide_fields,
				server_version: None,
			}),
			input_schema: legacy.input_schema,
			output_transform,
			output_schema: None,
			version: legacy.version,
			metadata: legacy.metadata,
			tags: Vec::new(),
			deprecated: None,
			depends: Vec::new(),
		}
	}

	/// Builder: set description
	pub fn with_description(mut self, desc: impl Into<String>) -> Self {
		self.description = Some(desc.into());
		self
	}

	/// Builder: set output transform
	pub fn with_output_transform(mut self, transform: OutputTransform) -> Self {
		self.output_transform = Some(transform);
		self
	}

	/// Builder: set output schema (JSON Schema sent to MCP clients)
	pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
		self.output_schema = Some(schema);
		self
	}

	/// Check if this is a source-based tool
	pub fn is_source(&self) -> bool {
		matches!(self.implementation, ToolImplementation::Source(_))
	}

	/// Check if this is a composition
	pub fn is_composition(&self) -> bool {
		matches!(self.implementation, ToolImplementation::Spec(_))
	}

	/// Get the source tool if this is a source-based tool
	pub fn source_tool(&self) -> Option<&SourceTool> {
		match &self.implementation {
			ToolImplementation::Source(s) => Some(s),
			_ => None,
		}
	}

	/// Get the pattern spec if this is a composition
	pub fn pattern_spec(&self) -> Option<&PatternSpec> {
		match &self.implementation {
			ToolImplementation::Spec(s) => Some(s),
			_ => None,
		}
	}

	/// Get the names of tools referenced by this definition
	pub fn referenced_tools(&self) -> Vec<&str> {
		match &self.implementation {
			ToolImplementation::Source(_) => vec![],
			ToolImplementation::Spec(spec) => spec.referenced_tools(),
		}
	}
}

impl SourceTool {
	/// Builder: add a default value
	pub fn with_default(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
		self.defaults.insert(key.into(), value);
		self
	}

	/// Builder: set hidden fields
	pub fn with_hidden_fields(mut self, fields: Vec<String>) -> Self {
		self.hide_fields = fields;
		self
	}
}

// Legacy builder methods for VirtualToolDef
impl LegacyVirtualToolDef {
	/// Create a simple virtual tool mapping
	pub fn new(name: impl Into<String>, target: impl Into<String>, tool: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			source: ToolSource {
				target: target.into(),
				tool: tool.into(),
			},
			description: None,
			input_schema: None,
			defaults: HashMap::new(),
			hide_fields: Vec::new(),
			output_schema: None,
			version: None,
			metadata: HashMap::new(),
		}
	}

	/// Builder method to set description
	pub fn with_description(mut self, desc: impl Into<String>) -> Self {
		self.description = Some(desc.into());
		self
	}

	/// Builder method to add a default value
	pub fn with_default(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
		self.defaults.insert(key.into(), value);
		self
	}

	/// Builder method to hide fields
	pub fn with_hidden_fields(mut self, fields: Vec<String>) -> Self {
		self.hide_fields = fields;
		self
	}

	/// Builder method to set output schema
	pub fn with_output_schema(mut self, schema: OutputSchema) -> Self {
		self.output_schema = Some(schema);
		self
	}
}

impl OutputSchema {
	/// Create an output schema with the given properties
	pub fn new(properties: HashMap<String, OutputField>) -> Self {
		Self {
			schema_type: default_object_type(),
			properties,
		}
	}
}

impl OutputField {
	/// Create a new output field with JSONPath source
	pub fn new(field_type: impl Into<String>, source_field: impl Into<String>) -> Self {
		Self {
			field_type: field_type.into(),
			source_field: Some(source_field.into()),
			description: None,
		}
	}

	/// Create a field without JSONPath (passthrough)
	pub fn passthrough(field_type: impl Into<String>) -> Self {
		Self {
			field_type: field_type.into(),
			source_field: None,
			description: None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_minimal_registry() {
		let json = r#"{
            "tools": []
        }"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.schema_version, "1.0");
		assert!(registry.tools.is_empty());
	}

	#[test]
	fn test_parse_source_tool() {
		let json = r#"{
			"name": "get_weather",
			"source": {
				"target": "weather",
				"tool": "fetch_weather"
			}
		}"#;

		let tool: ToolDefinition = serde_json::from_str(json).unwrap();
		assert_eq!(tool.name, "get_weather");
		assert!(tool.is_source());
		let source = tool.source_tool().unwrap();
		assert_eq!(source.target, "weather");
		assert_eq!(source.tool, "fetch_weather");
	}

	#[test]
	fn test_parse_composition_tool() {
		let json = r#"{
			"name": "research_pipeline",
			"spec": {
				"pipeline": {
					"steps": [
						{
							"id": "search",
							"operation": { "tool": { "name": "web_search" } }
						}
					]
				}
			}
		}"#;

		let tool: ToolDefinition = serde_json::from_str(json).unwrap();
		assert_eq!(tool.name, "research_pipeline");
		assert!(tool.is_composition());
	}

	#[test]
	fn test_parse_mixed_registry() {
		let json = r#"{
			"schemaVersion": "1.0",
			"tools": [
				{
					"name": "get_weather",
					"source": {
						"target": "weather",
						"tool": "fetch_weather"
					}
				},
				{
					"name": "research_pipeline",
					"spec": {
						"scatterGather": {
							"targets": [
								{ "tool": "search_web" },
								{ "tool": "search_arxiv" }
							],
							"aggregation": { "ops": [{ "flatten": true }] }
						}
					}
				}
			]
		}"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.tools.len(), 2);
		assert!(registry.tools[0].is_source());
		assert!(registry.tools[1].is_composition());
	}

	#[test]
	fn test_parse_tool_with_output_transform() {
		let json = r#"{
			"name": "normalized_search",
			"source": {
				"target": "search",
				"tool": "raw_search"
			},
			"outputTransform": {
				"mappings": {
					"title": { "path": "$.result.title" },
					"url": { "path": "$.result.link" },
					"source": { "literal": { "stringValue": "web" } }
				}
			}
		}"#;

		let tool: ToolDefinition = serde_json::from_str(json).unwrap();
		assert!(tool.output_transform.is_some());
		let transform = tool.output_transform.unwrap();
		assert_eq!(transform.mappings.len(), 3);
	}

	#[test]
	fn test_legacy_virtual_tool_conversion() {
		let legacy = VirtualToolDef::new("get_weather", "weather", "fetch_weather")
			.with_description("Get weather info")
			.with_default("units", serde_json::json!("metric"));

		let unified = ToolDefinition::from_legacy(legacy);
		assert_eq!(unified.name, "get_weather");
		assert_eq!(unified.description, Some("Get weather info".to_string()));
		assert!(unified.is_source());

		let source = unified.source_tool().unwrap();
		assert_eq!(source.target, "weather");
		assert_eq!(
			source.defaults.get("units"),
			Some(&serde_json::json!("metric"))
		);
	}

	#[test]
	fn test_builder_source_tool() {
		let tool =
			ToolDefinition::source("my_tool", "backend", "original").with_description("A test tool");

		assert_eq!(tool.name, "my_tool");
		assert_eq!(tool.description, Some("A test tool".to_string()));
	}

	#[test]
	fn test_builder_composition_tool() {
		use super::super::patterns::{PipelineSpec, PipelineStep, StepOperation, ToolCall};

		let spec = PatternSpec::Pipeline(PipelineSpec {
			steps: vec![PipelineStep {
				id: "step1".to_string(),
				operation: StepOperation::Tool(ToolCall::new("search")),
				input: None,
			}],
		});

		let tool =
			ToolDefinition::composition("my_composition", spec).with_description("A composition");

		assert!(tool.is_composition());
		assert_eq!(tool.referenced_tools(), vec!["search"]);
	}

	#[test]
	fn test_registry_with_tools_legacy() {
		let registry = Registry::with_tools(vec![
			VirtualToolDef::new("tool1", "backend1", "original1"),
			VirtualToolDef::new("tool2", "backend2", "original2"),
		]);

		assert_eq!(registry.len(), 2);
		assert!(registry.tools[0].is_source());
		assert!(registry.tools[1].is_source());
	}

	// Preserve original test compatibility
	#[test]
	fn test_parse_registry_with_version() {
		let json = r#"{
            "schemaVersion": "2.0",
            "tools": []
        }"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.schema_version, "2.0");
	}

	#[test]
	fn test_registry_methods() {
		let empty = Registry::new();
		assert!(empty.is_empty());
		assert_eq!(empty.len(), 0);

		let with_tools = Registry::with_tools(vec![VirtualToolDef::new("tool1", "t", "t1")]);
		assert!(!with_tools.is_empty());
		assert_eq!(with_tools.len(), 1);
	}

	// =============================================================================
	// v2 Registry Tests
	// =============================================================================

	#[test]
	fn test_parse_v2_registry_with_schemas() {
		let json = r#"{
			"schemaVersion": "2.0",
			"schemas": [
				{
					"name": "WeatherInput",
					"version": "1.0.0",
					"description": "Input schema for weather queries",
					"schema": {
						"type": "object",
						"properties": {
							"city": { "type": "string" }
						},
						"required": ["city"]
					}
				}
			],
			"tools": []
		}"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.schema_version, "2.0");
		assert_eq!(registry.schemas.len(), 1);
		assert_eq!(registry.schemas[0].name, "WeatherInput");
		assert_eq!(registry.schemas[0].version, Some("1.0.0".to_string()));
	}

	#[test]
	fn test_parse_v2_registry_with_servers() {
		let json = r#"{
			"schemaVersion": "2.0",
			"servers": [
				{
					"name": "doc-service",
					"version": "1.2.0",
					"description": "Document processing service",
					"deprecated": false,
					"provides": [
						{ "tool": "search_documents", "version": "1.0.0" },
						{ "tool": "get_document" }
					]
				}
			],
			"tools": []
		}"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.servers.len(), 1);
		assert_eq!(registry.servers[0].name, "doc-service");
		assert_eq!(registry.servers[0].version, Some("1.2.0".to_string()));
		assert!(!registry.servers[0].deprecated);
		assert_eq!(registry.servers[0].provides.len(), 2);
	}

	#[test]
	fn test_parse_v2_registry_with_agents() {
		let json = r#"{
			"schemaVersion": "2.0",
			"agents": [
				{
					"name": "research-agent",
					"version": "1.0.0",
					"description": "Agent that performs research tasks",
					"url": "https://agents.internal/research",
					"protocolVersion": "0.2.1",
					"defaultInputModes": ["text", "application/json"],
					"defaultOutputModes": ["text"],
					"skills": [
						{
							"id": "research",
							"name": "Research Topic",
							"description": "Research a topic and provide summary",
							"tags": ["research", "knowledge"]
						}
					],
					"capabilities": {
						"streaming": true,
						"pushNotifications": false
					},
					"provider": {
						"organization": "Research Team",
						"url": "https://research.example.com"
					}
				}
			],
			"tools": []
		}"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.agents.len(), 1);
		let agent = &registry.agents[0];
		assert_eq!(agent.name, "research-agent");
		assert_eq!(agent.version, Some("1.0.0".to_string()));
		assert_eq!(agent.url, Some("https://agents.internal/research".to_string()));
		assert_eq!(agent.skills.len(), 1);
		assert_eq!(agent.skills[0].id, "research");
		assert!(agent.capabilities.as_ref().unwrap().streaming.unwrap());
		assert_eq!(
			agent.provider.as_ref().unwrap().organization,
			"Research Team"
		);
	}

	#[test]
	fn test_parse_tool_with_dependencies() {
		let json = r#"{
			"name": "comprehensive_search",
			"description": "Search with dependencies",
			"spec": {
				"scatterGather": {
					"targets": [
						{ "tool": "web_search" },
						{ "tool": "arxiv_search" }
					],
					"aggregation": { "ops": [{ "flatten": true }] }
				}
			},
			"tags": ["search", "research"],
			"deprecated": "Use unified_search instead",
			"depends": [
				{ "type": "tool", "name": "web_search", "version": "1.0.0" },
				{ "type": "tool", "name": "arxiv_search" },
				{ "type": "agent", "name": "summarizer", "skill": "summarize" }
			]
		}"#;

		let tool: ToolDefinition = serde_json::from_str(json).unwrap();
		assert_eq!(tool.name, "comprehensive_search");
		assert_eq!(tool.tags, vec!["search", "research"]);
		assert_eq!(tool.deprecated, Some("Use unified_search instead".to_string()));
		assert_eq!(tool.depends.len(), 3);
		assert_eq!(tool.depends[0].dep_type, DependencyType::Tool);
		assert_eq!(tool.depends[0].name, "web_search");
		assert_eq!(tool.depends[2].dep_type, DependencyType::Agent);
		assert_eq!(tool.depends[2].skill, Some("summarize".to_string()));
	}

	#[test]
	fn test_parse_source_tool_with_server_version() {
		let json = r#"{
			"name": "get_document",
			"source": {
				"target": "doc-service",
				"tool": "fetch_document",
				"serverVersion": ">=1.2.0"
			}
		}"#;

		let tool: ToolDefinition = serde_json::from_str(json).unwrap();
		let source = tool.source_tool().unwrap();
		assert_eq!(source.target, "doc-service");
		assert_eq!(source.server_version, Some(">=1.2.0".to_string()));
	}

	#[test]
	fn test_parse_deprecated_server() {
		let json = r#"{
			"name": "legacy-service",
			"version": "0.9.0",
			"deprecated": true,
			"deprecationMessage": "Migrate to new-service v2.0"
		}"#;

		let server: Server = serde_json::from_str(json).unwrap();
		assert!(server.deprecated);
		assert_eq!(
			server.deprecation_message,
			Some("Migrate to new-service v2.0".to_string())
		);
	}

	#[test]
	fn test_parse_full_v2_registry() {
		let json = r#"{
			"schemaVersion": "2.0",
			"schemas": [
				{
					"name": "SearchQuery",
					"schema": { "type": "object" }
				}
			],
			"servers": [
				{
					"name": "search-backend",
					"version": "2.0.0"
				}
			],
			"agents": [
				{
					"name": "search-agent",
					"version": "1.0.0"
				}
			],
			"tools": [
				{
					"name": "search",
					"source": {
						"target": "search-backend",
						"tool": "query",
						"serverVersion": "2.0.0"
					},
					"depends": [
						{ "type": "tool", "name": "query", "version": "2.0.0" }
					]
				}
			],
			"metadata": {
				"owner": "search-team",
				"classification": "internal"
			}
		}"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.schema_version, "2.0");
		assert_eq!(registry.schemas.len(), 1);
		assert_eq!(registry.servers.len(), 1);
		assert_eq!(registry.agents.len(), 1);
		assert_eq!(registry.tools.len(), 1);
		assert_eq!(registry.metadata.get("owner").unwrap(), "search-team");
	}
}
