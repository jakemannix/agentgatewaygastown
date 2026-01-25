//! Golden tests for registry parsing
//!
//! These tests verify that registry JSON files can be parsed and key semantics
//! are preserved. They should pass with EITHER hand-written types OR proto-generated types.
//!
//! NOTE: We intentionally DO NOT test exact JSON round-trip matching because:
//! 1. The JSON format will change when we migrate to proto3 JSON serialization
//! 2. Current serde adds default values that weren't in original
//! 3. What matters is semantic equivalence, not byte-for-byte JSON match

use crate::mcp::registry::types::{Registry, ToolImplementation};

fn load_fixture(relative_path: &str) -> String {
	let manifest_dir = env!("CARGO_MANIFEST_DIR");
	let full_path = std::path::Path::new(manifest_dir).join(relative_path);
	std::fs::read_to_string(&full_path).unwrap_or_else(|e| {
		panic!("Failed to read fixture {}: {}", full_path.display(), e);
	})
}

fn parse_fixture(path: &str) -> Registry {
	let json = load_fixture(path);
	serde_json::from_str(&json).unwrap_or_else(|e| panic!("Failed to parse {}: {}", path, e))
}

// =============================================================================
// SEMANTIC TESTS - These test what matters for the migration
// =============================================================================

#[test]
fn test_minimal() {
	let r = parse_fixture("../../tests/fixtures/registry/minimal.json");
	assert_eq!(r.schema_version, "1.0");
	assert!(r.tools.is_empty());
}

#[test]
fn test_source_tools_parsed() {
	let r = parse_fixture("../../tests/fixtures/registry/v1-source-tools.json");
	assert_eq!(r.tools.len(), 2);
	assert!(matches!(&r.tools[0].implementation, ToolImplementation::Source(_)));
	assert!(matches!(&r.tools[1].implementation, ToolImplementation::Source(_)));
}

#[test]
fn test_pipeline_parsed() {
	let r = parse_fixture("../../tests/fixtures/registry/pipeline.json");
	assert_eq!(r.tools.len(), 1);
	assert!(matches!(&r.tools[0].implementation, ToolImplementation::Spec(_)));
}

#[test]
fn test_scatter_gather_parsed() {
	let r = parse_fixture("../../tests/fixtures/registry/scatter-gather.json");
	assert_eq!(r.tools.len(), 1);
	assert!(matches!(&r.tools[0].implementation, ToolImplementation::Spec(_)));
}

#[test]
fn test_filter_parsed() {
	let r = parse_fixture("../../tests/fixtures/registry/filter.json");
	assert_eq!(r.tools.len(), 3);
}

#[test]
fn test_schema_map_parsed() {
	let r = parse_fixture("../../tests/fixtures/registry/schema-map.json");
	assert_eq!(r.tools.len(), 1);
}

#[test]
fn test_map_each_parsed() {
	let r = parse_fixture("../../tests/fixtures/registry/map-each.json");
	assert_eq!(r.tools.len(), 2);
}

#[test]
fn test_output_transform_parsed() {
	let r = parse_fixture("../../tests/fixtures/registry/output-transform.json");
	assert!(r.tools[0].output_transform.is_some());
}

#[test]
fn test_v2_features_parsed() {
	let r = parse_fixture("../../tests/fixtures/registry/v2-full.json");
	assert_eq!(r.schema_version, "2.0");
	assert!(!r.schemas.is_empty());
	assert!(!r.servers.is_empty());
	assert!(!r.agents.is_empty());
}

#[test]
fn test_stateful_patterns_parsed() {
	// These have IR but no runtime - just verify parsing works
	let r = parse_fixture("../../tests/fixtures/registry/stateful-patterns.json");
	assert_eq!(r.tools.len(), 5);
}

#[test]
fn test_existing_demos_parse() {
	let demos = [
		"../../examples/pattern-demos/configs/registry.json",
		"../../examples/pattern-demos/configs/registry-v2-example.json",
	];
	for path in demos {
		let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
		if full.exists() {
			let _ = parse_fixture(path); // Just verify it parses
		}
	}
}

// =============================================================================
// PROTO-GENERATED TYPE TESTS
// These verify that the proto-generated types with pbjson serde work correctly
// =============================================================================

/// Test module for proto-generated registry types
mod proto_types {
	use crate::types::proto::registry as proto;

	/// Test that a minimal Registry can be parsed from proto3 JSON format
	#[test]
	fn test_proto_minimal_registry() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": []
		}"#;
		let registry: proto::Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.schema_version, "2.0");
		assert!(registry.tools.is_empty());
	}

	/// Test SourceTool parsing with proto3 JSON
	#[test]
	fn test_proto_source_tool() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": [{
				"name": "my_tool",
				"description": "A tool",
				"source": {
					"server": "backend",
					"tool": "actual_tool"
				}
			}]
		}"#;
		let registry: proto::Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.tools.len(), 1);
		let tool = &registry.tools[0];
		assert_eq!(tool.name, "my_tool");

		if let Some(proto::tool_definition::Implementation::Source(source)) = &tool.implementation {
			assert_eq!(source.server, "backend");
			assert_eq!(source.tool, "actual_tool");
		} else {
			panic!("Expected Source implementation");
		}
	}

	/// Test Pipeline parsing with proto3 JSON
	#[test]
	fn test_proto_pipeline() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": [{
				"name": "pipeline_tool",
				"spec": {
					"pipeline": {
						"steps": [{
							"id": "step1",
							"operation": {"tool": {"name": "fetch"}},
							"input": {"input": {"path": "$"}}
						}]
					}
				}
			}]
		}"#;
		let registry: proto::Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.tools.len(), 1);

		if let Some(proto::tool_definition::Implementation::Spec(spec)) = &registry.tools[0].implementation {
			if let Some(proto::pattern_spec::Pattern::Pipeline(pipeline)) = &spec.pattern {
				assert_eq!(pipeline.steps.len(), 1);
				assert_eq!(pipeline.steps[0].id, "step1");
			} else {
				panic!("Expected Pipeline pattern");
			}
		} else {
			panic!("Expected Spec implementation");
		}
	}

	/// Test ScatterGather parsing with proto3 JSON
	#[test]
	fn test_proto_scatter_gather() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": [{
				"name": "scatter_tool",
				"spec": {
					"scatterGather": {
						"targets": [
							{"tool": "tool1"},
							{"tool": "tool2"}
						],
						"aggregation": {
							"ops": [{"flatten": true}]
						}
					}
				}
			}]
		}"#;
		let registry: proto::Registry = serde_json::from_str(json).unwrap();

		if let Some(proto::tool_definition::Implementation::Spec(spec)) = &registry.tools[0].implementation {
			if let Some(proto::pattern_spec::Pattern::ScatterGather(sg)) = &spec.pattern {
				assert_eq!(sg.targets.len(), 2);
			} else {
				panic!("Expected ScatterGather pattern");
			}
		} else {
			panic!("Expected Spec implementation");
		}
	}

	/// Test that proto types can round-trip through JSON
	#[test]
	fn test_proto_roundtrip() {
		let registry = proto::Registry {
			schema_version: "2.0".to_string(),
			tools: vec![proto::ToolDefinition {
				name: "test_tool".to_string(),
				description: Some("A test tool".to_string()),
				implementation: Some(proto::tool_definition::Implementation::Source(proto::SourceTool {
					server: "backend".to_string(),
					tool: "actual".to_string(),
					defaults: Default::default(),
					hide_fields: vec![],
					server_version: None,
				})),
				input_schema: None,
				output_transform: None,
				version: None,
				metadata: Default::default(),
			}],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
		};

		// Serialize to JSON
		let json = serde_json::to_string_pretty(&registry).unwrap();

		// Parse back
		let parsed: proto::Registry = serde_json::from_str(&json).unwrap();

		assert_eq!(parsed.schema_version, "2.0");
		assert_eq!(parsed.tools.len(), 1);
		assert_eq!(parsed.tools[0].name, "test_tool");
	}

	/// Test ConstructBinding in pipeline (the new addition)
	#[test]
	fn test_proto_construct_binding() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": [{
				"name": "construct_test",
				"spec": {
					"pipeline": {
						"steps": [{
							"id": "step1",
							"operation": {"tool": {"name": "combine"}},
							"input": {
								"construct": {
									"fields": {
										"a": {"input": {"path": "$.x"}},
										"b": {"input": {"path": "$.y"}}
									}
								}
							}
						}]
					}
				}
			}]
		}"#;
		let registry: proto::Registry = serde_json::from_str(json).unwrap();

		if let Some(proto::tool_definition::Implementation::Spec(spec)) = &registry.tools[0].implementation {
			if let Some(proto::pattern_spec::Pattern::Pipeline(pipeline)) = &spec.pattern {
				let step = &pipeline.steps[0];
				if let Some(input) = &step.input {
					if let Some(proto::data_binding::Source::Construct(construct)) = &input.source {
						assert_eq!(construct.fields.len(), 2);
						assert!(construct.fields.contains_key("a"));
						assert!(construct.fields.contains_key("b"));
					} else {
						panic!("Expected Construct binding");
					}
				} else {
					panic!("Expected input binding");
				}
			} else {
				panic!("Expected Pipeline pattern");
			}
		} else {
			panic!("Expected Spec implementation");
		}
	}
}

// =============================================================================
// COMPATIBILITY TESTS
// Document differences between hand-written (v1) and proto-generated (v2) types
// =============================================================================

/// Documents the field naming differences between v1 and v2 formats
mod compatibility {
	use crate::mcp::registry::types as v1;
	use crate::types::proto::registry as proto;

	/// v1 uses "target", v2 uses "server" for SourceTool
	///
	/// v1 JSON format (current hand-written types):
	/// ```json
	/// {"source": {"target": "backend", "tool": "mytool"}}
	/// ```
	///
	/// v2 JSON format (proto-generated types):
	/// ```json
	/// {"source": {"server": "backend", "tool": "mytool"}}
	/// ```
	///
	/// The v1 types accept both "target" and "server" via serde alias.
	/// The v2 types only accept "server" (proto3 canonical format).
	#[test]
	fn test_source_tool_field_naming() {
		// v1 format with "target" - works with hand-written types
		let v1_json = r#"{
			"schemaVersion": "1.0",
			"tools": [{"name": "t", "source": {"target": "backend", "tool": "mytool"}}]
		}"#;
		let v1_registry: v1::Registry = serde_json::from_str(v1_json).unwrap();
		assert_eq!(v1_registry.tools[0].source_tool().unwrap().target, "backend");

		// v2 format with "server" - works with proto types
		let v2_json = r#"{
			"schemaVersion": "2.0",
			"tools": [{"name": "t", "source": {"server": "backend", "tool": "mytool"}}]
		}"#;
		let v2_registry: proto::Registry = serde_json::from_str(v2_json).unwrap();
		if let Some(proto::tool_definition::Implementation::Source(src)) = &v2_registry.tools[0].implementation
		{
			assert_eq!(src.server, "backend");
		} else {
			panic!("Expected Source");
		}

		// v1 types also accept "server" (via alias) - for forward compatibility
		let v1_with_server: v1::Registry = serde_json::from_str(v2_json).unwrap();
		assert_eq!(v1_with_server.tools[0].source_tool().unwrap().target, "backend");
	}

	/// v1 uses "stepId", v2 uses "step_id" (which becomes "stepId" in proto3 JSON)
	/// Both serialize to camelCase in JSON.
	#[test]
	fn test_step_binding_field_naming() {
		// Both formats use "stepId" in JSON (proto3 converts snake_case to camelCase)
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": [{
				"name": "t",
				"spec": {
					"pipeline": {
						"steps": [{
							"id": "s1",
							"operation": {"tool": {"name": "t1"}},
							"input": {"step": {"stepId": "s0", "path": "$"}}
						}]
					}
				}
			}]
		}"#;

		// Works with proto types
		let proto_registry: proto::Registry = serde_json::from_str(json).unwrap();
		if let Some(proto::tool_definition::Implementation::Spec(spec)) = &proto_registry.tools[0].implementation
		{
			if let Some(proto::pattern_spec::Pattern::Pipeline(p)) = &spec.pattern {
				if let Some(input) = &p.steps[0].input {
					if let Some(proto::data_binding::Source::Step(sb)) = &input.source {
						assert_eq!(sb.step_id, "s0");
					} else {
						panic!("Expected Step binding");
					}
				}
			}
		}

		// Works with v1 types
		let v1_registry: v1::Registry = serde_json::from_str(json).unwrap();
		assert!(!v1_registry.tools.is_empty());
	}

	/// Test that proto types serialize to canonical proto3 JSON format
	#[test]
	fn test_proto_serializes_to_canonical_format() {
		let registry = proto::Registry {
			schema_version: "2.0".to_string(),
			tools: vec![proto::ToolDefinition {
				name: "test".to_string(),
				description: None,
				implementation: Some(proto::tool_definition::Implementation::Source(proto::SourceTool {
					server: "my_backend".to_string(),
					tool: "my_tool".to_string(),
					defaults: Default::default(),
					hide_fields: vec!["secret".to_string()],
					server_version: None,
				})),
				input_schema: None,
				output_transform: None,
				version: None,
				metadata: Default::default(),
			}],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
		};

		let json = serde_json::to_value(&registry).unwrap();

		// Verify field names are proto3 canonical (camelCase)
		let tool = &json["tools"][0];
		let source = &tool["source"];
		assert!(source.get("server").is_some(), "Should have 'server' field");
		assert!(source.get("target").is_none(), "Should NOT have 'target' field");
		assert!(source.get("hideFields").is_some(), "Should have 'hideFields' (camelCase)");
	}

	/// Migration guide: Convert v1 JSON to v2 proto format
	///
	/// Key changes:
	/// 1. "target" → "server" in SourceTool
	/// 2. schemaVersion: "1.0" → "2.0"
	/// 3. All field names should be proto3 canonical (camelCase)
	#[test]
	fn test_migration_v1_to_v2() {
		// v1 input
		let v1_json = r#"{
			"schemaVersion": "1.0",
			"tools": [{
				"name": "my_tool",
				"source": {
					"target": "backend",
					"tool": "actual_tool",
					"defaults": {"key": "value"},
					"hideFields": ["secret"]
				}
			}]
		}"#;

		// Parse with v1 types
		let v1: v1::Registry = serde_json::from_str(v1_json).unwrap();

		// Manual migration to v2 (this would be automated in a migration script)
		let src = v1.tools[0].source_tool().unwrap();
		let v2 = proto::Registry {
			schema_version: "2.0".to_string(),
			tools: vec![proto::ToolDefinition {
				name: v1.tools[0].name.clone(),
				description: v1.tools[0].description.clone(),
				implementation: Some(proto::tool_definition::Implementation::Source(proto::SourceTool {
					server: src.target.clone(), // target → server
					tool: src.tool.clone(),
					defaults: src
						.defaults
						.clone()
						.into_iter()
						.map(|(k, v)| (k, json_to_proto_value(v)))
						.collect(),
					hide_fields: src.hide_fields.clone(),
					server_version: src.server_version.clone(),
				})),
				input_schema: v1.tools[0].input_schema.clone().map(|s| {
					prost_wkt_types::Struct {
						fields: serde_json::from_value::<std::collections::HashMap<String, serde_json::Value>>(s)
							.unwrap_or_default()
							.into_iter()
							.map(|(k, v)| (k, json_to_proto_value(v)))
							.collect(),
					}
				}),
				output_transform: None,
				version: None,
				metadata: Default::default(),
			}],
			schemas: vec![],
			servers: vec![],
			agents: vec![],
		};

		// Verify v2 format
		let v2_json = serde_json::to_value(&v2).unwrap();
		assert_eq!(v2_json["schemaVersion"], "2.0");
		assert_eq!(v2_json["tools"][0]["source"]["server"], "backend");
	}

	/// Helper to convert serde_json::Value to prost_wkt_types::Value
	fn json_to_proto_value(v: serde_json::Value) -> prost_wkt_types::Value {
		use prost_wkt_types::value::Kind;
		prost_wkt_types::Value {
			kind: Some(match v {
				serde_json::Value::Null => Kind::NullValue(0),
				serde_json::Value::Bool(b) => Kind::BoolValue(b),
				serde_json::Value::Number(n) => Kind::NumberValue(n.as_f64().unwrap_or(0.0)),
				serde_json::Value::String(s) => Kind::StringValue(s),
				serde_json::Value::Array(arr) => Kind::ListValue(prost_wkt_types::ListValue {
					values: arr.into_iter().map(json_to_proto_value).collect(),
				}),
				serde_json::Value::Object(obj) => Kind::StructValue(prost_wkt_types::Struct {
					fields: obj.into_iter().map(|(k, v)| (k, json_to_proto_value(v))).collect(),
				}),
			}),
		}
	}
}
