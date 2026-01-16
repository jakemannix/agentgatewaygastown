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
	/// Schema version for compatibility
	#[serde(default = "default_schema_version")]
	pub schema_version: String,

	/// List of tool definitions (virtual tools and compositions)
	#[serde(default)]
	pub tools: Vec<ToolDefinition>,
}

fn default_schema_version() -> String {
	"1.0".to_string()
}

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
	pub target: String,

	/// Original tool name on that target
	pub tool: String,

	/// Fields to inject at call time (supports ${ENV_VAR} substitution)
	#[serde(default)]
	pub defaults: HashMap<String, serde_json::Value>,

	/// Fields to remove from schema (hidden from agents)
	#[serde(default)]
	pub hide_fields: Vec<String>,
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
		Self { mappings: schema_map.mappings }
	}

	/// Create an empty output transform
	pub fn empty() -> Self {
		Self { mappings: HashMap::new() }
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
		Self { schema_version: default_schema_version(), tools }
	}

	/// Create a registry with legacy virtual tool definitions
	pub fn with_tools(tools: Vec<VirtualToolDef>) -> Self {
		Self {
			schema_version: default_schema_version(),
			tools: tools.into_iter().map(ToolDefinition::from_legacy).collect(),
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
			}),
			input_schema: None,
			output_transform: None,
			output_schema: None,
			version: None,
			metadata: HashMap::new(),
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
			}),
			input_schema: legacy.input_schema,
			output_transform,
			output_schema: None,
			version: legacy.version,
			metadata: legacy.metadata,
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
	pub fn new(
		name: impl Into<String>,
		target: impl Into<String>,
		tool: impl Into<String>,
	) -> Self {
		Self {
			name: name.into(),
			source: ToolSource { target: target.into(), tool: tool.into() },
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
		Self { schema_type: default_object_type(), properties }
	}
}

impl OutputField {
	/// Create a new output field with JSONPath source
	pub fn new(field_type: impl Into<String>, source_field: impl Into<String>) -> Self {
		Self { field_type: field_type.into(), source_field: Some(source_field.into()), description: None }
	}

	/// Create a field without JSONPath (passthrough)
	pub fn passthrough(field_type: impl Into<String>) -> Self {
		Self { field_type: field_type.into(), source_field: None, description: None }
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
		assert_eq!(source.defaults.get("units"), Some(&serde_json::json!("metric")));
	}

	#[test]
	fn test_builder_source_tool() {
		let tool = ToolDefinition::source("my_tool", "backend", "original")
			.with_description("A test tool");

		assert_eq!(tool.name, "my_tool");
		assert_eq!(tool.description, Some("A test tool".to_string()));
	}

	#[test]
	fn test_builder_composition_tool() {
		use super::super::patterns::{PipelineSpec, PipelineStep, StepOperation, ToolCall};

		let spec = PatternSpec::Pipeline(PipelineSpec {
			steps: vec![PipelineStep {
				id: "step1".to_string(),
				operation: StepOperation::Tool(ToolCall { name: "search".to_string() }),
				input: None,
			}],
		});

		let tool = ToolDefinition::composition("my_composition", spec)
			.with_description("A composition");

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
}
