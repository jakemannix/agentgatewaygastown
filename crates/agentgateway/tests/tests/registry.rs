// Integration tests for the tool registry functionality

use std::collections::HashMap;

use agentgateway::mcp::registry::{
	CompiledRegistry, OutputField, OutputSchema, Registry, RegistryClient, RegistryStore,
	RegistryStoreRef, ToolDefinition, ToolImplementation, VirtualToolDef,
};
use tempfile::NamedTempFile;

/// Helper to extract source tool info from ToolImplementation
fn get_source_info(tool: &ToolDefinition) -> Option<(&str, &str)> {
	match &tool.implementation {
		ToolImplementation::Source(src) => Some((src.target.as_str(), src.tool.as_str())),
		_ => None,
	}
}

/// Helper to get defaults from a source tool
fn get_defaults(tool: &ToolDefinition) -> Option<&HashMap<String, serde_json::Value>> {
	match &tool.implementation {
		ToolImplementation::Source(src) => Some(&src.defaults),
		_ => None,
	}
}

/// Helper to get hide_fields from a source tool
fn get_hide_fields(tool: &ToolDefinition) -> Option<&Vec<String>> {
	match &tool.implementation {
		ToolImplementation::Source(src) => Some(&src.hide_fields),
		_ => None,
	}
}

/// Test loading a registry from a file source
#[tokio::test]
async fn test_registry_file_loading() -> anyhow::Result<()> {
	// Create a temporary registry file
	let registry_json = r#"{
		"schemaVersion": "1.0",
		"tools": [
			{
				"name": "get_weather",
				"source": {
					"target": "weather-backend",
					"tool": "fetch_weather_data"
				},
				"description": "Get weather for a city",
				"defaults": {
					"api_key": "test-key"
				},
				"hideFields": ["debug_mode"]
			}
		]
	}"#;

	let temp_file = NamedTempFile::with_suffix(".json")?;
	std::fs::write(temp_file.path(), registry_json)?;

	let file_uri = format!("file://{}", temp_file.path().display());

	// Create registry client
	let client =
		RegistryClient::from_uri(&file_uri, std::time::Duration::from_secs(300), None).unwrap();

	// Fetch and parse registry
	let registry = client.fetch().await?;

	assert_eq!(registry.len(), 1);
	assert_eq!(registry.tools[0].name, "get_weather");
	let (target, tool) = get_source_info(&registry.tools[0]).expect("should be source tool");
	assert_eq!(target, "weather-backend");
	assert_eq!(tool, "fetch_weather_data");

	Ok(())
}

/// Test registry compilation and tool lookup
#[tokio::test]
async fn test_registry_compilation() -> anyhow::Result<()> {
	let registry = Registry::with_tools(vec![
		VirtualToolDef::new("search", "search-backend", "web_search")
			.with_description("Search the web"),
		VirtualToolDef::new("get_weather", "weather-backend", "fetch_weather"),
	]);

	let compiled = CompiledRegistry::compile(registry)?;

	// Test lookup by virtual name
	assert!(compiled.get_tool("search").is_some());
	assert!(compiled.get_tool("get_weather").is_some());
	assert!(compiled.get_tool("unknown").is_none());

	// Test is_virtualized checks
	assert!(compiled.is_virtualized("search-backend", "web_search"));
	assert!(compiled.is_virtualized("weather-backend", "fetch_weather"));
	assert!(!compiled.is_virtualized("unknown", "unknown"));

	Ok(())
}

/// Test output transformation with JSONPath
#[tokio::test]
async fn test_output_transformation() -> anyhow::Result<()> {
	let mut properties = HashMap::new();
	properties.insert(
		"temperature".to_string(),
		OutputField::new("number", "$.data.temp"),
	);
	properties.insert(
		"conditions".to_string(),
		OutputField::new("string", "$.data.weather"),
	);

	let registry = Registry::with_tools(vec![VirtualToolDef::new(
		"get_weather",
		"weather-backend",
		"fetch_weather",
	)
	.with_output_schema(OutputSchema::new(properties))]);

	let compiled = CompiledRegistry::compile(registry)?;

	// Test output transformation
	let raw_output = serde_json::json!({
		"data": {
			"temp": 72.5,
			"weather": "sunny",
			"humidity": 45
		},
		"metadata": {
			"provider": "test"
		}
	});

	let transformed = compiled.transform_output("get_weather", raw_output)?;

	// Verify transformed output
	assert_eq!(transformed["temperature"], 72.5);
	assert_eq!(transformed["conditions"], "sunny");
	// metadata should not be in output (not in output schema)
	assert!(transformed.get("metadata").is_none());

	Ok(())
}

/// Test default injection in call arguments
#[tokio::test]
async fn test_default_injection() -> anyhow::Result<()> {
	let registry = Registry::with_tools(vec![VirtualToolDef::new(
		"get_weather",
		"weather-backend",
		"fetch_weather",
	)
	.with_default("api_key", serde_json::json!("secret-key"))
	.with_default("units", serde_json::json!("metric"))]);

	let compiled = CompiledRegistry::compile(registry)?;

	// Prepare call args with only user-provided fields
	let user_args = serde_json::json!({
		"location": "San Francisco"
	});

	let (target, tool_name, args) = compiled.prepare_call_args("get_weather", user_args.clone())?;

	// Check defaults were injected
	assert_eq!(target.as_str(), "weather-backend");
	assert_eq!(tool_name.as_str(), "fetch_weather");

	assert_eq!(args["location"], "San Francisco");
	assert_eq!(args["api_key"], "secret-key");
	assert_eq!(args["units"], "metric");

	Ok(())
}

/// Test registry store hot-reload functionality
#[tokio::test]
async fn test_registry_store_update() -> anyhow::Result<()> {
	let store = RegistryStore::new();
	let store_ref = RegistryStoreRef::new(store);

	// Initially empty
	assert!(!store_ref.has_registry());

	// Load a registry
	let registry = Registry::with_tools(vec![VirtualToolDef::new(
		"tool1",
		"backend1",
		"original_tool1",
	)]);

	store_ref.update(registry)?;
	assert!(store_ref.has_registry());

	// Verify tool is accessible
	{
		let guard = store_ref.get();
		let compiled = guard.as_ref().as_ref().unwrap();
		assert!(compiled.get_tool("tool1").is_some());
	}

	// Update with new registry
	let new_registry = Registry::with_tools(vec![
		VirtualToolDef::new("tool1", "backend1", "original_tool1"),
		VirtualToolDef::new("tool2", "backend2", "original_tool2"),
	]);

	store_ref.update(new_registry)?;

	// Verify both tools are now accessible
	{
		let guard = store_ref.get();
		let compiled = guard.as_ref().as_ref().unwrap();
		assert!(compiled.get_tool("tool1").is_some());
		assert!(compiled.get_tool("tool2").is_some());
	}

	Ok(())
}

/// Test registry with full JSON parsing from file
#[tokio::test]
async fn test_registry_full_json_parsing() -> anyhow::Result<()> {
	// Note: v2 format has defaults/hideFields inside the source object
	let registry_json = r#"{
		"schemaVersion": "2.0",
		"tools": [
			{
				"name": "get_weather",
				"source": {
					"target": "weather",
					"tool": "fetch_weather",
					"defaults": {
						"api_key": "test-key",
						"units": "metric"
					},
					"hideFields": ["debug_mode", "raw_output"]
				},
				"description": "Get current weather for a city",
				"inputSchema": {
					"type": "object",
					"properties": {
						"city": {"type": "string"}
					},
					"required": ["city"]
				},
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
			},
			{
				"name": "search_web",
				"source": {
					"target": "search",
					"tool": "web_search"
				},
				"description": "Search the web"
			}
		]
	}"#;

	let temp_file = NamedTempFile::with_suffix(".json")?;
	std::fs::write(temp_file.path(), registry_json)?;

	let file_uri = format!("file://{}", temp_file.path().display());
	let client = RegistryClient::from_uri(&file_uri, std::time::Duration::from_secs(300), None)?;

	let registry = client.fetch().await?;

	// Verify registry was parsed correctly
	assert_eq!(registry.len(), 2);

	// Check first tool
	let weather_tool = &registry.tools[0];
	assert_eq!(weather_tool.name, "get_weather");
	let (target, tool) = get_source_info(weather_tool).expect("should be source tool");
	assert_eq!(target, "weather");
	assert_eq!(tool, "fetch_weather");
	assert_eq!(
		weather_tool.description,
		Some("Get current weather for a city".to_string())
	);
	assert_eq!(get_defaults(weather_tool).unwrap().len(), 2);
	assert_eq!(get_hide_fields(weather_tool).unwrap().len(), 2);
	assert!(weather_tool.output_schema.is_some());
	assert_eq!(weather_tool.version, Some("2.1.0".to_string()));
	assert_eq!(weather_tool.metadata.len(), 2);

	// Check second tool
	let search_tool = &registry.tools[1];
	assert_eq!(search_tool.name, "search_web");
	let (target, _) = get_source_info(search_tool).expect("should be source tool");
	assert_eq!(target, "search");
	assert!(get_defaults(search_tool).unwrap().is_empty());
	assert!(get_hide_fields(search_tool).unwrap().is_empty());

	// Compile and verify lookups work
	let compiled = CompiledRegistry::compile(registry)?;
	assert!(compiled.get_tool("get_weather").is_some());
	assert!(compiled.get_tool("search_web").is_some());
	assert!(compiled.is_virtualized("weather", "fetch_weather"));
	assert!(compiled.is_virtualized("search", "web_search"));

	Ok(())
}

/// Test that prepare_call_args returns error for unknown tools
#[tokio::test]
async fn test_prepare_call_args_unknown_tool() -> anyhow::Result<()> {
	let registry = Registry::with_tools(vec![VirtualToolDef::new(
		"known_tool",
		"backend",
		"original",
	)]);

	let compiled = CompiledRegistry::compile(registry)?;

	let result = compiled.prepare_call_args("unknown_tool", serde_json::json!({}));
	assert!(result.is_err());

	Ok(())
}

/// Test output transformation with passthrough (no output schema)
#[tokio::test]
async fn test_output_transformation_passthrough() -> anyhow::Result<()> {
	let registry = Registry::with_tools(vec![VirtualToolDef::new(
		"simple_tool",
		"backend",
		"original",
	)
	// No output schema - should passthrough
	]);

	let compiled = CompiledRegistry::compile(registry)?;

	let original_output = serde_json::json!({
		"result": "test",
		"extra": 123
	});

	let transformed = compiled.transform_output("simple_tool", original_output.clone())?;

	// Should be identical (passthrough)
	assert_eq!(transformed, original_output);

	Ok(())
}

// =============================================================================
// Composition Tests
// =============================================================================

use agentgateway::mcp::registry::{
	PatternSpec, PipelineSpec, PipelineStep, StepOperation, ToolCall,
	ScatterGatherSpec, ScatterTarget, AggregationStrategy, AggregationOp,
};

/// Test parsing and compiling a composition-based tool
#[tokio::test]
async fn test_composition_parsing() -> anyhow::Result<()> {
	let registry_json = r#"{
		"schemaVersion": "1.0",
		"tools": [
			{
				"name": "research_pipeline",
				"description": "Multi-source research",
				"spec": {
					"pipeline": {
						"steps": [
							{
								"id": "search",
								"operation": { "tool": { "name": "web_search" } }
							},
							{
								"id": "summarize",
								"operation": { "tool": { "name": "summarize" } }
							}
						]
					}
				}
			}
		]
	}"#;

	let registry: Registry = serde_json::from_str(registry_json)?;
	assert_eq!(registry.len(), 1);

	let compiled = CompiledRegistry::compile(registry)?;
	assert!(compiled.is_composition("research_pipeline"));
	assert!(!compiled.is_source_tool("research_pipeline"));

	Ok(())
}

/// Test mixed registry with both source tools and compositions
#[tokio::test]
async fn test_mixed_registry() -> anyhow::Result<()> {
	// Create source tool
	let source_tool = ToolDefinition::source("get_weather", "weather", "fetch_weather");

	// Create composition
	let composition = ToolDefinition::composition(
		"multi_search",
		PatternSpec::ScatterGather(ScatterGatherSpec {
			targets: vec![
				ScatterTarget::Tool("search_web".to_string()),
				ScatterTarget::Tool("search_arxiv".to_string()),
			],
			aggregation: AggregationStrategy {
				ops: vec![AggregationOp::Flatten(true)],
			},
			timeout_ms: Some(5000),
			fail_fast: false,
		}),
	);

	let registry = Registry::with_tool_definitions(vec![source_tool, composition]);
	let compiled = CompiledRegistry::compile(registry)?;

	// Check source tool
	assert!(compiled.is_source_tool("get_weather"));
	assert!(!compiled.is_composition("get_weather"));
	assert!(compiled.is_virtualized("weather", "fetch_weather"));

	// Check composition
	assert!(compiled.is_composition("multi_search"));
	assert!(!compiled.is_source_tool("multi_search"));

	Ok(())
}

/// Test two-pass compilation with forward references
#[tokio::test]
async fn test_forward_reference_resolution() -> anyhow::Result<()> {
	// Composition references a tool defined after it
	let registry_json = r#"{
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

	let registry: Registry = serde_json::from_str(registry_json)?;
	let compiled = CompiledRegistry::compile(registry)?;

	// Both should exist and have correct types
	assert!(compiled.is_composition("pipeline"));
	assert!(compiled.is_source_tool("normalized_search"));

	Ok(())
}

/// Test composition tool references are resolved
#[tokio::test]
async fn test_composition_references() -> anyhow::Result<()> {
	let composition = ToolDefinition::composition(
		"research",
		PatternSpec::Pipeline(PipelineSpec {
			steps: vec![
				PipelineStep {
					id: "step1".to_string(),
					operation: StepOperation::Tool(ToolCall::new("search")),
					input: None,
				},
				PipelineStep {
					id: "step2".to_string(),
					operation: StepOperation::Tool(ToolCall::new("process")),
					input: None,
				},
			],
		}),
	);

	let registry = Registry::with_tool_definitions(vec![composition]);
	let compiled = CompiledRegistry::compile(registry)?;

	let tool = compiled.get_tool("research").unwrap();
	let comp_info = tool.composition_info().unwrap();

	// Check references were collected
	assert!(comp_info.resolved_references.contains(&"search".to_string()));
	assert!(comp_info.resolved_references.contains(&"process".to_string()));

	Ok(())
}

/// Test duplicate tool name detection
#[tokio::test]
async fn test_duplicate_tool_name_error() -> anyhow::Result<()> {
	let registry_json = r#"{
		"tools": [
			{ "name": "duplicate", "source": { "target": "a", "tool": "a" } },
			{ "name": "duplicate", "source": { "target": "b", "tool": "b" } }
		]
	}"#;

	let registry: Registry = serde_json::from_str(registry_json)?;
	let result = CompiledRegistry::compile(registry);

	assert!(result.is_err());
	let err = result.unwrap_err();
	assert!(err.to_string().contains("duplicate"));

	Ok(())
}

/// Test composition with output transform
#[tokio::test]
async fn test_composition_output_transform() -> anyhow::Result<()> {
	let registry_json = r#"{
		"tools": [
			{
				"name": "normalized_search",
				"spec": {
					"schemaMap": {
						"mappings": {
							"title": { "path": "$.result.name" }
						}
					}
				},
				"outputTransform": {
					"mappings": {
						"final_title": { "path": "$.title" },
						"source": { "literal": { "stringValue": "processed" } }
					}
				}
			}
		]
	}"#;

	let registry: Registry = serde_json::from_str(registry_json)?;
	let compiled = CompiledRegistry::compile(registry)?;

	assert!(compiled.is_composition("normalized_search"));

	// The composition has an output transform
	let tool = compiled.get_tool("normalized_search").unwrap();
	let comp_info = tool.composition_info().unwrap();
	assert!(comp_info.output_transform.is_some());

	Ok(())
}

/// Test all pattern types can be parsed
#[tokio::test]
async fn test_all_pattern_types_parsing() -> anyhow::Result<()> {
	let registry_json = r#"{
		"tools": [
			{
				"name": "pipeline_test",
				"spec": {
					"pipeline": {
						"steps": [
							{ "id": "s1", "operation": { "tool": { "name": "tool1" } } }
						]
					}
				}
			},
			{
				"name": "scatter_test",
				"spec": {
					"scatterGather": {
						"targets": [{ "tool": "t1" }, { "tool": "t2" }],
						"aggregation": { "ops": [{ "flatten": true }] }
					}
				}
			},
			{
				"name": "filter_test",
				"spec": {
					"filter": {
						"predicate": {
							"field": "$.score",
							"op": "gt",
							"value": { "numberValue": 0.5 }
						}
					}
				}
			},
			{
				"name": "schema_map_test",
				"spec": {
					"schemaMap": {
						"mappings": {
							"title": { "path": "$.name" }
						}
					}
				}
			},
			{
				"name": "map_each_test",
				"spec": {
					"mapEach": {
						"inner": { "tool": "process" }
					}
				}
			}
		]
	}"#;

	let registry: Registry = serde_json::from_str(registry_json)?;
	assert_eq!(registry.len(), 5);

	let compiled = CompiledRegistry::compile(registry)?;

	// All should be compositions
	assert!(compiled.is_composition("pipeline_test"));
	assert!(compiled.is_composition("scatter_test"));
	assert!(compiled.is_composition("filter_test"));
	assert!(compiled.is_composition("schema_map_test"));
	assert!(compiled.is_composition("map_each_test"));

	Ok(())
}

/// Test prepare_call_args fails for compositions
#[tokio::test]
async fn test_prepare_call_args_composition_error() -> anyhow::Result<()> {
	let composition = ToolDefinition::composition(
		"my_composition",
		PatternSpec::Pipeline(PipelineSpec { steps: vec![] }),
	);

	let registry = Registry::with_tool_definitions(vec![composition]);
	let compiled = CompiledRegistry::compile(registry)?;

	// Should error because compositions require the executor
	let result = compiled.prepare_call_args("my_composition", serde_json::json!({}));
	assert!(result.is_err());

	Ok(())
}

// =============================================================================
// Registry v2 Demo Config Integration Tests
// =============================================================================

use agentgateway::mcp::registry::{validate_registry, ValidationError};
use std::path::PathBuf;

/// Get path to the demo registry file
fn demo_registry_path() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../examples/pattern-demos/configs/registry-v2-example.json")
}

/// Test loading and parsing the full v2 demo registry
#[tokio::test]
async fn test_load_demo_registry_v2() -> anyhow::Result<()> {
	let path = demo_registry_path();
	let content = std::fs::read_to_string(&path)?;
	let registry: Registry = serde_json::from_str(&content)?;

	// Verify counts match expected
	assert_eq!(registry.schemas.len(), 12, "Expected 12 schemas");
	assert_eq!(registry.servers.len(), 10, "Expected 10 servers");
	assert_eq!(registry.tools.len(), 27, "Expected 27 tools");
	assert_eq!(registry.agents.len(), 5, "Expected 5 agents");

	Ok(())
}

/// Test that the demo registry passes validation
#[tokio::test]
async fn test_demo_registry_validation() -> anyhow::Result<()> {
	let path = demo_registry_path();
	let content = std::fs::read_to_string(&path)?;
	let registry: Registry = serde_json::from_str(&content)?;

	let result = validate_registry(&registry);

	// The demo registry should be valid (no errors)
	assert!(
		result.is_ok(),
		"Demo registry should pass validation, got errors: {:?}",
		result.errors
	);

	// It may have warnings (e.g., deprecations), that's OK
	if result.has_warnings() {
		println!(
			"Demo registry has {} warnings (expected for deprecation demos)",
			result.warnings.len()
		);
	}

	Ok(())
}

/// Test that the demo registry compiles successfully
#[tokio::test]
async fn test_demo_registry_compilation() -> anyhow::Result<()> {
	let path = demo_registry_path();
	let content = std::fs::read_to_string(&path)?;
	let registry: Registry = serde_json::from_str(&content)?;

	let compiled = CompiledRegistry::compile(registry)?;

	// Verify some key tools exist
	assert!(
		compiled.get_tool("search_documents").is_some(),
		"search_documents tool should exist"
	);
	assert!(compiled.get_tool("fetch").is_some(), "fetch tool should exist");

	// Verify compositions exist (tools with spec.pipeline/scatterGather/saga)
	assert!(
		compiled.is_composition("fetch_and_store"),
		"fetch_and_store should be a composition"
	);

	Ok(())
}

/// Test v2 features: schema references, servers, agents
#[tokio::test]
async fn test_demo_registry_v2_features() -> anyhow::Result<()> {
	let path = demo_registry_path();
	let content = std::fs::read_to_string(&path)?;
	let registry: Registry = serde_json::from_str(&content)?;

	// Check schema features
	let search_query_schema = registry.schemas.iter().find(|s| s.name == "SearchQuery");
	assert!(search_query_schema.is_some(), "SearchQuery schema should exist");
	assert_eq!(
		search_query_schema.unwrap().version,
		Some("1.0.0".to_string()),
		"Schema should be versioned"
	);

	// Check server features
	let doc_server = registry.servers.iter().find(|s| s.name == "document-service");
	assert!(doc_server.is_some(), "document-service should exist");
	assert!(
		doc_server.unwrap().version.is_some(),
		"Server should be versioned"
	);

	// Check agent features
	let research_agent = registry.agents.iter().find(|a| a.name == "research-agent");
	assert!(research_agent.is_some(), "research-agent should exist");
	assert!(
		!research_agent.unwrap().skills.is_empty(),
		"Agent should have skills"
	);

	Ok(())
}

/// Test validation catches cycles when present
#[tokio::test]
async fn test_validation_detects_cycle_in_demo_extension() -> anyhow::Result<()> {
	let path = demo_registry_path();
	let content = std::fs::read_to_string(&path)?;
	let mut registry: Registry = serde_json::from_str(&content)?;

	// Add a cycle: make a tool depend on itself via another tool
	// Find two tools and create a cycle
	if registry.tools.len() >= 2 {
		use agentgateway::mcp::registry::{Dependency, DependencyType};

		// Create cycle: tool_0 -> tool_1 -> tool_0
		let tool_0_name = registry.tools[0].name.clone();
		let tool_1_name = registry.tools[1].name.clone();

		registry.tools[0].depends.push(Dependency {
			dep_type: DependencyType::Tool,
			name: tool_1_name.clone(),
			version: None,
			skill: None,
		});
		registry.tools[1].depends.push(Dependency {
			dep_type: DependencyType::Tool,
			name: tool_0_name.clone(),
			version: None,
			skill: None,
		});

		let result = validate_registry(&registry);

		// Should detect the cycle
		assert!(
			!result.is_ok(),
			"Should detect cycle we introduced"
		);
		assert!(
			result.errors.iter().any(|e| matches!(e, ValidationError::DependencyCycle(_))),
			"Should have DependencyCycle error"
		);
	}

	Ok(())
}

/// Test runtime hooks with demo registry
#[tokio::test]
async fn test_demo_registry_runtime_hooks() -> anyhow::Result<()> {
	use agentgateway::mcp::registry::{RuntimeHooks, CallerIdentity};

	let path = demo_registry_path();
	let content = std::fs::read_to_string(&path)?;
	let registry: Registry = serde_json::from_str(&content)?;

	let hooks = RuntimeHooks::new(&registry);

	// Anonymous caller should see all tools
	let anonymous = CallerIdentity::default();
	let visible = hooks.get_visible_tools(&anonymous);
	assert_eq!(visible.len(), 27, "Anonymous caller should see all 27 tools");

	// Caller with declared deps should only see those deps
	let restricted = CallerIdentity {
		agent_name: Some("test-agent".to_string()),
		agent_version: Some("1.0.0".to_string()),
		declared_deps: ["search_documents", "fetch"]
			.iter()
			.map(|s| s.to_string())
			.collect(),
	};
	let visible = hooks.get_visible_tools(&restricted);
	assert_eq!(visible.len(), 2, "Restricted caller should only see 2 tools");

	Ok(())
}

/// Test that the ecommerce demo registry's composition tools are correctly parsed
#[tokio::test]
async fn test_ecommerce_registry_compositions() -> anyhow::Result<()> {
	use std::path::PathBuf;

	let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../examples/ecommerce-demo/gateway-configs/ecommerce_registry_v2.json");
	let content = std::fs::read_to_string(&path)?;
	let registry: Registry = serde_json::from_str(&content)?;

	// Find composition tools
	let compositions: Vec<_> = registry
		.tools
		.iter()
		.filter(|t| t.is_composition())
		.collect();

	// Should have personalized_search and product_with_availability
	assert!(
		compositions.len() >= 2,
		"Expected at least 2 compositions, got {}. Tools: {:?}",
		compositions.len(),
		registry.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
	);

	let composition_names: Vec<_> = compositions.iter().map(|c| c.name.as_str()).collect();
	assert!(
		composition_names.contains(&"personalized_search"),
		"personalized_search should be a composition, found: {:?}",
		composition_names
	);
	assert!(
		composition_names.contains(&"product_with_availability"),
		"product_with_availability should be a composition, found: {:?}",
		composition_names
	);

	// Verify they compile successfully
	let compiled = CompiledRegistry::compile(registry)?;

	// Check compositions are accessible and recognized
	assert!(
		compiled.is_composition("personalized_search"),
		"personalized_search should be recognized as composition after compilation"
	);
	assert!(
		compiled.is_composition("product_with_availability"),
		"product_with_availability should be recognized as composition after compilation"
	);

	Ok(())
}
