// Runtime Hooks Module (WP4)
//
// Provides runtime hooks for Registry v2 including:
// - Pre-call dependency checking (verify dependencies before execution)
// - Caller context injection (add caller identity to execution context)
// - Dependency resolution at call time
// - Dependency-scoped tool discovery (WP11 integration)

use std::collections::HashSet;

use super::types::{DependencyType, Registry, ToolDefinition};

/// Caller identity extracted from requests (WP10 integration)
#[derive(Debug, Clone, PartialEq)]
pub struct CallerIdentity {
	/// Agent name if caller is a registered agent
	pub agent_name: Option<String>,
	/// Agent version if known
	pub agent_version: Option<String>,
	/// Declared dependencies from agent's registration
	pub declared_deps: HashSet<String>,
}

impl Default for CallerIdentity {
	fn default() -> Self {
		Self {
			agent_name: None,
			agent_version: None,
			declared_deps: HashSet::new(),
		}
	}
}

/// Execution context passed to tool invocations
#[derive(Debug, Clone)]
pub struct CallContext {
	/// The caller's identity
	pub caller: CallerIdentity,
	/// Registry for dependency lookups
	pub registry_version: String,
}

/// Result of a pre-call dependency check
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyCheckResult {
	/// All dependencies satisfied
	Ok,
	/// Dependency not declared by caller
	UndeclaredDependency {
		tool: String,
		dependency: String,
		dep_type: DependencyType,
	},
	/// Dependency not found in registry
	MissingDependency {
		tool: String,
		dependency: String,
		dep_type: DependencyType,
	},
	/// Dependency version mismatch
	VersionMismatch {
		tool: String,
		dependency: String,
		required: String,
		available: String,
	},
	/// Tool itself is not accessible to caller
	ToolNotAccessible { tool: String, reason: String },
}

/// Tool visibility result for dependency-scoped discovery
#[derive(Debug, Clone, PartialEq)]
pub struct ToolVisibility {
	/// Whether the tool is visible to the caller
	pub visible: bool,
	/// If not visible, the reason
	pub reason: Option<String>,
}

/// Runtime hooks for dependency checking and context injection
pub struct RuntimeHooks<'a> {
	registry: &'a Registry,
}

impl<'a> RuntimeHooks<'a> {
	/// Create a new RuntimeHooks instance
	pub fn new(registry: &'a Registry) -> Self {
		Self { registry }
	}

	/// Check if a tool's dependencies are satisfied before execution
	///
	/// Returns Ok if all dependencies are available and the caller has
	/// declared them. Returns an error describing the first unsatisfied
	/// dependency.
	pub fn check_pre_call_dependencies(
		&self,
		_tool_name: &str,
		_caller: &CallerIdentity,
	) -> DependencyCheckResult {
		// TODO(WP4): Implement pre-call dependency checking
		// - Find tool in registry
		// - Check each dependency exists
		// - Check caller has declared each dependency
		// - Check version constraints
		DependencyCheckResult::Ok
	}

	/// Get tools visible to a specific caller based on their declared dependencies
	///
	/// This implements dependency-scoped discovery (WP11):
	/// - Agents only see tools they've declared as dependencies
	/// - Plus tools that have no dependencies themselves (leaf tools)
	pub fn get_visible_tools(&self, _caller: &CallerIdentity) -> Vec<&ToolDefinition> {
		// TODO(WP4): Implement dependency-scoped discovery
		// - If caller has no declared deps, return all tools (backwards compat)
		// - Otherwise, filter to tools in declared_deps + leaf tools
		self.registry.tools.iter().collect()
	}

	/// Check if a specific tool is visible to a caller
	pub fn is_tool_visible(&self, _tool_name: &str, _caller: &CallerIdentity) -> ToolVisibility {
		// TODO(WP4): Implement tool visibility check
		ToolVisibility {
			visible: true,
			reason: None,
		}
	}

	/// Resolve all dependencies for a tool, returning them in execution order
	///
	/// This performs a topological sort of dependencies to determine
	/// the order in which they should be resolved/initialized.
	pub fn resolve_dependency_order(&self, tool_name: &str) -> Result<Vec<String>, String> {
		// TODO(WP4): Implement topological sort of dependencies
		Ok(vec![tool_name.to_string()])
	}

	/// Create an execution context for a tool invocation
	pub fn create_context(&self, caller: CallerIdentity) -> CallContext {
		CallContext {
			caller,
			registry_version: self.registry.schema_version.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;
	use crate::mcp::registry::types::{
		Dependency, Registry, SourceTool, ToolDefinition, ToolImplementation,
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

	fn simple_tool(name: &str) -> ToolDefinition {
		tool_with_deps(name, vec![])
	}

	fn caller_with_deps(deps: &[&str]) -> CallerIdentity {
		CallerIdentity {
			agent_name: Some("test-agent".to_string()),
			agent_version: Some("1.0.0".to_string()),
			declared_deps: deps.iter().map(|s| s.to_string()).collect(),
		}
	}

	fn anonymous_caller() -> CallerIdentity {
		CallerIdentity::default()
	}

	// =============================================================================
	// WP4 Failing Tests: Pre-call Dependency Checking
	// =============================================================================

	#[test]
	fn test_pre_call_check_missing_dependency_declaration() {
		// Tool depends on "search", but caller hasn't declared "search" as a dependency
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_deps("research", vec![("search", DependencyType::Tool)]),
				simple_tool("search"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		let caller = caller_with_deps(&[]); // Caller hasn't declared any deps

		let result = hooks.check_pre_call_dependencies("research", &caller);

		// Should fail because caller hasn't declared "search" dependency
		assert!(
			matches!(result, DependencyCheckResult::UndeclaredDependency { .. }),
			"Expected UndeclaredDependency, got {:?}",
			result
		);
	}

	#[test]
	fn test_pre_call_check_passes_when_deps_declared() {
		// Tool depends on "search", and caller has declared "search"
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_deps("research", vec![("search", DependencyType::Tool)]),
				simple_tool("search"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		let caller = caller_with_deps(&["search", "research"]);

		let result = hooks.check_pre_call_dependencies("research", &caller);

		// Should pass because caller has declared the dependency
		assert_eq!(result, DependencyCheckResult::Ok);
	}

	#[test]
	fn test_pre_call_check_transitive_dependencies() {
		// A -> B -> C: calling A requires B and C to be declared
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_deps("tool_a", vec![("tool_b", DependencyType::Tool)]),
				tool_with_deps("tool_b", vec![("tool_c", DependencyType::Tool)]),
				simple_tool("tool_c"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		// Caller declares A and B but not C (transitive dep)
		let caller = caller_with_deps(&["tool_a", "tool_b"]);

		let result = hooks.check_pre_call_dependencies("tool_a", &caller);

		// Should fail because C is a transitive dependency
		assert!(
			matches!(&result, DependencyCheckResult::UndeclaredDependency { dependency, .. } if dependency == "tool_c"),
			"Expected UndeclaredDependency for tool_c, got {:?}",
			result
		);
	}

	#[test]
	fn test_pre_call_check_missing_tool_in_registry() {
		// Tool declares dependency on non-existent tool
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_deps("broken", vec![("nonexistent", DependencyType::Tool)]),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		let caller = caller_with_deps(&["broken", "nonexistent"]);

		let result = hooks.check_pre_call_dependencies("broken", &caller);

		// Should fail because "nonexistent" doesn't exist in registry
		assert!(
			matches!(result, DependencyCheckResult::MissingDependency { .. }),
			"Expected MissingDependency, got {:?}",
			result
		);
	}

	#[test]
	fn test_pre_call_check_version_mismatch() {
		// Tool requires search@>=2.0.0 but registry has 1.0.0
		let mut search_tool = simple_tool("search");
		search_tool.version = Some("1.0.0".to_string());

		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_versioned_dep("research", "search", ">=2.0.0"),
				search_tool,
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		let caller = caller_with_deps(&["research", "search"]);

		let result = hooks.check_pre_call_dependencies("research", &caller);

		// Should fail due to version mismatch
		assert!(
			matches!(result, DependencyCheckResult::VersionMismatch { .. }),
			"Expected VersionMismatch, got {:?}",
			result
		);
	}

	#[test]
	fn test_pre_call_anonymous_caller_can_call_leaf_tools() {
		// Anonymous callers (no declared deps) can call tools with no dependencies
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				simple_tool("leaf_tool"),
				tool_with_deps("complex_tool", vec![("leaf_tool", DependencyType::Tool)]),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		let caller = anonymous_caller();

		// Anonymous can call leaf tool
		let result = hooks.check_pre_call_dependencies("leaf_tool", &caller);
		assert_eq!(result, DependencyCheckResult::Ok, "Anonymous should call leaf tools");

		// But not complex tool with dependencies
		let result = hooks.check_pre_call_dependencies("complex_tool", &caller);
		assert!(
			!matches!(result, DependencyCheckResult::Ok),
			"Anonymous should not call tools with undeclared dependencies"
		);
	}

	// =============================================================================
	// WP4 Failing Tests: Dependency-Scoped Discovery (WP11 integration)
	// =============================================================================

	#[test]
	fn test_visibility_filters_by_declared_deps() {
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				simple_tool("search"),
				simple_tool("fetch"),
				simple_tool("summarize"),
				tool_with_deps("research", vec![("search", DependencyType::Tool)]),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		// Caller only declares "search" and "research"
		let caller = caller_with_deps(&["search", "research"]);

		let visible = hooks.get_visible_tools(&caller);
		let visible_names: HashSet<_> = visible.iter().map(|t| t.name.as_str()).collect();

		// Should only see "search" and "research", not "fetch" or "summarize"
		assert!(
			visible_names.contains("search"),
			"Should see declared dep 'search'"
		);
		assert!(
			visible_names.contains("research"),
			"Should see declared dep 'research'"
		);
		assert!(
			!visible_names.contains("fetch"),
			"Should NOT see undeclared 'fetch'"
		);
		assert!(
			!visible_names.contains("summarize"),
			"Should NOT see undeclared 'summarize'"
		);
	}

	#[test]
	fn test_visibility_anonymous_sees_all() {
		// Backwards compatibility: anonymous callers see all tools
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				simple_tool("search"),
				simple_tool("fetch"),
				simple_tool("summarize"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		let caller = anonymous_caller();

		let visible = hooks.get_visible_tools(&caller);

		// Anonymous caller should see all tools (backwards compat)
		assert_eq!(visible.len(), 3, "Anonymous should see all tools");
	}

	#[test]
	fn test_visibility_single_tool_check() {
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				simple_tool("search"),
				simple_tool("secret_tool"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		let caller = caller_with_deps(&["search"]);

		let search_vis = hooks.is_tool_visible("search", &caller);
		let secret_vis = hooks.is_tool_visible("secret_tool", &caller);

		assert!(search_vis.visible, "Should see declared 'search'");
		assert!(
			!secret_vis.visible,
			"Should NOT see undeclared 'secret_tool'"
		);
		assert!(
			secret_vis.reason.is_some(),
			"Should have reason for invisibility"
		);
	}

	// =============================================================================
	// WP4 Failing Tests: Dependency Resolution Order
	// =============================================================================

	#[test]
	fn test_resolve_dependency_order_simple() {
		// A -> B -> C should resolve as [C, B, A]
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![
				tool_with_deps("tool_a", vec![("tool_b", DependencyType::Tool)]),
				tool_with_deps("tool_b", vec![("tool_c", DependencyType::Tool)]),
				simple_tool("tool_c"),
			],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		let order = hooks.resolve_dependency_order("tool_a").unwrap();

		// Dependencies should come before dependents
		let c_idx = order.iter().position(|n| n == "tool_c");
		let b_idx = order.iter().position(|n| n == "tool_b");
		let a_idx = order.iter().position(|n| n == "tool_a");

		assert!(c_idx.is_some() && b_idx.is_some() && a_idx.is_some());
		assert!(
			c_idx.unwrap() < b_idx.unwrap(),
			"C should come before B"
		);
		assert!(
			b_idx.unwrap() < a_idx.unwrap(),
			"B should come before A"
		);
	}

	#[test]
	fn test_resolve_dependency_order_diamond() {
		// Diamond: A -> B, A -> C, B -> D, C -> D
		// Valid order: D, B, C, A (or D, C, B, A)
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

		let hooks = RuntimeHooks::new(&registry);
		let order = hooks.resolve_dependency_order("tool_a").unwrap();

		let d_idx = order.iter().position(|n| n == "tool_d").unwrap();
		let b_idx = order.iter().position(|n| n == "tool_b").unwrap();
		let c_idx = order.iter().position(|n| n == "tool_c").unwrap();
		let a_idx = order.iter().position(|n| n == "tool_a").unwrap();

		// D must come before B, C, A
		assert!(d_idx < b_idx, "D should come before B");
		assert!(d_idx < c_idx, "D should come before C");
		assert!(d_idx < a_idx, "D should come before A");
		// A must come last
		assert!(a_idx > b_idx && a_idx > c_idx, "A should come after B and C");
	}

	#[test]
	fn test_resolve_dependency_order_handles_cycle() {
		// Cycle: A -> B -> A (should error, not hang)
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

		let hooks = RuntimeHooks::new(&registry);
		let result = hooks.resolve_dependency_order("tool_a");

		assert!(result.is_err(), "Cycle should return error, not hang");
	}

	// =============================================================================
	// WP4 Failing Tests: Execution Context
	// =============================================================================

	#[test]
	fn test_create_context_includes_caller_info() {
		let registry = Registry {
			schema_version: "2.0".to_string(),
			tools: vec![],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
			metadata: HashMap::new(),
		};

		let hooks = RuntimeHooks::new(&registry);
		let caller = CallerIdentity {
			agent_name: Some("my-agent".to_string()),
			agent_version: Some("1.0.0".to_string()),
			declared_deps: ["search", "fetch"].iter().map(|s| s.to_string()).collect(),
		};

		let ctx = hooks.create_context(caller.clone());

		assert_eq!(ctx.caller.agent_name, Some("my-agent".to_string()));
		assert_eq!(ctx.registry_version, "2.0");
		assert!(ctx.caller.declared_deps.contains("search"));
	}
}
