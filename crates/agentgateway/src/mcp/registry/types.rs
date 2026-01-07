// Registry types for virtual tool definitions

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Parsed registry from JSON
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
	/// Schema version for compatibility
	#[serde(default = "default_schema_version")]
	pub schema_version: String,

	/// List of virtual tool definitions
	#[serde(default)]
	pub tools: Vec<VirtualToolDef>,
}

fn default_schema_version() -> String {
	"1.0".to_string()
}

/// Virtual tool definition from registry
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualToolDef {
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

	/// Output transformation schema with JSONPath mappings
	#[serde(default)]
	pub output_schema: Option<OutputSchema>,

	/// Semantic version of this tool definition
	#[serde(default)]
	pub version: Option<String>,

	/// Arbitrary metadata (owner, classification, etc.)
	#[serde(default)]
	pub metadata: HashMap<String, serde_json::Value>,
}

/// Source backend tool reference
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolSource {
	/// Target name (MCP server/backend name)
	pub target: String,

	/// Original tool name on that target
	pub tool: String,
}

/// Output transformation schema
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

/// Output field definition
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

impl Registry {
	/// Create an empty registry
	pub fn new() -> Self {
		Self::default()
	}

	/// Create a registry with the given tools
	pub fn with_tools(tools: Vec<VirtualToolDef>) -> Self {
		Self {
			schema_version: default_schema_version(),
			tools,
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

impl VirtualToolDef {
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
	fn test_parse_registry_with_version() {
		let json = r#"{
            "schemaVersion": "2.0",
            "tools": []
        }"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.schema_version, "2.0");
	}

	#[test]
	fn test_parse_simple_virtual_tool() {
		let json = r#"{
            "name": "get_weather",
            "source": {
                "target": "weather",
                "tool": "fetch_weather"
            }
        }"#;

		let tool: VirtualToolDef = serde_json::from_str(json).unwrap();
		assert_eq!(tool.name, "get_weather");
		assert_eq!(tool.source.target, "weather");
		assert_eq!(tool.source.tool, "fetch_weather");
		assert!(tool.description.is_none());
		assert!(tool.defaults.is_empty());
		assert!(tool.hide_fields.is_empty());
	}

	#[test]
	fn test_parse_full_virtual_tool() {
		let json = r#"{
            "name": "get_weather",
            "source": {
                "target": "weather",
                "tool": "fetch_weather"
            },
            "description": "Get current weather for a city",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "city": {"type": "string"}
                },
                "required": ["city"]
            },
            "defaults": {
                "api_key": "${WEATHER_API_KEY}",
                "units": "metric"
            },
            "hideFields": ["debug_mode", "raw_output"],
            "outputSchema": {
                "type": "object",
                "properties": {
                    "temperature": {
                        "type": "number",
                        "sourceField": "$.data.current.temp_f"
                    },
                    "conditions": {
                        "type": "string",
                        "sourceField": "$.data.current.condition.text"
                    }
                }
            },
            "version": "2.1.0",
            "metadata": {
                "owner": "weather-team",
                "dataClassification": "public"
            }
        }"#;

		let tool: VirtualToolDef = serde_json::from_str(json).unwrap();
		assert_eq!(tool.name, "get_weather");
		assert_eq!(tool.description, Some("Get current weather for a city".to_string()));
		assert_eq!(tool.defaults.len(), 2);
		assert_eq!(tool.defaults.get("units"), Some(&serde_json::json!("metric")));
		assert_eq!(tool.hide_fields, vec!["debug_mode", "raw_output"]);
		assert_eq!(tool.version, Some("2.1.0".to_string()));

		let output = tool.output_schema.unwrap();
		assert_eq!(output.schema_type, "object");
		assert_eq!(output.properties.len(), 2);

		let temp_field = output.properties.get("temperature").unwrap();
		assert_eq!(temp_field.field_type, "number");
		assert_eq!(temp_field.source_field, Some("$.data.current.temp_f".to_string()));
	}

	#[test]
	fn test_parse_full_registry() {
		let json = r#"{
            "schemaVersion": "1.0",
            "tools": [
                {
                    "name": "get_weather",
                    "source": {"target": "weather", "tool": "fetch_weather"}
                },
                {
                    "name": "search_web",
                    "source": {"target": "search", "tool": "web_search"},
                    "description": "Search the web"
                }
            ]
        }"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.tools.len(), 2);
		assert_eq!(registry.tools[0].name, "get_weather");
		assert_eq!(registry.tools[1].name, "search_web");
		assert_eq!(registry.tools[1].description, Some("Search the web".to_string()));
	}

	#[test]
	fn test_builder_pattern() {
		let tool = VirtualToolDef::new("my_tool", "backend", "original_tool")
			.with_description("A test tool")
			.with_default("key", serde_json::json!("value"))
			.with_hidden_fields(vec!["secret".to_string()]);

		assert_eq!(tool.name, "my_tool");
		assert_eq!(tool.source.target, "backend");
		assert_eq!(tool.source.tool, "original_tool");
		assert_eq!(tool.description, Some("A test tool".to_string()));
		assert_eq!(tool.defaults.get("key"), Some(&serde_json::json!("value")));
		assert_eq!(tool.hide_fields, vec!["secret"]);
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

	#[test]
	fn test_serialize_registry() {
		let tool = VirtualToolDef::new("get_weather", "weather", "fetch_weather")
			.with_description("Get weather");

		let registry = Registry::with_tools(vec![tool]);

		let json = serde_json::to_string_pretty(&registry).unwrap();
		assert!(json.contains("get_weather"));
		assert!(json.contains("fetch_weather"));

		// Round-trip test
		let parsed: Registry = serde_json::from_str(&json).unwrap();
		assert_eq!(parsed.tools.len(), 1);
		assert_eq!(parsed.tools[0].name, "get_weather");
	}
}
