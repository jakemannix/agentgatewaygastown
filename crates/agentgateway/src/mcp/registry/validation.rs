// Registry Validation Module (WP3)
//
// Provides validation for Registry v2 including:
// - Dependency cycle detection
// - Missing dependency errors
// - Schema reference validation
// - Deprecation warnings
// - Version constraint validation

use super::types::Registry;
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
		// TODO(WP3): Implement full validation
		// This is a stub that returns Ok - tests will fail until implemented
		ValidationResult::ok()
	}

	/// Check for duplicate names (tools, schemas, servers, agents)
	pub fn validate_unique_names(&self) -> ValidationResult {
		// TODO(WP3): Implement duplicate name detection
		ValidationResult::ok()
	}

	/// Check for dependency cycles in tool definitions
	pub fn validate_no_cycles(&self) -> ValidationResult {
		// TODO(WP3): Implement cycle detection using DFS/topological sort
		ValidationResult::ok()
	}

	/// Check that all declared dependencies exist in the registry
	pub fn validate_dependencies_exist(&self) -> ValidationResult {
		// TODO(WP3): Implement dependency existence validation
		ValidationResult::ok()
	}

	/// Check that all schema $refs point to existing schemas
	pub fn validate_schema_refs(&self) -> ValidationResult {
		// TODO(WP3): Implement schema reference validation
		ValidationResult::ok()
	}

	/// Check for deprecated tool/server/agent usage and emit warnings
	pub fn validate_deprecations(&self) -> ValidationResult {
		// TODO(WP3): Implement deprecation warnings
		ValidationResult::ok()
	}

	/// Check version constraints on dependencies
	pub fn validate_version_constraints(&self) -> ValidationResult {
		// TODO(WP3): Implement version constraint validation
		ValidationResult::ok()
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
			metadata: HashMap::new(),
		};

		let result = validate_registry(&registry);

		// Valid registry should pass
		// Stub implementation returns Ok, so this passes
		assert!(result.is_ok(), "Valid registry should pass validation");
	}
}
