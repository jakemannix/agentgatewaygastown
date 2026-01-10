// Integration tests for the tool registry functionality

use std::collections::HashMap;

use agentgateway::mcp::registry::{
	CompiledRegistry, OutputField, OutputSchema, Registry, RegistryClient, RegistryStore,
	RegistryStoreRef, ServerDef, ToolDef,
};
use tempfile::NamedTempFile;

/// Test loading a registry from a file source
#[tokio::test]
async fn test_registry_file_loading() -> anyhow::Result<()> {
	// Create a temporary registry file with the new unified format
	let registry_json = r#"{
		"schemaVersion": "1.0",
		"servers": [
			{
				"name": "weather-backend",
				"stdio": {"command": "weather-cli", "args": []}
			}
		],
		"tools": [
			{
				"name": "fetch_weather_data",
				"server": "weather-backend",
				"description": "Backend weather tool"
			},
			{
				"name": "get_weather",
				"source": "fetch_weather_data",
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

	assert_eq!(registry.len(), 2);
	assert_eq!(registry.servers.len(), 1);
	assert_eq!(registry.tools[0].name, "fetch_weather_data");
	assert_eq!(registry.tools[0].server, Some("weather-backend".to_string()));
	assert_eq!(registry.tools[1].name, "get_weather");
	assert_eq!(registry.tools[1].source, Some("fetch_weather_data".to_string()));

	Ok(())
}

/// Test registry compilation and tool lookup
#[tokio::test]
async fn test_registry_compilation() -> anyhow::Result<()> {
	let servers = vec![
		ServerDef::stdio("search-backend", "search-cli", vec![]),
		ServerDef::stdio("weather-backend", "weather-cli", vec![]),
	];
	let tools = vec![
		ToolDef::base("web_search", "search-backend"),
		ToolDef::base("fetch_weather", "weather-backend"),
		ToolDef::virtual_tool("search", "web_search").with_description("Search the web"),
		ToolDef::virtual_tool("get_weather", "fetch_weather"),
	];
	let registry = Registry::with_servers_and_tools(servers, tools);

	let compiled = CompiledRegistry::compile(registry)?;

	// Test lookup by name (both base and virtual)
	assert!(compiled.get_tool("search").is_some());
	assert!(compiled.get_tool("get_weather").is_some());
	assert!(compiled.get_tool("web_search").is_some());
	assert!(compiled.get_tool("unknown").is_none());

	// Test is_virtualized checks (should resolve to backend tool)
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

	let servers = vec![ServerDef::stdio("weather-backend", "cmd", vec![])];
	let tools = vec![
		ToolDef::base("fetch_weather", "weather-backend"),
		ToolDef::virtual_tool("get_weather", "fetch_weather")
			.with_output_schema(OutputSchema::new(properties)),
	];
	let registry = Registry::with_servers_and_tools(servers, tools);

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
	let servers = vec![ServerDef::stdio("weather-backend", "cmd", vec![])];
	let tools = vec![
		ToolDef::base("fetch_weather", "weather-backend"),
		ToolDef::virtual_tool("get_weather", "fetch_weather")
			.with_default("api_key", serde_json::json!("secret-key"))
			.with_default("units", serde_json::json!("metric")),
	];
	let registry = Registry::with_servers_and_tools(servers, tools);

	let compiled = CompiledRegistry::compile(registry)?;

	// Prepare call args with only user-provided fields
	let user_args = serde_json::json!({
		"location": "San Francisco"
	});

	let (server, backend_tool, args) =
		compiled.prepare_call_args("get_weather", user_args.clone())?;

	// Check defaults were injected
	assert_eq!(server.as_str(), "weather-backend");
	assert_eq!(backend_tool.as_str(), "fetch_weather");

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
	let servers = vec![ServerDef::stdio("backend1", "cmd", vec![])];
	let tools = vec![ToolDef::base("tool1", "backend1")];
	let registry = Registry::with_servers_and_tools(servers, tools);

	store_ref.update(registry)?;
	assert!(store_ref.has_registry());

	// Verify tool is accessible
	{
		let guard = store_ref.get();
		let compiled = guard.as_ref().as_ref().unwrap();
		assert!(compiled.get_tool("tool1").is_some());
	}

	// Update with new registry
	let servers = vec![
		ServerDef::stdio("backend1", "cmd", vec![]),
		ServerDef::stdio("backend2", "cmd", vec![]),
	];
	let tools = vec![
		ToolDef::base("tool1", "backend1"),
		ToolDef::base("tool2", "backend2"),
	];
	let new_registry = Registry::with_servers_and_tools(servers, tools);

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
		"servers": [
			{"name": "weather", "stdio": {"command": "weather-cli", "args": []}},
			{"name": "search", "stdio": {"command": "search-cli", "args": []}}
		],
		"tools": [
			{
				"name": "fetch_weather",
				"server": "weather",
				"description": "Backend weather fetch tool",
				"inputSchema": {
					"type": "object",
					"properties": {
						"city": {"type": "string"}
					},
					"required": ["city"]
				},
				"version": "2.1.0"
			},
			{
				"name": "get_weather",
				"source": "fetch_weather",
				"description": "Get current weather for a city",
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
				"metadata": {
					"owner": "weather-team",
					"dataClassification": "public"
				}
			},
			{
				"name": "web_search",
				"server": "search",
				"description": "Backend search tool"
			},
			{
				"name": "search_web",
				"source": "web_search",
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
	assert_eq!(registry.len(), 4);
	assert_eq!(registry.servers.len(), 2);

	// Check base weather tool
	let fetch_weather = &registry.tools[0];
	assert_eq!(fetch_weather.name, "fetch_weather");
	assert_eq!(fetch_weather.server, Some("weather".to_string()));
	assert_eq!(fetch_weather.version, Some("2.1.0".to_string()));

	// Check virtual weather tool
	let weather_tool = &registry.tools[1];
	assert_eq!(weather_tool.name, "get_weather");
	assert_eq!(weather_tool.source, Some("fetch_weather".to_string()));
	assert_eq!(
		weather_tool.description,
		Some("Get current weather for a city".to_string())
	);
	assert_eq!(weather_tool.defaults.len(), 2);
	assert_eq!(weather_tool.hide_fields.len(), 2);
	assert!(weather_tool.output_schema.is_some());
	assert_eq!(weather_tool.metadata.len(), 2);

	// Check base search tool
	let web_search = &registry.tools[2];
	assert_eq!(web_search.name, "web_search");
	assert_eq!(web_search.server, Some("search".to_string()));

	// Check virtual search tool
	let search_tool = &registry.tools[3];
	assert_eq!(search_tool.name, "search_web");
	assert_eq!(search_tool.source, Some("web_search".to_string()));
	assert!(search_tool.defaults.is_empty());
	assert!(search_tool.hide_fields.is_empty());

	// Compile and verify lookups work
	let compiled = CompiledRegistry::compile(registry)?;
	assert!(compiled.get_tool("get_weather").is_some());
	assert!(compiled.get_tool("search_web").is_some());
	assert!(compiled.get_tool("fetch_weather").is_some());
	assert!(compiled.get_tool("web_search").is_some());
	assert!(compiled.is_virtualized("weather", "fetch_weather"));
	assert!(compiled.is_virtualized("search", "web_search"));

	Ok(())
}

/// Test that prepare_call_args returns error for unknown tools
#[tokio::test]
async fn test_prepare_call_args_unknown_tool() -> anyhow::Result<()> {
	let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
	let tools = vec![ToolDef::base("known_tool", "backend")];
	let registry = Registry::with_servers_and_tools(servers, tools);

	let compiled = CompiledRegistry::compile(registry)?;

	let result = compiled.prepare_call_args("unknown_tool", serde_json::json!({}));
	assert!(result.is_err());

	Ok(())
}

/// Test output transformation with passthrough (no output schema)
#[tokio::test]
async fn test_output_transformation_passthrough() -> anyhow::Result<()> {
	let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
	let tools = vec![
		ToolDef::base("original", "backend"),
		ToolDef::virtual_tool("simple_tool", "original"), // No output schema - should passthrough
	];
	let registry = Registry::with_servers_and_tools(servers, tools);

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

/// Test source chain resolution with multiple levels
#[tokio::test]
async fn test_source_chain_resolution() -> anyhow::Result<()> {
	let servers = vec![ServerDef::stdio("backend", "cmd", vec![])];
	let tools = vec![
		ToolDef::base("base_tool", "backend").with_default("a", serde_json::json!("from_base")),
		ToolDef::virtual_tool("level1", "base_tool")
			.with_default("b", serde_json::json!("from_level1")),
		ToolDef::virtual_tool("level2", "level1")
			.with_default("c", serde_json::json!("from_level2")),
	];
	let registry = Registry::with_servers_and_tools(servers, tools);

	let compiled = CompiledRegistry::compile(registry)?;

	// Test that level2 resolves to the correct backend
	let (server, backend_tool, args) =
		compiled.prepare_call_args("level2", serde_json::json!({}))?;

	assert_eq!(server, "backend");
	assert_eq!(backend_tool, "base_tool");

	// All defaults from the chain should be merged
	assert_eq!(args["a"], "from_base");
	assert_eq!(args["b"], "from_level1");
	assert_eq!(args["c"], "from_level2");

	Ok(())
}
