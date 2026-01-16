// Compiled registry ready for runtime use
//
// Supports both source-based tools (1:1 virtual tools) and composition-based tools (N:1).
// Uses two-pass compilation for order-independent reference resolution.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use rmcp::model::Tool;
use serde_json_path::JsonPath;

use super::error::RegistryError;
use super::patterns::{FieldSource, PatternSpec};
use super::types::{
	OutputTransform, Registry, SourceTool, ToolDefinition, ToolImplementation,
	VirtualToolDef,
};

/// Maximum depth for reference resolution (safety limit)
const MAX_REFERENCE_DEPTH: usize = 100;

/// Compiled registry ready for runtime use
#[derive(Debug)]
pub struct CompiledRegistry {
	/// Tool name -> compiled tool
	tools_by_name: HashMap<String, Arc<CompiledTool>>,
	/// (target, source_tool) -> virtual tool names (for reverse lookup, source tools only)
	tools_by_source: HashMap<(String, String), Vec<String>>,
}

/// A compiled tool - either a source-based tool or a composition
#[derive(Debug)]
pub struct CompiledTool {
	/// Original definition
	pub def: ToolDefinition,
	/// Compiled form based on implementation type
	pub compiled: CompiledImplementation,
}

/// Compiled implementation
#[derive(Debug)]
pub enum CompiledImplementation {
	/// Source-based tool (1:1 mapping)
	Source(CompiledSourceTool),
	/// Composition (N:1 orchestration)
	Composition(CompiledComposition),
}

/// Compiled source-based (virtual) tool
#[derive(Debug)]
pub struct CompiledSourceTool {
	/// Source tool reference
	pub source: SourceTool,
	/// Pre-compiled output transform
	pub output_transform: Option<CompiledOutputTransform>,
	/// Merged schema (source schema with hideFields applied)
	pub effective_schema: Option<serde_json::Value>,
}

/// Compiled composition
#[derive(Debug)]
pub struct CompiledComposition {
	/// The pattern spec
	pub spec: PatternSpec,
	/// Pre-compiled output transform
	pub output_transform: Option<CompiledOutputTransform>,
	/// Resolved tool references (name -> index in registry)
	pub resolved_references: Vec<String>,
}

/// Compiled output transform with pre-compiled JSONPath expressions
#[derive(Debug)]
pub struct CompiledOutputTransform {
	/// Field name -> compiled field source
	pub fields: HashMap<String, CompiledFieldSource>,
}

/// Compiled field source
#[derive(Debug)]
pub enum CompiledFieldSource {
	/// JSONPath extraction
	Path { jsonpath: JsonPath, original: String },
	/// Literal value
	Literal(serde_json::Value),
	/// Coalesce: first non-null from paths
	Coalesce { paths: Vec<JsonPath>, originals: Vec<String> },
	/// Template interpolation
	Template { template: String, vars: HashMap<String, JsonPath> },
	/// Concatenation
	Concat { paths: Vec<JsonPath>, separator: String },
	/// Nested mapping
	Nested(Box<CompiledOutputTransform>),
}

// =============================================================================
// Legacy compatibility alias
// =============================================================================

/// Legacy type alias for backward compatibility
pub type CompiledVirtualTool = CompiledTool;

/// Compiled output field (legacy format)
#[derive(Debug)]
pub struct CompiledOutputField {
	/// The field type from the schema
	pub field_type: String,
	/// Pre-compiled JSONPath expression (None if passthrough)
	pub jsonpath: Option<JsonPath>,
	/// Original path string for error messages
	pub source_path: Option<String>,
}

// =============================================================================
// CompiledRegistry Implementation
// =============================================================================

impl CompiledRegistry {
	/// Compile a registry from its raw definition using two-pass compilation
	///
	/// Pass 1: Index all tools by name (order-independent)
	/// Pass 2: Compile each tool, resolving references
	pub fn compile(registry: Registry) -> Result<Self, RegistryError> {
		// Pass 1: Index all definitions by name
		let mut defs_by_name: HashMap<String, ToolDefinition> = HashMap::new();
		for tool_def in registry.tools {
			if defs_by_name.contains_key(&tool_def.name) {
				return Err(RegistryError::DuplicateToolName(tool_def.name.clone()));
			}
			defs_by_name.insert(tool_def.name.clone(), tool_def);
		}

		// Pass 2: Compile each tool
		let mut tools_by_name: HashMap<String, Arc<CompiledTool>> = HashMap::new();
		let mut tools_by_source: HashMap<(String, String), Vec<String>> = HashMap::new();

		for (name, def) in &defs_by_name {
			let compiled = CompiledTool::compile(def, &defs_by_name, 0)?;

			// Index source-based tools by their source for reverse lookup
			if let ToolImplementation::Source(ref source) = def.implementation {
				let source_key = (source.target.clone(), source.tool.clone());
				tools_by_source.entry(source_key).or_default().push(name.clone());
			}

			tools_by_name.insert(name.clone(), Arc::new(compiled));
		}

		Ok(Self { tools_by_name, tools_by_source })
	}

	/// Create an empty compiled registry
	pub fn empty() -> Self {
		Self { tools_by_name: HashMap::new(), tools_by_source: HashMap::new() }
	}

	/// Look up tool by name
	pub fn get_tool(&self, name: &str) -> Option<&Arc<CompiledTool>> {
		self.tools_by_name.get(name)
	}

	/// Check if a tool is a composition
	pub fn is_composition(&self, name: &str) -> bool {
		self.tools_by_name.get(name).map(|t| t.is_composition()).unwrap_or(false)
	}

	/// Check if a tool is a source-based (virtual) tool
	pub fn is_source_tool(&self, name: &str) -> bool {
		self.tools_by_name.get(name).map(|t| t.is_source()).unwrap_or(false)
	}

	/// Check if a backend tool is virtualized
	pub fn is_virtualized(&self, target: &str, tool: &str) -> bool {
		self.tools_by_source.contains_key(&(target.to_string(), tool.to_string()))
	}

	/// Get virtual tool names for a given source tool
	pub fn get_virtual_names(&self, target: &str, tool: &str) -> Option<&Vec<String>> {
		self.tools_by_source.get(&(target.to_string(), tool.to_string()))
	}

	/// Transform backend tool list to virtual tool list
	///
	/// This replaces source tools with their virtual counterparts and passes through
	/// non-virtualized tools unchanged. Compositions are not affected by this.
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
						if let Some(virtual_tool) = compiled.create_virtual_tool(source_tool_def) {
							result.push((target.clone(), virtual_tool));
						}
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

		// Add compositions as synthetic tools
		for (name, compiled) in &self.tools_by_name {
			if compiled.is_composition() {
				let output_schema = compiled
					.def
					.output_schema
					.as_ref()
					.and_then(|v| v.as_object().cloned())
					.map(Arc::new);

				let composition_tool = Tool {
					name: Cow::Owned(name.clone()),
					title: None,
					description: compiled.def.description.clone().map(Cow::Owned),
					input_schema: Arc::new(
						compiled
							.def
							.input_schema
							.clone()
							.and_then(|v| v.as_object().cloned())
							.unwrap_or_default(),
					),
					output_schema,
					annotations: None,
					icons: None,
					meta: None,
				};
				result.push(("_composition".to_string(), composition_tool));
			}
		}

		result
	}

	/// Prepare arguments for backend call (inject defaults, resolve env vars)
	///
	/// Returns (target, tool_name, transformed_args) for source-based tools.
	/// Returns error for compositions (they require the executor).
	pub fn prepare_call_args(
		&self,
		virtual_name: &str,
		args: serde_json::Value,
	) -> Result<(String, String, serde_json::Value), RegistryError> {
		let tool = self.get_tool(virtual_name).ok_or_else(|| RegistryError::tool_not_found(virtual_name))?;

		match &tool.compiled {
			CompiledImplementation::Source(source) => {
				let target = source.source.target.clone();
				let tool_name = source.source.tool.clone();
				let transformed_args = tool.inject_defaults(args)?;
				Ok((target, tool_name, transformed_args))
			},
			CompiledImplementation::Composition(_) => {
				Err(RegistryError::CompositionRequiresExecutor(virtual_name.to_string()))
			},
		}
	}

	/// Transform backend response to virtual response
	pub fn transform_output(
		&self,
		virtual_name: &str,
		response: serde_json::Value,
	) -> Result<serde_json::Value, RegistryError> {
		let tool = self.get_tool(virtual_name).ok_or_else(|| RegistryError::tool_not_found(virtual_name))?;

		tool.transform_output(response)
	}

	/// Get all tool names
	pub fn tool_names(&self) -> impl Iterator<Item = &String> {
		self.tools_by_name.keys()
	}

	/// Get number of tools
	pub fn len(&self) -> usize {
		self.tools_by_name.len()
	}

	/// Check if registry is empty
	pub fn is_empty(&self) -> bool {
		self.tools_by_name.is_empty()
	}
}

// =============================================================================
// CompiledTool Implementation
// =============================================================================

impl CompiledTool {
	/// Compile a tool definition
	pub fn compile(
		def: &ToolDefinition,
		all_defs: &HashMap<String, ToolDefinition>,
		depth: usize,
	) -> Result<Self, RegistryError> {
		if depth > MAX_REFERENCE_DEPTH {
			return Err(RegistryError::ReferenceDepthExceeded(def.name.clone()));
		}

		let compiled = match &def.implementation {
			ToolImplementation::Source(source) => {
				let output_transform = if let Some(ref transform) = def.output_transform {
					Some(CompiledOutputTransform::compile(transform)?)
				} else {
					None
				};

				CompiledImplementation::Source(CompiledSourceTool {
					source: source.clone(),
					output_transform,
					effective_schema: None,
				})
			},
			ToolImplementation::Spec(spec) => {
				let output_transform = if let Some(ref transform) = def.output_transform {
					Some(CompiledOutputTransform::compile(transform)?)
				} else {
					None
				};

				// Resolve references
				let referenced = spec.referenced_tools();
				let mut resolved_references = Vec::new();

				for ref_name in referenced {
					// Check if reference exists (could be in registry or a backend tool)
					if all_defs.contains_key(ref_name) {
						resolved_references.push(ref_name.to_string());
					} else {
						// Assume it's a backend tool - will be validated at runtime
						resolved_references.push(ref_name.to_string());
					}
				}

				CompiledImplementation::Composition(CompiledComposition {
					spec: spec.clone(),
					output_transform,
					resolved_references,
				})
			},
		};

		Ok(Self { def: def.clone(), compiled })
	}

	/// Legacy: compile from VirtualToolDef
	pub fn compile_legacy(legacy_def: VirtualToolDef) -> Result<Self, RegistryError> {
		let def = ToolDefinition::from_legacy(legacy_def);
		let defs = HashMap::new();
		Self::compile(&def, &defs, 0)
	}

	/// Check if this is a source-based tool
	pub fn is_source(&self) -> bool {
		matches!(self.compiled, CompiledImplementation::Source(_))
	}

	/// Check if this is a composition
	pub fn is_composition(&self) -> bool {
		matches!(self.compiled, CompiledImplementation::Composition(_))
	}

	/// Get source tool info if this is a source-based tool
	pub fn source_info(&self) -> Option<&CompiledSourceTool> {
		match &self.compiled {
			CompiledImplementation::Source(s) => Some(s),
			_ => None,
		}
	}

	/// Get composition info if this is a composition
	pub fn composition_info(&self) -> Option<&CompiledComposition> {
		match &self.compiled {
			CompiledImplementation::Composition(c) => Some(c),
			_ => None,
		}
	}

	/// Create a virtual tool from a source tool definition (for source-based tools only)
	pub fn create_virtual_tool(&self, source: &Tool) -> Option<Tool> {
		let source_tool = self.source_info()?;

		// Convert registry output_schema (Value) to Arc<Map> format if present
		let output_schema = self
			.def
			.output_schema
			.as_ref()
			.and_then(|v| v.as_object().cloned())
			.map(Arc::new)
			.or_else(|| source.output_schema.clone());

		Some(Tool {
			name: Cow::Owned(self.def.name.clone()),
			title: source.title.clone(),
			description: self.def.description.clone().map(Cow::Owned).or_else(|| source.description.clone()),
			input_schema: self.compute_effective_schema(source, source_tool),
			output_schema,
			annotations: source.annotations.clone(),
			icons: source.icons.clone(),
			meta: source.meta.clone(),
		})
	}

	/// Compute effective input schema by applying hideFields to source schema
	fn compute_effective_schema(
		&self,
		source: &Tool,
		source_tool: &CompiledSourceTool,
	) -> Arc<serde_json::Map<String, serde_json::Value>> {
		// If we have a complete override schema, use it
		if let Some(ref override_schema) = self.def.input_schema {
			if let Some(obj) = override_schema.as_object() {
				return Arc::new(obj.clone());
			}
		}

		// Start with source schema (clone the inner Map)
		let mut schema: serde_json::Map<String, serde_json::Value> = source.input_schema.as_ref().clone();

		// Apply hideFields
		if !source_tool.source.hide_fields.is_empty() {
			if let Some(props) = schema.get_mut("properties") {
				if let Some(obj) = props.as_object_mut() {
					for field in &source_tool.source.hide_fields {
						obj.remove(field);
					}
				}
			}
			// Also remove from required array
			if let Some(required) = schema.get_mut("required") {
				if let Some(arr) = required.as_array_mut() {
					arr.retain(|v| {
						v.as_str().map(|s| !source_tool.source.hide_fields.contains(&s.to_string())).unwrap_or(true)
					});
				}
			}
		}

		Arc::new(schema)
	}

	/// Inject default values into arguments
	pub fn inject_defaults(&self, mut args: serde_json::Value) -> Result<serde_json::Value, RegistryError> {
		let defaults = match &self.compiled {
			CompiledImplementation::Source(s) => &s.source.defaults,
			CompiledImplementation::Composition(_) => return Ok(args), // No defaults for compositions
		};

		if defaults.is_empty() {
			return Ok(args);
		}

		let obj =
			args.as_object_mut().ok_or_else(|| RegistryError::SchemaValidation("arguments must be an object".into()))?;

		for (key, value) in defaults {
			// Don't override if already provided
			if obj.contains_key(key) {
				continue;
			}

			// Resolve environment variables in string values
			let resolved_value = resolve_env_vars(value)?;
			obj.insert(key.clone(), resolved_value);
		}

		Ok(args)
	}

	/// Transform output using the output transform
	pub fn transform_output(&self, response: serde_json::Value) -> Result<serde_json::Value, RegistryError> {
		let transform = match &self.compiled {
			CompiledImplementation::Source(s) => s.output_transform.as_ref(),
			CompiledImplementation::Composition(c) => c.output_transform.as_ref(),
		};

		let Some(transform) = transform else {
			return Ok(response);
		};

		// Extract JSON if embedded in text
		let json_response = extract_json_from_response(&response)?;

		transform.apply(&json_response)
	}

	/// Check if this tool has an output transform defined
	pub fn has_output_transform(&self) -> bool {
		match &self.compiled {
			CompiledImplementation::Source(s) => s.output_transform.is_some(),
			CompiledImplementation::Composition(c) => c.output_transform.is_some(),
		}
	}

	/// Get the output transform field names for logging
	pub fn output_transform_fields(&self) -> Option<Vec<&str>> {
		let transform = match &self.compiled {
			CompiledImplementation::Source(s) => s.output_transform.as_ref(),
			CompiledImplementation::Composition(c) => c.output_transform.as_ref(),
		};
		transform.map(|t| t.fields.keys().map(|s| s.as_str()).collect())
	}
}

// =============================================================================
// CompiledOutputTransform Implementation
// =============================================================================

impl CompiledOutputTransform {
	/// Compile an output transform
	pub fn compile(transform: &OutputTransform) -> Result<Self, RegistryError> {
		let mut fields = HashMap::new();

		for (name, source) in &transform.mappings {
			let compiled = CompiledFieldSource::compile(source)?;
			fields.insert(name.clone(), compiled);
		}

		Ok(Self { fields })
	}

	/// Apply the transform to a JSON value
	///
	/// Handles array item mappings like `repos[*].name` which project fields onto array items.
	pub fn apply(&self, input: &serde_json::Value) -> Result<serde_json::Value, RegistryError> {
		let mut result = serde_json::Map::new();

		// Separate base fields from array item mappings (e.g., "repos" vs "repos[*].name")
		let mut base_fields: HashMap<&str, &CompiledFieldSource> = HashMap::new();
		let mut array_item_mappings: HashMap<&str, Vec<(&str, &CompiledFieldSource)>> = HashMap::new();

		for (field_name, field_source) in &self.fields {
			if let Some(bracket_pos) = field_name.find("[*].") {
				// This is an array item mapping like "repos[*].name"
				let base_array = &field_name[..bracket_pos];
				let item_field = &field_name[bracket_pos + 4..]; // Skip "[*]."
				array_item_mappings.entry(base_array).or_default().push((item_field, field_source));
			} else {
				base_fields.insert(field_name.as_str(), field_source);
			}
		}

		// Process base fields first
		for (field_name, field_source) in &base_fields {
			let value = field_source.extract(input)?;

			// Check if this base field has array item mappings
			if let Some(item_mappings) = array_item_mappings.get(*field_name) {
				// Transform each item in the array
				if let serde_json::Value::Array(items) = value {
					let transformed: Result<Vec<serde_json::Value>, RegistryError> = items
						.iter()
						.map(|item| {
							let mut obj = serde_json::Map::new();
							for (item_field_name, item_field_source) in item_mappings {
								let item_value = item_field_source.extract(item)?;
								obj.insert((*item_field_name).to_string(), item_value);
							}
							Ok(serde_json::Value::Object(obj))
						})
						.collect();
					result.insert((*field_name).to_string(), serde_json::Value::Array(transformed?));
				} else {
					// Not an array, just insert as-is
					result.insert((*field_name).to_string(), value);
				}
			} else {
				result.insert((*field_name).to_string(), value);
			}
		}

		// Handle array item mappings for arrays that weren't explicitly defined as base fields
		// (this would be an error case, but handle gracefully)
		for (base_array, _) in &array_item_mappings {
			if !base_fields.contains_key(*base_array) && !result.contains_key(*base_array) {
				// Array item mappings without a base array definition - skip with null
				result.insert((*base_array).to_string(), serde_json::Value::Null);
			}
		}

		Ok(serde_json::Value::Object(result))
	}
}

impl CompiledFieldSource {
	/// Compile a field source
	pub fn compile(source: &FieldSource) -> Result<Self, RegistryError> {
		match source {
			FieldSource::Path(path) => {
				let jsonpath =
					JsonPath::parse(path).map_err(|e| RegistryError::invalid_jsonpath(path, e.to_string()))?;
				Ok(CompiledFieldSource::Path { jsonpath, original: path.clone() })
			},
			FieldSource::Literal(lit) => Ok(CompiledFieldSource::Literal(lit.to_json_value())),
			FieldSource::Coalesce(c) => {
				let mut paths = Vec::new();
				let mut originals = Vec::new();
				for path in &c.paths {
					let jsonpath =
						JsonPath::parse(path).map_err(|e| RegistryError::invalid_jsonpath(path, e.to_string()))?;
					paths.push(jsonpath);
					originals.push(path.clone());
				}
				Ok(CompiledFieldSource::Coalesce { paths, originals })
			},
			FieldSource::Template(t) => {
				let mut vars = HashMap::new();
				for (name, path) in &t.vars {
					let jsonpath =
						JsonPath::parse(path).map_err(|e| RegistryError::invalid_jsonpath(path, e.to_string()))?;
					vars.insert(name.clone(), jsonpath);
				}
				Ok(CompiledFieldSource::Template { template: t.template.clone(), vars })
			},
			FieldSource::Concat(c) => {
				let mut paths = Vec::new();
				for path in &c.paths {
					let jsonpath =
						JsonPath::parse(path).map_err(|e| RegistryError::invalid_jsonpath(path, e.to_string()))?;
					paths.push(jsonpath);
				}
				Ok(CompiledFieldSource::Concat { paths, separator: c.separator.clone().unwrap_or_default() })
			},
			FieldSource::Nested(nested) => {
				let compiled = CompiledOutputTransform::compile(&OutputTransform { mappings: nested.mappings.clone() })?;
				Ok(CompiledFieldSource::Nested(Box::new(compiled)))
			},
		}
	}

	/// Extract a value from input
	pub fn extract(&self, input: &serde_json::Value) -> Result<serde_json::Value, RegistryError> {
		match self {
			CompiledFieldSource::Path { jsonpath, .. } => {
				let nodes = jsonpath.query(input);
				let values: Vec<_> = nodes.iter().map(|v| (*v).clone()).collect();
				Ok(match values.len() {
					0 => serde_json::Value::Null,
					1 => values.into_iter().next().unwrap(),
					_ => serde_json::Value::Array(values),
				})
			},
			CompiledFieldSource::Literal(value) => Ok(value.clone()),
			CompiledFieldSource::Coalesce { paths, .. } => {
				for path in paths {
					let nodes = path.query(input);
					if let Some(first) = nodes.iter().next() {
						if !first.is_null() {
							return Ok((*first).clone());
						}
					}
				}
				Ok(serde_json::Value::Null)
			},
			CompiledFieldSource::Template { template, vars } => {
				let mut result = template.clone();
				for (name, path) in vars {
					let nodes = path.query(input);
					let value = nodes.iter().next().and_then(|v| v.as_str()).unwrap_or("");
					result = result.replace(&format!("{{{}}}", name), value);
				}
				Ok(serde_json::Value::String(result))
			},
			CompiledFieldSource::Concat { paths, separator } => {
				let mut parts = Vec::new();
				for path in paths {
					let nodes = path.query(input);
					if let Some(first) = nodes.iter().next() {
						if let Some(s) = first.as_str() {
							parts.push(s.to_string());
						}
					}
				}
				Ok(serde_json::Value::String(parts.join(separator)))
			},
			CompiledFieldSource::Nested(transform) => transform.apply(input),
		}
	}
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Resolve ${ENV_VAR} patterns in a JSON value
fn resolve_env_vars(value: &serde_json::Value) -> Result<serde_json::Value, RegistryError> {
	match value {
		serde_json::Value::String(s) => {
			let resolved = resolve_env_string(s)?;
			Ok(serde_json::Value::String(resolved))
		},
		serde_json::Value::Object(obj) => {
			let mut new_obj = serde_json::Map::new();
			for (k, v) in obj {
				new_obj.insert(k.clone(), resolve_env_vars(v)?);
			}
			Ok(serde_json::Value::Object(new_obj))
		},
		serde_json::Value::Array(arr) => {
			let new_arr: Result<Vec<_>, _> = arr.iter().map(resolve_env_vars).collect();
			Ok(serde_json::Value::Array(new_arr?))
		},
		other => Ok(other.clone()),
	}
}

/// Resolve ${ENV_VAR} patterns in a string
fn resolve_env_string(s: &str) -> Result<String, RegistryError> {
	let mut result = s.to_string();
	let re = regex::Regex::new(r"\$\{([^}]+)\}").expect("valid regex");

	for cap in re.captures_iter(s) {
		let var_name = &cap[1];
		let value =
			std::env::var(var_name).map_err(|_| RegistryError::EnvVarNotFound { name: var_name.to_string() })?;
		result = result.replace(&cap[0], &value);
	}

	Ok(result)
}

/// Extract JSON from response (handles JSON embedded in text)
fn extract_json_from_response(response: &serde_json::Value) -> Result<serde_json::Value, RegistryError> {
	match response {
		serde_json::Value::Object(_) | serde_json::Value::Array(_) => Ok(response.clone()),
		serde_json::Value::String(s) => {
			if let Ok(json) = serde_json::from_str(s) {
				return Ok(json);
			}
			if let Some(json) = find_json_in_text(s) {
				return Ok(json);
			}
			Ok(response.clone())
		},
		other => Ok(other.clone()),
	}
}

/// Find JSON object or array embedded in text
fn find_json_in_text(text: &str) -> Option<serde_json::Value> {
	if let Some(start) = text.find('{') {
		if let Some(json) = try_parse_json_from(&text[start..], '{', '}') {
			return Some(json);
		}
	}
	if let Some(start) = text.find('[') {
		if let Some(json) = try_parse_json_from(&text[start..], '[', ']') {
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

// =============================================================================
// Legacy Compatibility (CompiledVirtualTool methods)
// =============================================================================

impl CompiledTool {
	/// Legacy: access def as VirtualToolDef (only works for source-based tools)
	pub fn legacy_def(&self) -> Option<VirtualToolDef> {
		if let ToolImplementation::Source(source) = &self.def.implementation {
			Some(VirtualToolDef {
				name: self.def.name.clone(),
				source: super::types::ToolSource { target: source.target.clone(), tool: source.tool.clone() },
				description: self.def.description.clone(),
				input_schema: self.def.input_schema.clone(),
				defaults: source.defaults.clone(),
				hide_fields: source.hide_fields.clone(),
				output_schema: None, // Not converting back to legacy format
				version: self.def.version.clone(),
				metadata: self.def.metadata.clone(),
			})
		} else {
			None
		}
	}
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;
	use crate::mcp::registry::patterns::{PipelineSpec, PipelineStep, ScatterGatherSpec, ScatterTarget, StepOperation, ToolCall, AggregationStrategy, AggregationOp};
	use crate::mcp::registry::types::OutputField;

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

	#[test]
	fn test_compile_empty_registry() {
		let registry = Registry::new();
		let compiled = CompiledRegistry::compile(registry).unwrap();
		assert!(compiled.is_empty());
		assert_eq!(compiled.len(), 0);
	}

	#[test]
	fn test_compile_simple_registry() {
		let tool = VirtualToolDef::new("get_weather", "weather", "fetch_weather");
		let registry = Registry::with_tools(vec![tool]);

		let compiled = CompiledRegistry::compile(registry).unwrap();
		assert_eq!(compiled.len(), 1);
		assert!(compiled.get_tool("get_weather").is_some());
		assert!(compiled.get_tool("nonexistent").is_none());
	}

	#[test]
	fn test_compile_mixed_registry() {
		// Source-based tool
		let source_tool = ToolDefinition::source("get_weather", "weather", "fetch_weather");

		// Composition-based tool
		let composition = ToolDefinition::composition(
			"research_pipeline",
			PatternSpec::Pipeline(PipelineSpec {
				steps: vec![PipelineStep {
					id: "search".to_string(),
					operation: StepOperation::Tool(ToolCall { name: "web_search".to_string() }),
					input: None,
				}],
			}),
		);

		let registry = Registry::with_tool_definitions(vec![source_tool, composition]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		assert_eq!(compiled.len(), 2);
		assert!(compiled.is_source_tool("get_weather"));
		assert!(compiled.is_composition("research_pipeline"));
	}

	#[test]
	fn test_two_pass_forward_reference() {
		// Composition references a tool defined after it
		let json = r#"{
			"schemaVersion": "1.0",
			"tools": [
				{
					"name": "pipeline",
					"spec": {
						"scatterGather": {
							"targets": [
								{ "tool": "normalized_search" }
							],
							"aggregation": { "ops": [] }
						}
					}
				},
				{
					"name": "normalized_search",
					"source": {
						"target": "search",
						"tool": "raw_search"
					}
				}
			]
		}"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		let compiled = CompiledRegistry::compile(registry).unwrap();

		assert_eq!(compiled.len(), 2);
		assert!(compiled.is_composition("pipeline"));
		assert!(compiled.is_source_tool("normalized_search"));
	}

	#[test]
	fn test_is_virtualized() {
		let tool = VirtualToolDef::new("get_weather", "weather", "fetch_weather");
		let registry = Registry::with_tools(vec![tool]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		assert!(compiled.is_virtualized("weather", "fetch_weather"));
		assert!(!compiled.is_virtualized("weather", "other_tool"));
		assert!(!compiled.is_virtualized("other_backend", "fetch_weather"));
	}

	#[test]
	fn test_transform_tools_replaces_virtualized() {
		let tool = VirtualToolDef::new("get_weather", "weather", "fetch_weather").with_description("Get weather info");
		let registry = Registry::with_tools(vec![tool]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let source_tool = create_source_tool("fetch_weather", "Original description");
		let backend_tools = vec![("weather".to_string(), source_tool)];

		let result = compiled.transform_tools(backend_tools);

		// Should have the virtual tool
		let virtual_tools: Vec<_> = result.iter().filter(|(t, _)| t == "weather").collect();
		assert_eq!(virtual_tools.len(), 1);
		assert_eq!(virtual_tools[0].1.name.as_ref(), "get_weather");
		assert_eq!(virtual_tools[0].1.description.as_deref(), Some("Get weather info"));
	}

	#[test]
	fn test_transform_tools_passthrough_non_virtualized() {
		let tool = VirtualToolDef::new("get_weather", "weather", "fetch_weather");
		let registry = Registry::with_tools(vec![tool]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let source_tool = create_source_tool("fetch_weather", "Weather");
		let other_tool = create_source_tool("other_tool", "Other");
		let backend_tools = vec![("weather".to_string(), source_tool), ("weather".to_string(), other_tool)];

		let result = compiled.transform_tools(backend_tools);

		let names: Vec<_> = result.iter().map(|(_, t)| t.name.as_ref()).collect();
		assert!(names.contains(&"get_weather"));
		assert!(names.contains(&"other_tool"));
	}

	#[test]
	fn test_hide_fields_in_schema() {
		let tool = VirtualToolDef::new("get_weather", "weather", "fetch_weather")
			.with_hidden_fields(vec!["debug_mode".to_string()]);

		let def = ToolDefinition::from_legacy(tool);
		let defs = HashMap::new();
		let compiled = CompiledTool::compile(&def, &defs, 0).unwrap();

		let source = create_source_tool("fetch_weather", "Weather");
		let virtual_tool = compiled.create_virtual_tool(&source).unwrap();

		let props = virtual_tool.input_schema.get("properties").unwrap();
		assert!(props.get("city").is_some());
		assert!(props.get("units").is_some());
		assert!(props.get("debug_mode").is_none());
	}

	#[test]
	fn test_inject_defaults() {
		let tool = VirtualToolDef::new("get_weather", "weather", "fetch_weather")
			.with_default("units", json!("metric"))
			.with_default("format", json!("json"));

		let def = ToolDefinition::from_legacy(tool);
		let defs = HashMap::new();
		let compiled = CompiledTool::compile(&def, &defs, 0).unwrap();

		let args = json!({"city": "Seattle"});
		let result = compiled.inject_defaults(args).unwrap();

		assert_eq!(result["city"], "Seattle");
		assert_eq!(result["units"], "metric");
		assert_eq!(result["format"], "json");
	}

	#[test]
	fn test_inject_defaults_does_not_override() {
		let tool = VirtualToolDef::new("get_weather", "weather", "fetch_weather").with_default("units", json!("metric"));

		let def = ToolDefinition::from_legacy(tool);
		let defs = HashMap::new();
		let compiled = CompiledTool::compile(&def, &defs, 0).unwrap();

		let args = json!({"city": "Seattle", "units": "imperial"});
		let result = compiled.inject_defaults(args).unwrap();

		assert_eq!(result["units"], "imperial");
	}

	#[test]
	fn test_inject_defaults_with_env_var() {
		unsafe {
			std::env::set_var("TEST_API_KEY_COMPILED", "secret123");
		}

		let mut tool = VirtualToolDef::new("test", "backend", "tool");
		tool.defaults.insert("api_key".to_string(), json!("${TEST_API_KEY_COMPILED}"));

		let def = ToolDefinition::from_legacy(tool);
		let defs = HashMap::new();
		let compiled = CompiledTool::compile(&def, &defs, 0).unwrap();

		let args = json!({});
		let result = compiled.inject_defaults(args).unwrap();

		assert_eq!(result["api_key"], "secret123");

		unsafe {
			std::env::remove_var("TEST_API_KEY_COMPILED");
		}
	}

	#[test]
	fn test_output_transformation_simple() {
		let mut props = HashMap::new();
		props.insert("temp".to_string(), OutputField::new("number", "$.temperature"));
		props.insert("city".to_string(), OutputField::new("string", "$.location.city"));

		let output_schema = super::super::types::OutputSchema::new(props);
		let tool = VirtualToolDef::new("test", "backend", "tool").with_output_schema(output_schema);

		let def = ToolDefinition::from_legacy(tool);
		let defs = HashMap::new();
		let compiled = CompiledTool::compile(&def, &defs, 0).unwrap();

		let response = json!({
			"temperature": 72.5,
			"location": {
				"city": "Seattle",
				"state": "WA"
			}
		});

		let result = compiled.transform_output(response).unwrap();

		assert_eq!(result["temp"], 72.5);
		assert_eq!(result["city"], "Seattle");
	}

	#[test]
	fn test_output_transformation_no_schema() {
		let tool = VirtualToolDef::new("test", "backend", "tool");
		let def = ToolDefinition::from_legacy(tool);
		let defs = HashMap::new();
		let compiled = CompiledTool::compile(&def, &defs, 0).unwrap();

		let response = json!({"original": "data"});
		let result = compiled.transform_output(response.clone()).unwrap();

		assert_eq!(result, response);
	}

	#[test]
	fn test_output_transform_array_item_mapping() {
		// Test the repos[*].name pattern for transforming array items
		let json = r#"{
			"name": "search_repos",
			"source": { "target": "github", "tool": "search_repositories" },
			"outputTransform": {
				"mappings": {
					"total": { "path": "$.total_count" },
					"repos": { "path": "$.items[*]" },
					"repos[*].name": { "path": "$.full_name" },
					"repos[*].stars": { "path": "$.stargazers_count" },
					"repos[*].url": { "path": "$.html_url" }
				}
			}
		}"#;

		let def: ToolDefinition = serde_json::from_str(json).unwrap();
		let defs = HashMap::new();
		let compiled = CompiledTool::compile(&def, &defs, 0).unwrap();

		let response = json!({
			"total_count": 2,
			"items": [
				{
					"full_name": "owner1/repo1",
					"stargazers_count": 100,
					"html_url": "https://github.com/owner1/repo1",
					"extra_field": "ignored"
				},
				{
					"full_name": "owner2/repo2",
					"stargazers_count": 200,
					"html_url": "https://github.com/owner2/repo2",
					"extra_field": "also_ignored"
				}
			]
		});

		let result = compiled.transform_output(response).unwrap();

		// Verify total is extracted
		assert_eq!(result["total"], 2);

		// Verify repos is an array of transformed items
		let repos = result["repos"].as_array().unwrap();
		assert_eq!(repos.len(), 2);

		// Verify first item has only the mapped fields
		assert_eq!(repos[0]["name"], "owner1/repo1");
		assert_eq!(repos[0]["stars"], 100);
		assert_eq!(repos[0]["url"], "https://github.com/owner1/repo1");
		assert!(repos[0].get("extra_field").is_none());
		assert!(repos[0].get("full_name").is_none()); // Should be renamed to "name"

		// Verify second item
		assert_eq!(repos[1]["name"], "owner2/repo2");
		assert_eq!(repos[1]["stars"], 200);
	}

	#[test]
	fn test_output_transform_coalesce() {
		let json = r#"{
			"name": "test",
			"source": { "target": "backend", "tool": "tool" },
			"outputTransform": {
				"mappings": {
					"url": { "coalesce": { "paths": ["$.pdf_url", "$.web_url", "$.fallback"] } }
				}
			}
		}"#;

		let def: ToolDefinition = serde_json::from_str(json).unwrap();
		let defs = HashMap::new();
		let compiled = CompiledTool::compile(&def, &defs, 0).unwrap();

		// First path has value
		let response = json!({"pdf_url": "http://pdf.example.com"});
		let result = compiled.transform_output(response).unwrap();
		assert_eq!(result["url"], "http://pdf.example.com");

		// First path null, second has value
		let response = json!({"pdf_url": null, "web_url": "http://web.example.com"});
		let result = compiled.transform_output(response).unwrap();
		assert_eq!(result["url"], "http://web.example.com");
	}

	#[test]
	fn test_output_transform_literal() {
		let json = r#"{
			"name": "test",
			"source": { "target": "backend", "tool": "tool" },
			"outputTransform": {
				"mappings": {
					"source": { "literal": { "stringValue": "arxiv" } },
					"relevance": { "literal": { "numberValue": 0.85 } }
				}
			}
		}"#;

		let def: ToolDefinition = serde_json::from_str(json).unwrap();
		let defs = HashMap::new();
		let compiled = CompiledTool::compile(&def, &defs, 0).unwrap();

		let response = json!({});
		let result = compiled.transform_output(response).unwrap();
		assert_eq!(result["source"], "arxiv");
		assert_eq!(result["relevance"], 0.85);
	}

	#[test]
	fn test_prepare_call_args() {
		let tool = VirtualToolDef::new("get_weather", "weather", "fetch_weather").with_default("units", json!("metric"));
		let registry = Registry::with_tools(vec![tool]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let args = json!({"city": "Seattle"});
		let (target, tool_name, transformed) = compiled.prepare_call_args("get_weather", args).unwrap();

		assert_eq!(target, "weather");
		assert_eq!(tool_name, "fetch_weather");
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
	fn test_prepare_call_args_composition_error() {
		let composition = ToolDefinition::composition(
			"pipeline",
			PatternSpec::Pipeline(PipelineSpec { steps: vec![] }),
		);
		let registry = Registry::with_tool_definitions(vec![composition]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let result = compiled.prepare_call_args("pipeline", json!({}));
		assert!(result.is_err());
	}

	#[test]
	fn test_multiple_virtual_tools_same_source() {
		let tool1 = VirtualToolDef::new("weather_metric", "weather", "fetch_weather").with_default("units", json!("metric"));
		let tool2 = VirtualToolDef::new("weather_imperial", "weather", "fetch_weather").with_default("units", json!("imperial"));

		let registry = Registry::with_tools(vec![tool1, tool2]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let names = compiled.get_virtual_names("weather", "fetch_weather").unwrap();
		assert_eq!(names.len(), 2);
		assert!(names.contains(&"weather_metric".to_string()));
		assert!(names.contains(&"weather_imperial".to_string()));
	}

	#[test]
	fn test_duplicate_tool_name_error() {
		let json = r#"{
			"tools": [
				{ "name": "duplicate", "source": { "target": "a", "tool": "a" } },
				{ "name": "duplicate", "source": { "target": "b", "tool": "b" } }
			]
		}"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		let result = CompiledRegistry::compile(registry);
		assert!(result.is_err());
	}

	#[test]
	fn test_composition_resolved_references() {
		let composition = ToolDefinition::composition(
			"pipeline",
			PatternSpec::ScatterGather(ScatterGatherSpec {
				targets: vec![
					ScatterTarget::Tool("tool_a".to_string()),
					ScatterTarget::Tool("tool_b".to_string()),
				],
				aggregation: AggregationStrategy { ops: vec![AggregationOp::Flatten(true)] },
				timeout_ms: None,
				fail_fast: false,
			}),
		);

		let registry = Registry::with_tool_definitions(vec![composition]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let tool = compiled.get_tool("pipeline").unwrap();
		let comp = tool.composition_info().unwrap();
		assert_eq!(comp.resolved_references.len(), 2);
		assert!(comp.resolved_references.contains(&"tool_a".to_string()));
		assert!(comp.resolved_references.contains(&"tool_b".to_string()));
	}

	#[test]
	fn test_extract_json_from_text() {
		let text = r#"Here is the result: {"temperature": 72.5, "city": "Seattle"} and some more text"#;
		let json = find_json_in_text(text).unwrap();

		assert_eq!(json["temperature"], 72.5);
		assert_eq!(json["city"], "Seattle");
	}

	#[test]
	fn test_extract_json_array_from_text() {
		let text = r#"Results: [1, 2, 3] done"#;
		let json = find_json_in_text(text).unwrap();

		assert!(json.is_array());
		assert_eq!(json.as_array().unwrap().len(), 3);
	}
}
