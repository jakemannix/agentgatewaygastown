// Compiled registry ready for runtime use
//
// The compilation process resolves source chains to determine:
// - The target server for each tool
// - The original backend tool name
// - Merged defaults and hide_fields from the chain

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use rmcp::model::Tool;
use serde_json_path::JsonPath;

use super::error::RegistryError;
use super::types::{OutputSchema, Registry, ToolDef};

/// Compiled registry ready for runtime use
#[derive(Debug)]
pub struct CompiledRegistry {
	/// Original registry (for accessing servers)
	registry: Registry,
	/// Tool name -> compiled tool (both base and virtual)
	tools_by_name: HashMap<String, Arc<CompiledVirtualTool>>,
	/// (server, backend_tool_name) -> tool names (for reverse lookup from backend)
	tools_by_source: HashMap<(String, String), Vec<String>>,
}

/// Resolved information about the backend target for a tool
#[derive(Debug, Clone)]
pub struct ResolvedTarget {
	/// Server name from registry
	pub server: String,
	/// Original tool name on the backend (may differ from virtual tool name)
	pub backend_tool: String,
}

/// A compiled tool with pre-compiled JSONPath expressions and resolved target
#[derive(Debug)]
pub struct CompiledVirtualTool {
	/// Original definition
	pub def: ToolDef,
	/// Resolved backend target (server + original tool name)
	pub target: ResolvedTarget,
	/// Pre-compiled JSONPath expressions for output projection
	pub output_paths: Option<HashMap<String, CompiledOutputField>>,
	/// Merged schema (source schema with hideFields applied)
	pub effective_schema: Option<serde_json::Value>,
	/// Merged defaults from source chain
	pub merged_defaults: HashMap<String, serde_json::Value>,
	/// Merged hide_fields from source chain
	pub merged_hide_fields: Vec<String>,
}

/// Compiled output field with pre-compiled JSONPath
#[derive(Debug)]
pub struct CompiledOutputField {
	/// The field type from the schema
	pub field_type: String,
	/// Pre-compiled JSONPath expression (None if passthrough)
	pub jsonpath: Option<JsonPath>,
	/// Original path string for error messages
	pub source_path: Option<String>,
}

impl CompiledRegistry {
	/// Compile a registry from its raw definition
	pub fn compile(registry: Registry) -> Result<Self, RegistryError> {
		let mut tools_by_name = HashMap::new();
		let mut tools_by_source: HashMap<(String, String), Vec<String>> = HashMap::new();

		// Build a map for source chain resolution
		let tools_map: HashMap<&str, &ToolDef> =
			registry.tools.iter().map(|t| (t.name.as_str(), t)).collect();

		for tool_def in &registry.tools {
			let compiled = CompiledVirtualTool::compile(tool_def.clone(), &tools_map)?;
			let name = tool_def.name.clone();

			// Index by resolved target for reverse lookup
			let source_key = (
				compiled.target.server.clone(),
				compiled.target.backend_tool.clone(),
			);
			tools_by_source
				.entry(source_key)
				.or_default()
				.push(name.clone());

			tools_by_name.insert(name, Arc::new(compiled));
		}

		Ok(Self {
			registry,
			tools_by_name,
			tools_by_source,
		})
	}

	/// Create an empty compiled registry
	pub fn empty() -> Self {
		Self {
			registry: Registry::new(),
			tools_by_name: HashMap::new(),
			tools_by_source: HashMap::new(),
		}
	}

	/// Get the original registry (for accessing servers)
	pub fn registry(&self) -> &Registry {
		&self.registry
	}

	/// Look up virtual tool by exposed name
	pub fn get_tool(&self, name: &str) -> Option<&Arc<CompiledVirtualTool>> {
		self.tools_by_name.get(name)
	}

	/// Check if a backend tool is virtualized
	pub fn is_virtualized(&self, target: &str, tool: &str) -> bool {
		self.tools_by_source
			.contains_key(&(target.to_string(), tool.to_string()))
	}

	/// Get virtual tool names for a given source tool
	pub fn get_virtual_names(&self, target: &str, tool: &str) -> Option<&Vec<String>> {
		self.tools_by_source
			.get(&(target.to_string(), tool.to_string()))
	}

	/// Transform backend tool list to virtual tool list
	///
	/// This replaces source tools with their virtual counterparts and passes through
	/// non-virtualized tools unchanged.
	pub fn transform_tools(&self, backend_tools: Vec<(String, Tool)>) -> Vec<(String, Tool)> {
		let mut result = Vec::new();
		let mut virtualized_sources: std::collections::HashSet<(String, String)> =
			std::collections::HashSet::new();

		// First, add all virtual tools that have matching sources
		for ((target, source_tool), virtual_names) in &self.tools_by_source {
			// Find the source tool in backend_tools
			let source = backend_tools
				.iter()
				.find(|(t, tool)| t == target && tool.name.as_ref() == source_tool);

			if let Some((_, source_tool_def)) = source {
				virtualized_sources.insert((target.clone(), source_tool.clone()));

				// Create virtual tools from this source
				for vname in virtual_names {
					if let Some(compiled) = self.tools_by_name.get(vname) {
						let virtual_tool = compiled.create_virtual_tool(source_tool_def);
						result.push((target.clone(), virtual_tool));
					}
				}
			}
		}

		// Pass through non-virtualized tools
		for (target, tool) in backend_tools {
			let source_key = (target.clone(), tool.name.to_string());
			if !virtualized_sources.contains(&source_key) {
				result.push((target, tool));
			}
		}

		result
	}

	/// Prepare arguments for backend call (inject defaults, resolve env vars)
	///
	/// Returns (server, backend_tool_name, transformed_args)
	pub fn prepare_call_args(
		&self,
		tool_name: &str,
		args: serde_json::Value,
	) -> Result<(String, String, serde_json::Value), RegistryError> {
		let tool = self
			.get_tool(tool_name)
			.ok_or_else(|| RegistryError::tool_not_found(tool_name))?;

		let server = tool.target.server.clone();
		let backend_tool = tool.target.backend_tool.clone();
		let transformed_args = tool.inject_defaults(args)?;

		Ok((server, backend_tool, transformed_args))
	}

	/// Transform backend response to virtual response
	pub fn transform_output(
		&self,
		virtual_name: &str,
		response: serde_json::Value,
	) -> Result<serde_json::Value, RegistryError> {
		let tool = self
			.get_tool(virtual_name)
			.ok_or_else(|| RegistryError::tool_not_found(virtual_name))?;

		tool.transform_output(response)
	}

	/// Get all virtual tool names
	pub fn tool_names(&self) -> impl Iterator<Item = &String> {
		self.tools_by_name.keys()
	}

	/// Get number of virtual tools
	pub fn len(&self) -> usize {
		self.tools_by_name.len()
	}

	/// Check if registry is empty
	pub fn is_empty(&self) -> bool {
		self.tools_by_name.is_empty()
	}
}

impl CompiledVirtualTool {
	/// Compile a tool definition, resolving source chains
	pub fn compile(
		def: ToolDef,
		tools_map: &HashMap<&str, &ToolDef>,
	) -> Result<Self, RegistryError> {
		// Resolve the source chain to find server and backend tool name
		let (target, merged_defaults, merged_hide_fields) =
			Self::resolve_source_chain(&def, tools_map)?;

		let output_paths = if let Some(ref schema) = def.output_schema {
			Some(Self::compile_output_schema(schema)?)
		} else {
			None
		};

		Ok(Self {
			def,
			target,
			output_paths,
			effective_schema: None, // Computed lazily when source schema is known
			merged_defaults,
			merged_hide_fields,
		})
	}

	/// Resolve the source chain to find the target server and backend tool
	fn resolve_source_chain(
		def: &ToolDef,
		tools_map: &HashMap<&str, &ToolDef>,
	) -> Result<(ResolvedTarget, HashMap<String, serde_json::Value>, Vec<String>), RegistryError> {
		let mut merged_defaults = def.defaults.clone();
		let mut merged_hide_fields = def.hide_fields.clone();

		// If this is a base tool (has server), we're done
		if let Some(ref server) = def.server {
			let backend_tool = def.original_name.clone().unwrap_or_else(|| def.name.clone());
			return Ok((
				ResolvedTarget {
					server: server.clone(),
					backend_tool,
				},
				merged_defaults,
				merged_hide_fields,
			));
		}

		// Follow the source chain
		let Some(ref source_name) = def.source else {
			return Err(RegistryError::SourceResolution(format!(
				"Tool '{}' has neither 'server' nor 'source' field",
				def.name
			)));
		};

		let mut current_name = source_name.as_str();
		let mut visited = std::collections::HashSet::new();

		loop {
			if visited.contains(current_name) {
				return Err(RegistryError::SourceResolution(format!(
					"Circular source reference detected at '{}'",
					current_name
				)));
			}
			visited.insert(current_name);

			let Some(source_def) = tools_map.get(current_name) else {
				return Err(RegistryError::SourceResolution(format!(
					"Tool '{}' references unknown source '{}'",
					def.name, current_name
				)));
			};

			// Merge defaults (source defaults are lower priority, only add if not present)
			for (key, value) in &source_def.defaults {
				merged_defaults.entry(key.clone()).or_insert(value.clone());
			}

			// Merge hide_fields
			for field in &source_def.hide_fields {
				if !merged_hide_fields.contains(field) {
					merged_hide_fields.push(field.clone());
				}
			}

			// If source has a server, we found the base
			if let Some(ref server) = source_def.server {
				let backend_tool = source_def
					.original_name
					.clone()
					.unwrap_or_else(|| source_def.name.clone());
				return Ok((
					ResolvedTarget {
						server: server.clone(),
						backend_tool,
					},
					merged_defaults,
					merged_hide_fields,
				));
			}

			// Continue following the chain
			let Some(ref next_source) = source_def.source else {
				return Err(RegistryError::SourceResolution(format!(
					"Source '{}' has neither 'server' nor 'source' field",
					current_name
				)));
			};
			current_name = next_source.as_str();
		}
	}

	/// Compile output schema JSONPath expressions
	fn compile_output_schema(
		schema: &OutputSchema,
	) -> Result<HashMap<String, CompiledOutputField>, RegistryError> {
		let mut paths = HashMap::new();

		for (field_name, field_def) in &schema.properties {
			let jsonpath = if let Some(ref path) = field_def.source_field {
				Some(
					JsonPath::parse(path)
						.map_err(|e| RegistryError::invalid_jsonpath(path, e.to_string()))?,
				)
			} else {
				None
			};

			paths.insert(
				field_name.clone(),
				CompiledOutputField {
					field_type: field_def.field_type.clone(),
					jsonpath,
					source_path: field_def.source_field.clone(),
				},
			);
		}

		Ok(paths)
	}

	/// Create a virtual tool from a source tool definition
	pub fn create_virtual_tool(&self, source: &Tool) -> Tool {
		Tool {
			name: Cow::Owned(self.def.name.clone()),
			title: source.title.clone(),
			description: self
				.def
				.description
				.clone()
				.map(Cow::Owned)
				.or_else(|| source.description.clone()),
			input_schema: self.compute_effective_schema(source),
			output_schema: source.output_schema.clone(),
			annotations: source.annotations.clone(),
			icons: source.icons.clone(),
			meta: source.meta.clone(),
		}
	}

	/// Compute effective input schema by applying hideFields to source schema
	fn compute_effective_schema(
		&self,
		source: &Tool,
	) -> Arc<serde_json::Map<String, serde_json::Value>> {
		// If we have a complete override schema, use it
		if let Some(ref override_schema) = self.def.input_schema {
			if let Some(obj) = override_schema.as_object() {
				return Arc::new(obj.clone());
			}
		}

		// Start with source schema (clone the inner Map)
		let mut schema: serde_json::Map<String, serde_json::Value> =
			source.input_schema.as_ref().clone();

		// Apply merged_hide_fields (from entire source chain)
		if !self.merged_hide_fields.is_empty() {
			if let Some(props) = schema.get_mut("properties") {
				if let Some(obj) = props.as_object_mut() {
					for field in &self.merged_hide_fields {
						obj.remove(field);
					}
				}
			}
			// Also remove from required array
			if let Some(required) = schema.get_mut("required") {
				if let Some(arr) = required.as_array_mut() {
					arr.retain(|v| {
						v.as_str()
							.map(|s| !self.merged_hide_fields.contains(&s.to_string()))
							.unwrap_or(true)
					});
				}
			}
		}

		Arc::new(schema)
	}

	/// Inject default values into arguments (uses merged defaults from source chain)
	pub fn inject_defaults(
		&self,
		mut args: serde_json::Value,
	) -> Result<serde_json::Value, RegistryError> {
		if self.merged_defaults.is_empty() {
			return Ok(args);
		}

		let obj = args
			.as_object_mut()
			.ok_or_else(|| RegistryError::SchemaValidation("arguments must be an object".into()))?;

		for (key, value) in &self.merged_defaults {
			// Don't override if already provided
			if obj.contains_key(key) {
				continue;
			}

			// Resolve environment variables in string values
			let resolved_value = self.resolve_env_vars(value)?;
			obj.insert(key.clone(), resolved_value);
		}

		Ok(args)
	}

	/// Resolve ${ENV_VAR} patterns in a JSON value
	fn resolve_env_vars(&self, value: &serde_json::Value) -> Result<serde_json::Value, RegistryError> {
		match value {
			serde_json::Value::String(s) => {
				let resolved = self.resolve_env_string(s)?;
				Ok(serde_json::Value::String(resolved))
			},
			serde_json::Value::Object(obj) => {
				let mut new_obj = serde_json::Map::new();
				for (k, v) in obj {
					new_obj.insert(k.clone(), self.resolve_env_vars(v)?);
				}
				Ok(serde_json::Value::Object(new_obj))
			},
			serde_json::Value::Array(arr) => {
				let new_arr: Result<Vec<_>, _> = arr.iter().map(|v| self.resolve_env_vars(v)).collect();
				Ok(serde_json::Value::Array(new_arr?))
			},
			// Other types pass through unchanged
			other => Ok(other.clone()),
		}
	}

	/// Resolve ${ENV_VAR} patterns in a string
	fn resolve_env_string(&self, s: &str) -> Result<String, RegistryError> {
		let mut result = s.to_string();
		let re = regex::Regex::new(r"\$\{([^}]+)\}").expect("valid regex");

		for cap in re.captures_iter(s) {
			let var_name = &cap[1];
			let value = std::env::var(var_name)
				.map_err(|_| RegistryError::EnvVarNotFound { name: var_name.to_string() })?;
			result = result.replace(&cap[0], &value);
		}

		Ok(result)
	}

	/// Transform backend output using JSONPath projections
	pub fn transform_output(
		&self,
		response: serde_json::Value,
	) -> Result<serde_json::Value, RegistryError> {
		let Some(ref output_paths) = self.output_paths else {
			// No output transformation, pass through
			return Ok(response);
		};

		// First, try to extract JSON if the response is text
		let json_response = self.extract_json_from_response(&response)?;

		let mut result = serde_json::Map::new();

		for (field_name, field) in output_paths {
			let Some(ref jsonpath) = field.jsonpath else {
				// No JSONPath, try to get field directly from response
				if let Some(value) = json_response.get(field_name) {
					result.insert(field_name.clone(), value.clone());
				}
				continue;
			};

			// Query using JSONPath
			let nodes = jsonpath.query(&json_response);
			let node_vec: Vec<serde_json::Value> =
				nodes.iter().map(|v| (*v).clone()).collect();
			let value = if node_vec.is_empty() {
				serde_json::Value::Null
			} else if node_vec.len() == 1 {
				node_vec.into_iter().next().unwrap()
			} else {
				// Multiple matches - return as array
				serde_json::Value::Array(node_vec)
			};

			result.insert(field_name.clone(), value);
		}

		Ok(serde_json::Value::Object(result))
	}

	/// Extract JSON from response (handles JSON embedded in text)
	fn extract_json_from_response(
		&self,
		response: &serde_json::Value,
	) -> Result<serde_json::Value, RegistryError> {
		match response {
			// Already JSON object or array
			serde_json::Value::Object(_) | serde_json::Value::Array(_) => Ok(response.clone()),

			// Try to parse JSON from string
			serde_json::Value::String(s) => {
				// First try to parse the whole string as JSON
				if let Ok(json) = serde_json::from_str(s) {
					return Ok(json);
				}

				// Try to find JSON object or array in the text
				if let Some(json) = Self::find_json_in_text(s) {
					return Ok(json);
				}

				// Return as-is if no JSON found
				Ok(response.clone())
			},

			// Other types pass through
			other => Ok(other.clone()),
		}
	}

	/// Find JSON object or array embedded in text
	fn find_json_in_text(text: &str) -> Option<serde_json::Value> {
		// Look for JSON object
		if let Some(start) = text.find('{') {
			if let Some(json) = Self::try_parse_json_from(&text[start..], '{', '}') {
				return Some(json);
			}
		}

		// Look for JSON array
		if let Some(start) = text.find('[') {
			if let Some(json) = Self::try_parse_json_from(&text[start..], '[', ']') {
				return Some(json);
			}
		}

		None
	}

	/// Try to parse JSON starting from a given position
	fn try_parse_json_from(text: &str, open: char, close: char) -> Option<serde_json::Value> {
		let mut depth = 0;
		let mut end_pos = 0;
		let mut in_string = false;
		let mut escape_next = false;

		for (i, c) in text.char_indices() {
			if escape_next {
				escape_next = false;
				continue;
			}

			if c == '\\' && in_string {
				escape_next = true;
				continue;
			}

			if c == '"' {
				in_string = !in_string;
				continue;
			}

			if in_string {
				continue;
			}

			if c == open {
				depth += 1;
			} else if c == close {
				depth -= 1;
				if depth == 0 {
					end_pos = i + 1;
					break;
				}
			}
		}

		if end_pos > 0 {
			serde_json::from_str(&text[..end_pos]).ok()
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;
	use crate::mcp::registry::types::{OutputField, ServerDef, ToolDef};

	fn create_source_tool(name: &str, description: &str) -> Tool {
		let schema: serde_json::Map<String, serde_json::Value> = serde_json::from_value(json!({
			"type": "object",
			"properties": {
				"city": {"type": "string"},
				"units": {"type": "string"},
				"debug_mode": {"type": "boolean"}
			},
			"required": ["city"]
		}))
		.unwrap();

		Tool {
			name: Cow::Owned(name.to_string()),
			title: None,
			description: Some(Cow::Owned(description.to_string())),
			input_schema: Arc::new(schema),
			output_schema: None,
			annotations: None,
			icons: None,
			meta: None,
		}
	}

	fn create_test_registry() -> Registry {
		let servers = vec![ServerDef::stdio("weather", "weather-cli", vec![])];
		let tools = vec![
			ToolDef::base("fetch_weather", "weather"),
			ToolDef::virtual_tool("get_weather", "fetch_weather")
				.with_description("Get weather info"),
		];
		Registry::with_servers_and_tools(servers, tools)
	}

	#[test]
	fn test_compile_empty_registry() {
		let registry = Registry::new();
		let compiled = CompiledRegistry::compile(registry).unwrap();
		assert!(compiled.is_empty());
		assert_eq!(compiled.len(), 0);
	}

	#[test]
	fn test_compile_simple_registry() {
		let registry = create_test_registry();
		let compiled = CompiledRegistry::compile(registry).unwrap();

		// Both base and virtual tools are compiled
		assert_eq!(compiled.len(), 2);
		assert!(compiled.get_tool("fetch_weather").is_some());
		assert!(compiled.get_tool("get_weather").is_some());
		assert!(compiled.get_tool("nonexistent").is_none());
	}

	#[test]
	fn test_resolved_target() {
		let registry = create_test_registry();
		let compiled = CompiledRegistry::compile(registry).unwrap();

		// Base tool resolves to itself
		let base = compiled.get_tool("fetch_weather").unwrap();
		assert_eq!(base.target.server, "weather");
		assert_eq!(base.target.backend_tool, "fetch_weather");

		// Virtual tool resolves to base
		let virtual_tool = compiled.get_tool("get_weather").unwrap();
		assert_eq!(virtual_tool.target.server, "weather");
		assert_eq!(virtual_tool.target.backend_tool, "fetch_weather");
	}

	#[test]
	fn test_is_virtualized() {
		let registry = create_test_registry();
		let compiled = CompiledRegistry::compile(registry).unwrap();

		// Both tools map to (weather, fetch_weather)
		assert!(compiled.is_virtualized("weather", "fetch_weather"));
		assert!(!compiled.is_virtualized("weather", "other_tool"));
		assert!(!compiled.is_virtualized("other_backend", "fetch_weather"));
	}

	#[test]
	fn test_transform_tools_replaces_virtualized() {
		let registry = create_test_registry();
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let source_tool = create_source_tool("fetch_weather", "Original description");
		let backend_tools = vec![("weather".to_string(), source_tool)];

		let result = compiled.transform_tools(backend_tools);

		// Both base and virtual tools should be in the result
		assert_eq!(result.len(), 2);
		let names: Vec<_> = result.iter().map(|(_, t)| t.name.as_ref()).collect();
		assert!(names.contains(&"fetch_weather"));
		assert!(names.contains(&"get_weather"));
	}

	#[test]
	fn test_transform_tools_passthrough_non_virtualized() {
		let registry = create_test_registry();
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let source_tool = create_source_tool("fetch_weather", "Weather");
		let other_tool = create_source_tool("other_tool", "Other");
		let backend_tools = vec![
			("weather".to_string(), source_tool),
			("weather".to_string(), other_tool),
		];

		let result = compiled.transform_tools(backend_tools);

		// other_tool passes through, plus the virtualized ones
		assert_eq!(result.len(), 3);
		let names: Vec<_> = result.iter().map(|(_, t)| t.name.as_ref()).collect();
		assert!(names.contains(&"fetch_weather"));
		assert!(names.contains(&"get_weather"));
		assert!(names.contains(&"other_tool"));
	}

	#[test]
	fn test_hide_fields_in_schema() {
		let servers = vec![ServerDef::stdio("weather", "cmd", vec![])];
		let tools = vec![
			ToolDef::base("fetch_weather", "weather"),
			ToolDef::virtual_tool("get_weather", "fetch_weather")
				.with_hidden_fields(vec!["debug_mode".to_string()]),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let tool = compiled.get_tool("get_weather").unwrap();
		let source = create_source_tool("fetch_weather", "Weather");
		let virtual_tool = tool.create_virtual_tool(&source);

		let props = virtual_tool.input_schema.get("properties").unwrap();
		assert!(props.get("city").is_some());
		assert!(props.get("units").is_some());
		assert!(props.get("debug_mode").is_none());
	}

	#[test]
	fn test_inject_defaults() {
		let servers = vec![ServerDef::stdio("weather", "cmd", vec![])];
		let tools = vec![
			ToolDef::base("fetch_weather", "weather"),
			ToolDef::virtual_tool("get_weather", "fetch_weather")
				.with_default("units", json!("metric"))
				.with_default("format", json!("json")),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let tool = compiled.get_tool("get_weather").unwrap();
		let args = json!({"city": "Seattle"});
		let result = tool.inject_defaults(args).unwrap();

		assert_eq!(result["city"], "Seattle");
		assert_eq!(result["units"], "metric");
		assert_eq!(result["format"], "json");
	}

	#[test]
	fn test_inject_defaults_does_not_override() {
		let servers = vec![ServerDef::stdio("weather", "cmd", vec![])];
		let tools = vec![
			ToolDef::base("fetch_weather", "weather"),
			ToolDef::virtual_tool("get_weather", "fetch_weather")
				.with_default("units", json!("metric")),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let tool = compiled.get_tool("get_weather").unwrap();
		let args = json!({"city": "Seattle", "units": "imperial"});
		let result = tool.inject_defaults(args).unwrap();

		assert_eq!(result["units"], "imperial");
	}

	#[test]
	fn test_inject_defaults_with_env_var() {
		// SAFETY: This test runs in isolation and only modifies a test-specific env var
		unsafe {
			std::env::set_var("TEST_API_KEY", "secret123");
		}

		let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
		let mut tool = ToolDef::base("tool", "backend");
		tool.defaults
			.insert("api_key".to_string(), json!("${TEST_API_KEY}"));
		let registry = Registry::with_servers_and_tools(servers, vec![tool]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let tool = compiled.get_tool("tool").unwrap();
		let args = json!({});
		let result = tool.inject_defaults(args).unwrap();

		assert_eq!(result["api_key"], "secret123");

		// SAFETY: This test runs in isolation and only modifies a test-specific env var
		unsafe {
			std::env::remove_var("TEST_API_KEY");
		}
	}

	#[test]
	fn test_output_transformation_simple() {
		let mut props = HashMap::new();
		props.insert("temp".to_string(), OutputField::new("number", "$.temperature"));
		props.insert("city".to_string(), OutputField::new("string", "$.location.city"));

		let output_schema = OutputSchema::new(props);
		let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
		let tools = vec![
			ToolDef::base("tool", "backend").with_output_schema(output_schema),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let tool = compiled.get_tool("tool").unwrap();
		let response = json!({
			"temperature": 72.5,
			"location": {
				"city": "Seattle",
				"state": "WA"
			}
		});

		let result = tool.transform_output(response).unwrap();

		assert_eq!(result["temp"], 72.5);
		assert_eq!(result["city"], "Seattle");
	}

	#[test]
	fn test_output_transformation_nested_path() {
		let mut props = HashMap::new();
		props.insert(
			"temperature".to_string(),
			OutputField::new("number", "$.data.current.temp_f"),
		);
		props.insert(
			"conditions".to_string(),
			OutputField::new("string", "$.data.current.condition.text"),
		);

		let output_schema = OutputSchema::new(props);
		let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
		let tools = vec![
			ToolDef::base("tool", "backend").with_output_schema(output_schema),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let tool = compiled.get_tool("tool").unwrap();
		let response = json!({
			"data": {
				"current": {
					"temp_f": 52.3,
					"condition": {
						"text": "Cloudy"
					}
				}
			}
		});

		let result = tool.transform_output(response).unwrap();

		assert_eq!(result["temperature"], 52.3);
		assert_eq!(result["conditions"], "Cloudy");
	}

	#[test]
	fn test_output_transformation_no_schema() {
		let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
		let tools = vec![ToolDef::base("tool", "backend")];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let tool = compiled.get_tool("tool").unwrap();
		let response = json!({"original": "data"});
		let result = tool.transform_output(response.clone()).unwrap();

		assert_eq!(result, response);
	}

	#[test]
	fn test_extract_json_from_text() {
		let text = r#"Here is the result: {"temperature": 72.5, "city": "Seattle"} and some more text"#;
		let json = CompiledVirtualTool::find_json_in_text(text).unwrap();

		assert_eq!(json["temperature"], 72.5);
		assert_eq!(json["city"], "Seattle");
	}

	#[test]
	fn test_extract_json_array_from_text() {
		let text = r#"Results: [1, 2, 3] done"#;
		let json = CompiledVirtualTool::find_json_in_text(text).unwrap();

		assert!(json.is_array());
		assert_eq!(json.as_array().unwrap().len(), 3);
	}

	#[test]
	fn test_prepare_call_args() {
		let servers = vec![ServerDef::stdio("weather", "cmd", vec![])];
		let tools = vec![
			ToolDef::base("fetch_weather", "weather"),
			ToolDef::virtual_tool("get_weather", "fetch_weather")
				.with_default("units", json!("metric")),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let args = json!({"city": "Seattle"});
		let (server, backend_tool, transformed) =
			compiled.prepare_call_args("get_weather", args).unwrap();

		assert_eq!(server, "weather");
		assert_eq!(backend_tool, "fetch_weather");
		assert_eq!(transformed["city"], "Seattle");
		assert_eq!(transformed["units"], "metric");
	}

	#[test]
	fn test_prepare_call_args_unknown_tool() {
		let registry = Registry::new();
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let result = compiled.prepare_call_args("unknown", json!({}));
		assert!(result.is_err());
	}

	#[test]
	fn test_multiple_virtual_tools_same_source() {
		let servers = vec![ServerDef::stdio("weather", "cmd", vec![])];
		let tools = vec![
			ToolDef::base("fetch_weather", "weather"),
			ToolDef::virtual_tool("weather_metric", "fetch_weather")
				.with_default("units", json!("metric")),
			ToolDef::virtual_tool("weather_imperial", "fetch_weather")
				.with_default("units", json!("imperial")),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let names = compiled
			.get_virtual_names("weather", "fetch_weather")
			.unwrap();
		// Base tool + 2 virtual tools
		assert_eq!(names.len(), 3);
		assert!(names.contains(&"fetch_weather".to_string()));
		assert!(names.contains(&"weather_metric".to_string()));
		assert!(names.contains(&"weather_imperial".to_string()));
	}

	#[test]
	fn test_source_chain_resolution() {
		// Test multi-level source chain: level2 -> level1 -> base
		let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
		let tools = vec![
			ToolDef::base("base_tool", "backend").with_default("a", json!("from_base")),
			ToolDef::virtual_tool("level1", "base_tool").with_default("b", json!("from_level1")),
			ToolDef::virtual_tool("level2", "level1").with_default("c", json!("from_level2")),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let tool = compiled.get_tool("level2").unwrap();

		// Should resolve to the base server/tool
		assert_eq!(tool.target.server, "backend");
		assert_eq!(tool.target.backend_tool, "base_tool");

		// Merged defaults should include all levels
		let args = json!({});
		let result = tool.inject_defaults(args).unwrap();
		assert_eq!(result["a"], "from_base");
		assert_eq!(result["b"], "from_level1");
		assert_eq!(result["c"], "from_level2");
	}

	#[test]
	fn test_circular_reference_detection() {
		let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
		let tools = vec![
			ToolDef::virtual_tool("tool_a", "tool_b"),
			ToolDef::virtual_tool("tool_b", "tool_a"),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let result = CompiledRegistry::compile(registry);

		assert!(result.is_err());
		let err = result.unwrap_err();
		assert!(err.to_string().contains("Circular"));
	}

	#[test]
	fn test_missing_source_detection() {
		let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
		let tools = vec![ToolDef::virtual_tool("orphan", "nonexistent")];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let result = CompiledRegistry::compile(registry);

		assert!(result.is_err());
		let err = result.unwrap_err();
		assert!(err.to_string().contains("nonexistent"));
	}

	#[test]
	fn test_original_name_resolution() {
		let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
		let tools = vec![
			ToolDef::base("local_name", "backend").with_original_name("backend_name"),
			ToolDef::virtual_tool("alias", "local_name"),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		// Both should resolve to backend_name
		let base = compiled.get_tool("local_name").unwrap();
		assert_eq!(base.target.backend_tool, "backend_name");

		let alias = compiled.get_tool("alias").unwrap();
		assert_eq!(alias.target.backend_tool, "backend_name");
	}
}
