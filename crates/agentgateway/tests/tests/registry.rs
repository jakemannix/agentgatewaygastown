// Integration tests for the tool registry functionality

use std::collections::HashMap;

use agentgateway::mcp::registry::{
	CompiledRegistry, OutputField, OutputSchema, Registry, RegistryClient, RegistryStore,
	RegistryStoreRef, VirtualToolDef,
};
use tempfile::NamedTempFile;

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
	assert_eq!(registry.tools[0].source.target, "weather-backend");
	assert_eq!(registry.tools[0].source.tool, "fetch_weather_data");

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
	let registry_json = r#"{
		"schemaVersion": "1.0",
		"tools": [
			{
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
					"api_key": "test-key",
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
	assert_eq!(weather_tool.source.target, "weather");
	assert_eq!(weather_tool.source.tool, "fetch_weather");
	assert_eq!(
		weather_tool.description,
		Some("Get current weather for a city".to_string())
	);
	assert_eq!(weather_tool.defaults.len(), 2);
	assert_eq!(weather_tool.hide_fields.len(), 2);
	assert!(weather_tool.output_schema.is_some());
	assert_eq!(weather_tool.version, Some("2.1.0".to_string()));
	assert_eq!(weather_tool.metadata.len(), 2);

	// Check second tool
	let search_tool = &registry.tools[1];
	assert_eq!(search_tool.name, "search_web");
	assert_eq!(search_tool.source.target, "search");
	assert!(search_tool.defaults.is_empty());
	assert!(search_tool.hide_fields.is_empty());

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
