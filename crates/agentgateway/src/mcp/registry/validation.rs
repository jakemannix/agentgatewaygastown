// Registry Validation Module (WP3)
//
// Provides validation for Registry v2 including:
// - Dependency cycle detection
// - Missing dependency errors
// - Schema reference validation
// - Deprecation warnings
// - Version constraint validation

use std::collections::{HashMap, HashSet};

use super::types::{DependencyType, Registry, ToolImplementation};
use thiserror::Error;

/// Validation errors for registry v2
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ValidationError {
	#[error("dependency cycle detected: {}", .0.join(" -> "))]
	DependencyCycle(Vec<String>),

	#[error("missing dependency: tool '{tool}' depends on unknown {dep_type} '{dependency}'")]
	MissingDependency {
		tool: String,
		dependency: String,
		dep_type: String,
	},

	#[error("missing schema reference: '{reference}' in tool '{tool}'")]
	MissingSchemaRef { tool: String, reference: String },

	#[error("version constraint not satisfied: tool '{tool}' requires {dep_type} '{dependency}' version '{required}', found '{found}'")]
	VersionMismatch {
		tool: String,
		dependency: String,
		dep_type: String,
		required: String,
		found: String,
	},

	#[error("deprecated {entity_type} '{name}' is used by tool '{tool}': {message}")]
	DeprecatedUsage {
		tool: String,
		name: String,
		entity_type: String, // "tool" | "server" | "agent"
		message: String,
	},

	#[error("duplicate tool name: '{0}'")]
	DuplicateToolName(String),

	#[error("duplicate schema name: '{0}'")]
	DuplicateSchemeName(String),

	#[error("duplicate server name: '{0}'")]
	DuplicateServerName(String),

	#[error("duplicate agent name: '{0}'")]
	DuplicateAgentName(String),
}

/// Validation warning (non-fatal)
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationWarning {
	pub message: String,
	pub tool: Option<String>,
}

/// Result of registry validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
	/// Fatal errors that prevent the registry from being used
	pub errors: Vec<ValidationError>,
	/// Warnings that should be logged but don't block usage
	pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
	/// Create a successful result with no errors or warnings
	pub fn ok() -> Self {
		Self {
			errors: Vec::new(),
			warnings: Vec::new(),
		}
	}

	/// Check if validation passed (no errors)
	pub fn is_ok(&self) -> bool {
		self.errors.is_empty()
	}

	/// Check if there are any warnings
	pub fn has_warnings(&self) -> bool {
		!self.warnings.is_empty()
	}

	/// Add an error
	pub fn add_error(&mut self, error: ValidationError) {
		self.errors.push(error);
	}

	/// Add a warning
	pub fn add_warning(&mut self, warning: ValidationWarning) {
		self.warnings.push(warning);
	}
}

/// Registry validator for v2 registries
pub struct RegistryValidator<'a> {
	registry: &'a Registry,
}

impl<'a> RegistryValidator<'a> {
	/// Create a new validator for the given registry
	pub fn new(registry: &'a Registry) -> Self {
		Self { registry }
	}

	/// Validate the registry and return all errors and warnings
	pub fn validate(&self) -> ValidationResult {
		let mut result = ValidationResult::ok();

		// Run all validations and aggregate results
		let unique = self.validate_unique_names();
		result.errors.extend(unique.errors);
		result.warnings.extend(unique.warnings);

		let deps = self.validate_dependencies_exist();
		result.errors.extend(deps.errors);
		result.warnings.extend(deps.warnings);

		let cycles = self.validate_no_cycles();
		result.errors.extend(cycles.errors);
		result.warnings.extend(cycles.warnings);

		let schemas = self.validate_schema_refs();
		result.errors.extend(schemas.errors);
		result.warnings.extend(schemas.warnings);

		let deprecations = self.validate_deprecations();
		result.errors.extend(deprecations.errors);
		result.warnings.extend(deprecations.warnings);

		let versions = self.validate_version_constraints();
		result.errors.extend(versions.errors);
		result.warnings.extend(versions.warnings);

		result
	}

	/// Check for duplicate names (tools, schemas, servers, agents)
	pub fn validate_unique_names(&self) -> ValidationResult {
		let mut result = ValidationResult::ok();
		let mut seen_tools = HashSet::new();
		let mut seen_schemas = HashSet::new();
		let mut seen_servers = HashSet::new();
		let mut seen_agents = HashSet::new();

		for tool in &self.registry.tools {
			if !seen_tools.insert(&tool.name) {
				result.add_error(ValidationError::DuplicateToolName(tool.name.clone()));
			}
		}

		for schema in &self.registry.schemas {
			if !seen_schemas.insert(&schema.name) {
				result.add_error(ValidationError::DuplicateSchemeName(schema.name.clone()));
			}
		}

		for server in &self.registry.servers {
			if !seen_servers.insert(&server.name) {
				result.add_error(ValidationError::DuplicateServerName(server.name.clone()));
			}
		}

		for agent in &self.registry.agents {
			if !seen_agents.insert(&agent.name) {
				result.add_error(ValidationError::DuplicateAgentName(agent.name.clone()));
			}
		}

		result
	}

	/// Check for dependency cycles in tool definitions using DFS
	pub fn validate_no_cycles(&self) -> ValidationResult {
		let mut result = ValidationResult::ok();

		// Build adjacency list for tool dependencies
		let tool_names: HashSet<_> = self.registry.tools.iter().map(|t| t.name.as_str()).collect();

		// Track visited state: 0 = unvisited, 1 = in current path, 2 = fully visited
		let mut state: HashMap<&str, u8> = HashMap::new();
		let mut path: Vec<&str> = Vec::new();

		for tool in &self.registry.tools {
			if state.get(tool.name.as_str()).copied().unwrap_or(0) == 0 {
				if let Some(cycle) = self.dfs_find_cycle(&tool.name, &tool_names, &mut state, &mut path) {
					result.add_error(ValidationError::DependencyCycle(cycle));
				}
			}
		}

		result
	}

	/// DFS helper to find cycles
	fn dfs_find_cycle(
		&self,
		node: &'a str,
		tool_names: &HashSet<&'a str>,
		state: &mut HashMap<&'a str, u8>,
		path: &mut Vec<&'a str>,
	) -> Option<Vec<String>> {
		state.insert(node, 1); // Mark as visiting
		path.push(node);

		// Find this tool's dependencies
		if let Some(tool) = self.registry.tools.iter().find(|t| t.name == node) {
			for dep in &tool.depends {
				if dep.dep_type == DependencyType::Tool {
					let dep_name = dep.name.as_str();

					// Only check tools that exist in registry
					if !tool_names.contains(dep_name) {
						continue;
					}

					match state.get(dep_name).copied().unwrap_or(0) {
						1 => {
							// Found cycle - extract the cycle path
							let cycle_start = path.iter().position(|&n| n == dep_name).unwrap();
							let mut cycle: Vec<String> = path[cycle_start..].iter().map(|s| s.to_string()).collect();
							cycle.push(dep_name.to_string()); // Complete the cycle
							path.pop();
							state.insert(node, 2);
							return Some(cycle);
						}
						0 => {
							if let Some(cycle) = self.dfs_find_cycle(dep_name, tool_names, state, path) {
								return Some(cycle);
							}
						}
						_ => {} // Already fully visited, no cycle through this node
					}
				}
			}
		}

		path.pop();
		state.insert(node, 2); // Mark as fully visited
		None
	}

	/// Check that all declared dependencies exist in the registry
	pub fn validate_dependencies_exist(&self) -> ValidationResult {
		let mut result = ValidationResult::ok();

		let tool_names: HashSet<_> = self.registry.tools.iter().map(|t| t.name.as_str()).collect();
		let agent_names: HashSet<_> = self.registry.agents.iter().map(|a| a.name.as_str()).collect();

		for tool in &self.registry.tools {
			for dep in &tool.depends {
				let exists = match dep.dep_type {
					DependencyType::Tool => tool_names.contains(dep.name.as_str()),
					DependencyType::Agent => agent_names.contains(dep.name.as_str()),
				};

				if !exists {
					let dep_type = match dep.dep_type {
						DependencyType::Tool => "tool",
						DependencyType::Agent => "agent",
					};
					result.add_error(ValidationError::MissingDependency {
						tool: tool.name.clone(),
						dependency: dep.name.clone(),
						dep_type: dep_type.to_string(),
					});
				}
			}
		}

		result
	}

	/// Check that all schema $refs point to existing schemas
	pub fn validate_schema_refs(&self) -> ValidationResult {
		let mut result = ValidationResult::ok();

		let schema_names: HashSet<_> = self.registry.schemas.iter().map(|s| s.name.as_str()).collect();

		for tool in &self.registry.tools {
			// Check input_schema for $ref
			if let Some(ref schema) = tool.input_schema {
				self.check_schema_refs(schema, &tool.name, &schema_names, &mut result);
			}

			// Check output_schema for $ref
			if let Some(ref schema) = tool.output_schema {
				self.check_schema_refs(schema, &tool.name, &schema_names, &mut result);
			}
		}

		result
	}

	/// Helper to recursively check schema refs
	fn check_schema_refs(
		&self,
		schema: &serde_json::Value,
		tool_name: &str,
		schema_names: &HashSet<&str>,
		result: &mut ValidationResult,
	) {
		if let Some(obj) = schema.as_object() {
			if let Some(ref_val) = obj.get("$ref") {
				if let Some(ref_str) = ref_val.as_str() {
					// Parse #/schemas/Name format
					if ref_str.starts_with("#/schemas/") {
						let schema_name = &ref_str[10..]; // Skip "#/schemas/"
						if !schema_names.contains(schema_name) {
							result.add_error(ValidationError::MissingSchemaRef {
								tool: tool_name.to_string(),
								reference: ref_str.to_string(),
							});
						}
					}
				}
			}

			// Recursively check nested objects
			for (_, value) in obj {
				self.check_schema_refs(value, tool_name, schema_names, result);
			}
		} else if let Some(arr) = schema.as_array() {
			for item in arr {
				self.check_schema_refs(item, tool_name, schema_names, result);
			}
		}
	}

	/// Check for deprecated tool/server/agent usage and emit warnings
	pub fn validate_deprecations(&self) -> ValidationResult {
		let mut result = ValidationResult::ok();

		// Build lookup maps
		let deprecated_tools: HashMap<&str, &str> = self
			.registry
			.tools
			.iter()
			.filter_map(|t| t.deprecated.as_ref().map(|msg| (t.name.as_str(), msg.as_str())))
			.collect();

		let deprecated_servers: HashMap<&str, &str> = self
			.registry
			.servers
			.iter()
			.filter(|s| s.deprecated)
			.map(|s| {
				(
					s.name.as_str(),
					s.deprecation_message.as_deref().unwrap_or("deprecated"),
				)
			})
			.collect();

		// Check tool dependencies for deprecated tools
		for tool in &self.registry.tools {
			for dep in &tool.depends {
				if dep.dep_type == DependencyType::Tool {
					if let Some(&msg) = deprecated_tools.get(dep.name.as_str()) {
						result.add_warning(ValidationWarning {
							message: format!(
								"Tool '{}' depends on deprecated tool '{}': {}",
								tool.name, dep.name, msg
							),
							tool: Some(tool.name.clone()),
						});
					}
				}
			}

			// Check if tool sources from deprecated server
			if let ToolImplementation::Source(ref source) = tool.implementation {
				if let Some(&msg) = deprecated_servers.get(source.target.as_str()) {
					result.add_warning(ValidationWarning {
						message: format!(
							"Tool '{}' uses deprecated server '{}': {}",
							tool.name, source.target, msg
						),
						tool: Some(tool.name.clone()),
					});
				}
			}
		}

		result
	}

	/// Check version constraints on dependencies
	pub fn validate_version_constraints(&self) -> ValidationResult {
		let mut result = ValidationResult::ok();

		// Build version lookup
		let tool_versions: HashMap<&str, &str> = self
			.registry
			.tools
			.iter()
			.filter_map(|t| t.version.as_ref().map(|v| (t.name.as_str(), v.as_str())))
			.collect();

		for tool in &self.registry.tools {
			for dep in &tool.depends {
				if dep.dep_type == DependencyType::Tool {
					if let Some(ref required_version) = dep.version {
						if let Some(&actual_version) = tool_versions.get(dep.name.as_str()) {
							if !self.version_satisfies(actual_version, required_version) {
								result.add_error(ValidationError::VersionMismatch {
									tool: tool.name.clone(),
									dependency: dep.name.clone(),
									dep_type: "tool".to_string(),
									required: required_version.clone(),
									found: actual_version.to_string(),
								});
							}
						}
					}
				}
			}
		}

		result
	}

	/// Check if actual version satisfies the constraint
	fn version_satisfies(&self, actual: &str, constraint: &str) -> bool {
		// Parse semver constraint: >=X.Y.Z, <=X.Y.Z, >X.Y.Z, <X.Y.Z, =X.Y.Z, X.Y.Z
		let (op, version_str) = if constraint.starts_with(">=") {
			(">=", &constraint[2..])
		} else if constraint.starts_with("<=") {
			("<=", &constraint[2..])
		} else if constraint.starts_with('>') {
			(">", &constraint[1..])
		} else if constraint.starts_with('<') {
			("<", &constraint[1..])
		} else if constraint.starts_with('=') {
			("=", &constraint[1..])
		} else {
			("=", constraint) // Default to exact match
		};

		let actual_parts = Self::parse_version(actual);
		let required_parts = Self::parse_version(version_str);

		match op {
			">=" => actual_parts >= required_parts,
			"<=" => actual_parts <= required_parts,
			">" => actual_parts > required_parts,
			"<" => actual_parts < required_parts,
			"=" | _ => actual_parts == required_parts,
		}
	}

	/// Parse version string into comparable tuple
	fn parse_version(version: &str) -> (u32, u32, u32) {
		let parts: Vec<u32> = version
			.split('.')
			.filter_map(|p| p.parse().ok())
			.collect();

		(
			parts.first().copied().unwrap_or(0),
			parts.get(1).copied().unwrap_or(0),
			parts.get(2).copied().unwrap_or(0),
		)
	}
}

/// Convenience function to validate a registry
pub fn validate_registry(registry: &Registry) -> ValidationResult {
	RegistryValidator::new(registry).validate()
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;
	use crate::mcp::registry::types::{
		Dependency, DependencyType, Schema, Server, SourceTool, ToolDefinition, ToolImplementation,
	};

	// =============================================================================
	// Helper functions for building test registries
	// =============================================================================

	fn tool_with_deps(name: &str, deps: Vec<(&str, DependencyType)>) -> ToolDefinition {
		ToolDefinition {
			name: name.to_string(),
			description: None,
			implementation: ToolImplementation::Source(SourceTool {
				target: "backend".to_string(),
				tool: name.to_string(),
				defaults: HashMap::new(),
				hide_fields: Vec::new(),
				server_version: None,
			}),
			input_schema: None,
			output_transform: None,
			output_schema: None,
			version: Some("1.0.0".to_string()),
			metadata: HashMap::new(),
			tags: Vec::new(),
			deprecated: None,
			depends: deps
				.into_iter()
				.map(|(dep_name, dep_type)| Dependency {
					dep_type,
					name: dep_name.to_string(),
					version: None,
					skill: None,
				})
				.collect(),
		}
	}

	fn tool_with_versioned_dep(name: &str, dep_name: &str, version: &str) -> ToolDefinition {
		ToolDefinition {
			name: name.to_string(),
			description: None,
			implementation: ToolImplementation::Source(SourceTool {
				target: "backend".to_string(),
				tool: name.to_string(),
				defaults: HashMap::new(),
				hide_fields: Vec::new(),
				server_version: None,
			}),
			input_schema: None,
			output_transform: None,
			output_schema: None,
			version: Some("1.0.0".to_string()),
			metadata: HashMap::new(),
			tags: Vec::new(),
			deprecated: None,
			depends: vec![Dependency {
				dep_type: DependencyType::Tool,
				name: dep_name.to_string(),
				version: Some(version.to_string()),
				skill: None,
			}],
		}
	}

	fn deprecated_tool(name: &str, msg: &str) -> ToolDefinition {
		ToolDefinition {
			name: name.to_string(),
			description: None,
			implementation: ToolImplementation::Source(SourceTool {
				target: "backend".to_string(),
				tool: name.to_string(),
				defaults: HashMap::new(),
				hide_fields: Vec::new(),
				server_version: None,
			}),
			input_schema: None,
			output_transform: None,
			output_schema: None,
			version: Some("1.0.0".to_string()),
			metadata: HashMap::new(),
			tags: Vec::new(),
			deprecated: Some(msg.to_string()),
			depends: Vec::new(),
		}
	}

	fn simple_tool(name: &str) -> ToolDefinition {
		tool_with_deps(name, vec![])
	}

	fn versioned_tool(name: &str, version: &str) -> ToolDefinition {
		let mut tool = simple_tool(name);
		tool.version = Some(version.to_string());
		tool
	}

	// =============================================================================
	// WP3 Failing Tests: Dependency Cycle Detection
	// =============================================================================

	#[test]
	fn test_detect_simple_cycle() {
		// A -> B -> A (cycle)
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_deps("tool_a", vec![("tool_b", DependencyType::Tool)]),
				tool_with_deps("tool_b", vec![("tool_a", DependencyType::Tool)]),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_no_cycles();

		// This test should FAIL until WP3 is implemented
		// The current stub returns Ok, but we expect an error
		assert!(
			!result.is_ok(),
			"Expected cycle detection error, but validation passed"
		);
		assert!(
			result.errors.iter().any(|e| matches!(e, ValidationError::DependencyCycle(_))),
			"Expected DependencyCycle error, got: {:?}",
			result.errors
		);
	}

	#[test]
	fn test_detect_transitive_cycle() {
		// A -> B -> C -> A (longer cycle)
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_deps("tool_a", vec![("tool_b", DependencyType::Tool)]),
				tool_with_deps("tool_b", vec![("tool_c", DependencyType::Tool)]),
				tool_with_deps("tool_c", vec![("tool_a", DependencyType::Tool)]),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_no_cycles();

		assert!(
			!result.is_ok(),
			"Expected cycle detection error for A->B->C->A"
		);
		assert!(
			result.errors.iter().any(|e| matches!(e, ValidationError::DependencyCycle(_))),
			"Expected DependencyCycle error"
		);
	}

	#[test]
	fn test_detect_self_reference_cycle() {
		// A -> A (self-reference)
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![tool_with_deps("tool_a", vec![("tool_a", DependencyType::Tool)])],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_no_cycles();

		assert!(
			!result.is_ok(),
			"Expected cycle detection error for self-reference"
		);
	}

	#[test]
	fn test_no_cycle_valid_dag() {
		// A -> B, A -> C, B -> D, C -> D (valid DAG, no cycles)
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_deps(
					"tool_a",
					vec![
						("tool_b", DependencyType::Tool),
						("tool_c", DependencyType::Tool),
					],
				),
				tool_with_deps("tool_b", vec![("tool_d", DependencyType::Tool)]),
				tool_with_deps("tool_c", vec![("tool_d", DependencyType::Tool)]),
				simple_tool("tool_d"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_no_cycles();

		// This should pass even with stubs (returns Ok)
		assert!(result.is_ok(), "Valid DAG should not have cycle errors");
	}

	// =============================================================================
	// WP3 Failing Tests: Missing Dependency Detection
	// =============================================================================

	#[test]
	fn test_detect_missing_tool_dependency() {
		// tool_a depends on tool_nonexistent which doesn't exist
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![tool_with_deps(
				"tool_a",
				vec![("tool_nonexistent", DependencyType::Tool)],
			)],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_dependencies_exist();

		assert!(
			!result.is_ok(),
			"Expected missing dependency error"
		);
		assert!(
			result.errors.iter().any(|e| matches!(e,
				ValidationError::MissingDependency { tool, dependency, .. }
				if tool == "tool_a" && dependency == "tool_nonexistent"
			)),
			"Expected MissingDependency error for tool_nonexistent"
		);
	}

	#[test]
	fn test_detect_missing_agent_dependency() {
		// tool_a depends on agent_nonexistent which doesn't exist
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![tool_with_deps(
				"tool_a",
				vec![("agent_nonexistent", DependencyType::Agent)],
			)],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_dependencies_exist();

		assert!(
			!result.is_ok(),
			"Expected missing agent dependency error"
		);
		assert!(
			result.errors.iter().any(|e| matches!(e,
				ValidationError::MissingDependency { dep_type, .. }
				if dep_type == "agent"
			)),
			"Expected MissingDependency error with dep_type='agent'"
		);
	}

	#[test]
	fn test_valid_dependencies_exist() {
		// tool_a depends on tool_b, both exist
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_deps("tool_a", vec![("tool_b", DependencyType::Tool)]),
				simple_tool("tool_b"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_dependencies_exist();

		// This should pass - stubs return Ok, and it's valid anyway
		assert!(result.is_ok(), "Valid dependencies should pass");
	}

	// =============================================================================
	// WP3 Failing Tests: Schema Reference Validation
	// =============================================================================

	#[test]
	fn test_detect_missing_schema_ref() {
		// Tool references #/schemas/NonExistent which doesn't exist
		let mut tool = simple_tool("tool_a");
		tool.input_schema = Some(serde_json::json!({
			"$ref": "#/schemas/NonExistent"
		}));

		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![tool],
			schemas: vec![],  // No schemas defined!
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_schema_refs();

		assert!(
			!result.is_ok(),
			"Expected missing schema reference error"
		);
		assert!(
			result.errors.iter().any(|e| matches!(e,
				ValidationError::MissingSchemaRef { reference, .. }
				if reference == "#/schemas/NonExistent"
			)),
			"Expected MissingSchemaRef error"
		);
	}

	#[test]
	fn test_valid_schema_ref() {
		// Tool references #/schemas/WeatherInput which exists
		let mut tool = simple_tool("tool_a");
		tool.input_schema = Some(serde_json::json!({
			"$ref": "#/schemas/WeatherInput"
		}));

		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![tool],
			schemas: vec![Schema {
				name: "WeatherInput".to_string(),
				version: Some("1.0.0".to_string()),
				description: None,
				schema: serde_json::json!({"type": "object"}),
				metadata: HashMap::new(),
			}],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_schema_refs();

		assert!(result.is_ok(), "Valid schema ref should pass");
	}

	// =============================================================================
	// WP3 Failing Tests: Deprecation Warnings
	// =============================================================================

	#[test]
	fn test_warn_on_deprecated_tool_dependency() {
		// tool_a depends on tool_b which is deprecated
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_deps("tool_a", vec![("tool_b", DependencyType::Tool)]),
				deprecated_tool("tool_b", "Use tool_c instead"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_deprecations();

		// Deprecation should be a warning, not an error
		assert!(
			result.has_warnings(),
			"Expected deprecation warning"
		);
		assert!(
			result.warnings.iter().any(|w| w.message.contains("deprecated")),
			"Expected warning message about deprecation"
		);
	}

	#[test]
	fn test_warn_on_deprecated_server() {
		// Tool sources from a deprecated server
		let mut tool = simple_tool("tool_a");
		if let ToolImplementation::Source(ref mut source) = tool.implementation {
			source.target = "legacy-server".to_string();
		}

		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![tool],
			schemas: vec![],
			servers: vec![Server {
				name: "legacy-server".to_string(),
				version: Some("0.9.0".to_string()),
				description: None,
				provides: vec![],
				deprecated: true,
				deprecation_message: Some("Migrate to new-server v2.0".to_string()),
				metadata: HashMap::new(),
			}],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_deprecations();

		assert!(
			result.has_warnings(),
			"Expected deprecation warning for deprecated server"
		);
	}

	// =============================================================================
	// WP3 Failing Tests: Version Constraint Validation
	// =============================================================================

	#[test]
	fn test_version_constraint_not_satisfied() {
		// tool_a requires tool_b@>=2.0.0 but tool_b is 1.0.0
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_versioned_dep("tool_a", "tool_b", ">=2.0.0"),
				versioned_tool("tool_b", "1.0.0"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_version_constraints();

		assert!(
			!result.is_ok(),
			"Expected version constraint error"
		);
		assert!(
			result.errors.iter().any(|e| matches!(e, ValidationError::VersionMismatch { .. })),
			"Expected VersionMismatch error"
		);
	}

	#[test]
	fn test_version_constraint_satisfied() {
		// tool_a requires tool_b@>=1.0.0 and tool_b is 1.5.0
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_versioned_dep("tool_a", "tool_b", ">=1.0.0"),
				versioned_tool("tool_b", "1.5.0"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_version_constraints();

		// This should pass with proper implementation
		// Currently stub returns Ok, so it will pass
		assert!(result.is_ok(), "Valid version constraint should pass");
	}

	// =============================================================================
	// WP3 Failing Tests: Duplicate Name Detection
	// =============================================================================

	#[test]
	fn test_detect_duplicate_tool_names() {
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![simple_tool("my_tool"), simple_tool("my_tool")],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_unique_names();

		assert!(
			!result.is_ok(),
			"Expected duplicate tool name error"
		);
		assert!(
			result.errors.iter().any(|e| matches!(e, ValidationError::DuplicateToolName(name) if name == "my_tool")),
			"Expected DuplicateToolName error"
		);
	}

	#[test]
	fn test_detect_duplicate_schema_names() {
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![],
			schemas: vec![
				Schema {
					name: "MySchema".to_string(),
					version: Some("1.0.0".to_string()),
					description: None,
					schema: serde_json::json!({}),
					metadata: HashMap::new(),
				},
				Schema {
					name: "MySchema".to_string(),
					version: Some("2.0.0".to_string()),
					description: None,
					schema: serde_json::json!({}),
					metadata: HashMap::new(),
				},
			],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate_unique_names();

		assert!(
			!result.is_ok(),
			"Expected duplicate schema name error"
		);
	}

	// =============================================================================
	// WP3 Failing Tests: Full Validation Integration
	// =============================================================================

	#[test]
	fn test_full_validation_catches_multiple_errors() {
		// Registry with multiple problems:
		// - Duplicate tool name
		// - Missing dependency
		// - Cycle in remaining valid tools
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				simple_tool("dup_tool"),
				simple_tool("dup_tool"), // duplicate
				tool_with_deps("orphan", vec![("nonexistent", DependencyType::Tool)]), // missing dep
				tool_with_deps("cycle_a", vec![("cycle_b", DependencyType::Tool)]),
				tool_with_deps("cycle_b", vec![("cycle_a", DependencyType::Tool)]), // cycle
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = RegistryValidator::new(&registry).validate();

		assert!(
			!result.is_ok(),
			"Expected validation to catch multiple errors"
		);
		// Should have at least 3 errors (duplicate, missing dep, cycle)
		assert!(
			result.errors.len() >= 3,
			"Expected at least 3 errors, got {}",
			result.errors.len()
		);
	}

	#[test]
	fn test_valid_registry_passes_full_validation() {
		// A well-formed registry should pass all validation
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				simple_tool("tool_a"),
				tool_with_deps("tool_b", vec![("tool_a", DependencyType::Tool)]),
			],
			schemas: vec![Schema {
				name: "Input".to_string(),
				version: Some("1.0.0".to_string()),
				description: None,
				schema: serde_json::json!({"type": "object"}),
				metadata: HashMap::new(),
			}],
			servers: vec![Server {
				name: "backend".to_string(),
				version: Some("1.0.0".to_string()),
				description: None,
				provides: vec![],
				deprecated: false,
				deprecation_message: None,
				metadata: HashMap::new(),
			}],
			agents: vec![],
			unknown_caller_policy: Default::default(),
			metadata: HashMap::new(),
		};

		let result = validate_registry(&registry);

		// Valid registry should pass
		// Stub implementation returns Ok, so this passes
		assert!(result.is_ok(), "Valid registry should pass validation");
	}
}
