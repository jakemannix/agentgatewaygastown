//! Unit tests for the Enricher pattern
//!
//! These tests follow TDD - they are written first and will initially fail
//! until the implementation is complete.

use super::*;

/// Test basic deserialization of EnricherSpec with host backend
#[test]
fn test_enricher_spec_deserialization() {
	let json = r#"{
		"enrichments": [
			{
				"field": "user_data",
				"backend": {
					"host": "user-service.default.svc:8080"
				}
			}
		],
		"merge": "spread",
		"ignoreFailures": false
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	assert_eq!(spec.enrichments.len(), 1);
	assert_eq!(spec.enrichments[0].field.as_str(), "user_data");
	assert!(!spec.ignore_failures);
}

/// Test single enrichment source
#[test]
fn test_enrich_single_source() {
	let json = r#"{
		"enrichments": [
			{
				"field": "profile",
				"backend": {
					"host": "profile-service:8080"
				}
			}
		],
		"merge": "spread"
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	assert_eq!(spec.enrichments.len(), 1);
	assert_eq!(spec.enrichments[0].field.as_str(), "profile");
	assert!(matches!(spec.merge, MergeStrategy::Spread));
}

/// Test multiple enrichments in parallel
#[test]
fn test_enrich_multiple_parallel() {
	let json = r#"{
		"enrichments": [
			{
				"field": "user",
				"backend": {
					"host": "user-service:8080"
				}
			},
			{
				"field": "permissions",
				"backend": {
					"host": "auth-service:8080"
				}
			},
			{
				"field": "preferences",
				"backend": {
					"host": "prefs-service:8080"
				}
			}
		],
		"merge": "spread"
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	assert_eq!(spec.enrichments.len(), 3);
	assert_eq!(spec.enrichments[0].field.as_str(), "user");
	assert_eq!(spec.enrichments[1].field.as_str(), "permissions");
	assert_eq!(spec.enrichments[2].field.as_str(), "preferences");
}

/// Test merge strategy: spread into root object
#[test]
fn test_enrich_merge_spread() {
	let json = r#"{
		"enrichments": [],
		"merge": "spread"
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	assert!(matches!(spec.merge, MergeStrategy::Spread));
}

/// Test merge strategy: put under key
#[test]
fn test_enrich_merge_nested() {
	let json = r#"{
		"enrichments": [],
		"merge": {
			"nested": {
				"key": "enriched_data"
			}
		}
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	match spec.merge {
		MergeStrategy::Nested { key } => {
			assert_eq!(key.as_str(), "enriched_data");
		},
		_ => panic!("Expected Nested merge strategy"),
	}
}

/// Test ignore_failures flag
#[test]
fn test_enrich_ignore_failures() {
	let json = r#"{
		"enrichments": [
			{
				"field": "optional_data",
				"backend": {
					"host": "optional-service:8080"
				}
			}
		],
		"merge": "spread",
		"ignoreFailures": true
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	assert!(spec.ignore_failures);
}

/// Test fail on error (default behavior)
#[test]
fn test_enrich_fail_on_error() {
	let json = r#"{
		"enrichments": [
			{
				"field": "required_data",
				"backend": {
					"host": "required-service:8080"
				}
			}
		],
		"merge": "spread"
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	// Default should be false (fail on error)
	assert!(!spec.ignore_failures);
}

/// Test timeout configuration
#[test]
fn test_enrich_with_timeout() {
	let json = r#"{
		"enrichments": [],
		"merge": "spread",
		"timeoutMs": 5000
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	assert_eq!(spec.timeout_ms, Some(5000));
}

/// Test enrichment with input expression
#[test]
fn test_enrich_with_input_expression() {
	let json = r#"{
		"enrichments": [
			{
				"field": "user_details",
				"backend": {
					"host": "user-service:8080"
				},
				"input": "request.body.userId"
			}
		],
		"merge": "spread"
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	assert!(spec.enrichments[0].input.is_some());
}

/// Test schema map merge strategy
#[test]
fn test_enrich_schema_map() {
	let json = r#"{
		"enrichments": [
			{
				"field": "raw_user",
				"backend": {
					"host": "user-service:8080"
				}
			}
		],
		"merge": {
			"schemaMap": {
				"mappings": {
					"userName": "raw_user.name",
					"userEmail": "raw_user.email"
				}
			}
		}
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	match spec.merge {
		MergeStrategy::SchemaMap(schema) => {
			assert_eq!(schema.mappings.len(), 2);
		},
		_ => panic!("Expected SchemaMap merge strategy"),
	}
}

/// Test inline backend specification (host format)
#[test]
fn test_enrich_inline_backend() {
	let json = r#"{
		"enrichments": [
			{
				"field": "external_data",
				"backend": {
					"host": "api.example.com:443"
				}
			}
		],
		"merge": "spread"
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	assert_eq!(spec.enrichments.len(), 1);
}

/// Test serialization and verify structure
#[test]
fn test_enricher_serialization() {
	let json = r#"{
		"enrichments": [
			{
				"field": "data",
				"backend": {
					"host": "service:8080"
				}
			}
		],
		"merge": "spread",
		"ignoreFailures": true,
		"timeoutMs": 3000
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();

	// Verify deserialized values
	assert_eq!(spec.enrichments.len(), 1);
	assert_eq!(spec.enrichments[0].field.as_str(), "data");
	assert!(spec.ignore_failures);
	assert_eq!(spec.timeout_ms, Some(3000));

	// Verify serialization produces valid JSON
	let serialized = serde_json::to_string(&spec).unwrap();
	assert!(serialized.contains("\"field\":\"data\""));
	assert!(serialized.contains("\"ignoreFailures\":true"));
	assert!(serialized.contains("\"timeoutMs\":3000"));
}

/// Test backend reference format
#[test]
fn test_enrich_backend_reference() {
	let json = r#"{
		"enrichments": [
			{
				"field": "data",
				"backend": {
					"backend": "my-backend"
				}
			}
		],
		"merge": "spread"
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	assert_eq!(spec.enrichments.len(), 1);
}

/// Test service reference format
#[test]
fn test_enrich_service_reference() {
	let json = r#"{
		"enrichments": [
			{
				"field": "data",
				"backend": {
					"service": {
						"name": "namespace/my-service",
						"port": 8080
					}
				}
			}
		],
		"merge": "spread"
	}"#;

	let spec: EnricherSpec = serde_json::from_str(json).unwrap();
	assert_eq!(spec.enrichments.len(), 1);
}
