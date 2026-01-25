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
