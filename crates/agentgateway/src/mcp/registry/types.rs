// Registry types for virtual tool definitions
//
// Unified registry format with:
// - Named server definitions (stdio commands, remote URLs)
// - Base tools (reference servers by name, define backend tool schema)
// - Virtual tools (reference other tools via source, apply transformations)

use std::collections::HashMap;

use agent_core::prelude::Strng;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::types::agent::{
    McpTarget, McpTargetSpec, SimpleBackendReference, SseTargetSpec, StreamableHTTPTargetSpec,
    Target,
};

/// Parsed registry from JSON
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
	/// Schema version for compatibility
	#[serde(default = "default_schema_version")]
	pub schema_version: String,

	/// Named server definitions
	#[serde(default)]
	pub servers: Vec<ServerDef>,

	/// Shared schemas for $ref resolution
	#[serde(default)]
	pub schemas: HashMap<String, serde_json::Value>,

	/// List of tool definitions (base and virtual)
	#[serde(default)]
	pub tools: Vec<ToolDef>,
}

fn default_schema_version() -> String {
	"1.0".to_string()
}

/// Server definition - how to connect to an MCP backend
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDef {
	/// Server name (referenced by tools)
	pub name: String,

	/// Human-readable description
	#[serde(default)]
	pub description: Option<String>,

	/// Stdio process configuration (for local servers)
	#[serde(default)]
	pub stdio: Option<StdioConfig>,

	/// Remote URL (for HTTP-based servers)
	#[serde(default)]
	pub url: Option<String>,

	/// Transport type: "sse" or "streamablehttp"
	#[serde(default = "default_transport")]
	pub transport: String,

	/// Environment variables for stdio processes
	#[serde(default)]
	pub env: HashMap<String, String>,

	/// Auth type: "none" or "oauth"
	#[serde(default = "default_auth")]
	pub auth: String,
}

fn default_transport() -> String {
	"sse".to_string()
}

fn default_auth() -> String {
	"none".to_string()
}

/// Stdio process configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StdioConfig {
	/// Command to execute
	pub command: String,

	/// Command arguments
	#[serde(default)]
	pub args: Vec<String>,
}

/// Tool definition - either base (with server) or virtual (with source)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDef {
	/// Tool name exposed to agents
	pub name: String,

	/// Server reference (for base tools) - name of a server in the servers section
	#[serde(default)]
	pub server: Option<String>,

	/// Source tool reference (for virtual tools) - name of another tool in this registry
	#[serde(default)]
	pub source: Option<String>,

	/// Override description (optional, inherits from source if not set)
	#[serde(default)]
	pub description: Option<String>,

	/// Input schema (for base tools, or override for virtual tools)
	#[serde(default)]
	pub input_schema: Option<serde_json::Value>,

	/// Original name on the backend server (if different from name)
	#[serde(default)]
	pub original_name: Option<String>,

	/// Fields to inject at call time (supports ${ENV_VAR} substitution)
	#[serde(default)]
	pub defaults: HashMap<String, serde_json::Value>,

	/// Fields to remove from schema (hidden from agents)
	#[serde(default)]
	pub hide_fields: Vec<String>,

	/// Output transformation schema with JSONPath mappings
	#[serde(default)]
	pub output_schema: Option<OutputSchema>,

	/// Text extraction configuration for non-JSON responses
	#[serde(default)]
	pub text_extraction: Option<TextExtraction>,

	/// Semantic version of this tool definition
	#[serde(default)]
	pub version: Option<String>,

	/// Expected schema hash for drift detection (base tools only)
	#[serde(default)]
	pub expected_schema_hash: Option<String>,

	/// Validation mode: "strict", "warn", "skip"
	#[serde(default = "default_validation_mode")]
	pub validation_mode: String,

	/// Pin source tool version (virtual tools only)
	#[serde(default)]
	pub source_version_pin: Option<String>,

	/// Arbitrary metadata (owner, classification, etc.)
	#[serde(default)]
	pub metadata: HashMap<String, serde_json::Value>,
}

fn default_validation_mode() -> String {
	"warn".to_string()
}

/// Text extraction configuration for non-JSON responses
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextExtraction {
	/// Extraction mode: "json", "regex", "markdown_list"
	pub mode: String,

	/// Regex patterns for extraction (mode=regex)
	#[serde(default)]
	pub patterns: Vec<ExtractionPattern>,
}

/// Regex extraction pattern
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractionPattern {
	/// Field name to extract into
	pub field: String,

	/// Regex pattern with named groups
	pub pattern: String,
}

/// Output transformation schema
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSchema {
	/// Schema type (typically "object")
	#[serde(rename = "type", default = "default_object_type")]
	pub schema_type: String,

	/// Field definitions with JSONPath mappings
	#[serde(default)]
	pub properties: HashMap<String, OutputField>,
}

fn default_object_type() -> String {
	"object".to_string()
}

/// Output field definition
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputField {
	/// JSON Schema type (string, number, boolean, object, array)
	#[serde(rename = "type")]
	pub field_type: String,

	/// JSONPath expression to extract value from backend response
	#[serde(default)]
	pub source_field: Option<String>,

	/// Optional description for the field
	#[serde(default)]
	pub description: Option<String>,
}

impl Registry {
	/// Create an empty registry
	pub fn new() -> Self {
		Self::default()
	}

	/// Create a registry with servers and tools
	pub fn with_servers_and_tools(servers: Vec<ServerDef>, tools: Vec<ToolDef>) -> Self {
		Self {
			schema_version: default_schema_version(),
			servers,
			schemas: HashMap::new(),
			tools,
		}
	}

	/// Check if registry has any tools
	pub fn is_empty(&self) -> bool {
		self.tools.is_empty()
	}

	/// Get number of tools
	pub fn len(&self) -> usize {
		self.tools.len()
	}

	/// Get server by name
	pub fn get_server(&self, name: &str) -> Option<&ServerDef> {
		self.servers.iter().find(|s| s.name == name)
	}

	/// Get tool by name
	pub fn get_tool(&self, name: &str) -> Option<&ToolDef> {
		self.tools.iter().find(|t| t.name == name)
	}

	/// Check if a tool is a base tool (has server reference)
	pub fn is_base_tool(&self, name: &str) -> bool {
		self.get_tool(name).map(|t| t.server.is_some()).unwrap_or(false)
	}

	/// Check if a tool is a virtual tool (has source reference)
	pub fn is_virtual_tool(&self, name: &str) -> bool {
		self.get_tool(name).map(|t| t.source.is_some()).unwrap_or(false)
	}
}

impl ServerDef {
	/// Create a new stdio server definition
	pub fn stdio(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
		Self {
			name: name.into(),
			description: None,
			stdio: Some(StdioConfig {
				command: command.into(),
				args,
			}),
			url: None,
			transport: default_transport(),
			env: HashMap::new(),
			auth: default_auth(),
		}
	}

	/// Create a new remote server definition
	pub fn remote(name: impl Into<String>, url: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			description: None,
			stdio: None,
			url: Some(url.into()),
			transport: "streamablehttp".to_string(),
			env: HashMap::new(),
			auth: default_auth(),
		}
	}

	/// Builder method to set description
	pub fn with_description(mut self, desc: impl Into<String>) -> Self {
		self.description = Some(desc.into());
		self
	}

	/// Builder method to set auth type
	pub fn with_auth(mut self, auth: impl Into<String>) -> Self {
		self.auth = auth.into();
		self
	}

	/// Convert this server definition to an MCP target for the gateway.
	///
	/// Returns `None` if the server definition is invalid (neither stdio nor url).
	/// For OAuth-protected servers, returns the target but the gateway will need
	/// to handle authentication separately.
	pub fn to_mcp_target(&self) -> Option<McpTarget> {
		let spec = if let Some(ref stdio) = self.stdio {
			// Stdio server - self-contained with command and args
			McpTargetSpec::Stdio {
				cmd: stdio.command.clone(),
				args: stdio.args.clone(),
				env: self.env.clone(),
			}
		} else if let Some(ref url_str) = self.url {
			// Remote server - parse URL to extract host, port, and path
			let url = Url::parse(url_str).ok()?;
			let host = url.host_str()?;
			let port = url.port_or_known_default().unwrap_or(443);
			let path = url.path().to_string();

			// Create target from host:port
			let target: Target = (host, port).try_into().ok()?;
			let backend = SimpleBackendReference::InlineBackend(target);

			// Choose transport based on config
			match self.transport.as_str() {
				"sse" => McpTargetSpec::Sse(SseTargetSpec { backend, path }),
				_ => McpTargetSpec::Mcp(StreamableHTTPTargetSpec { backend, path }),
			}
		} else {
			// Invalid server - neither stdio nor url
			return None;
		};

		Some(McpTarget {
			name: Strng::from(self.name.as_str()),
			spec,
		})
	}

	/// Check if this is an OAuth-protected server
	pub fn requires_oauth(&self) -> bool {
		self.auth == "oauth"
	}
}

impl ToolDef {
	/// Create a base tool (references a server directly)
	pub fn base(name: impl Into<String>, server: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			server: Some(server.into()),
			source: None,
			description: None,
			input_schema: None,
			original_name: None,
			defaults: HashMap::new(),
			hide_fields: Vec::new(),
			output_schema: None,
			text_extraction: None,
			version: None,
			expected_schema_hash: None,
			validation_mode: default_validation_mode(),
			source_version_pin: None,
			metadata: HashMap::new(),
		}
	}

	/// Create a virtual tool (references another tool)
	pub fn virtual_tool(name: impl Into<String>, source: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			server: None,
			source: Some(source.into()),
			description: None,
			input_schema: None,
			original_name: None,
			defaults: HashMap::new(),
			hide_fields: Vec::new(),
			output_schema: None,
			text_extraction: None,
			version: None,
			expected_schema_hash: None,
			validation_mode: default_validation_mode(),
			source_version_pin: None,
			metadata: HashMap::new(),
		}
	}

	/// Builder method to set description
	pub fn with_description(mut self, desc: impl Into<String>) -> Self {
		self.description = Some(desc.into());
		self
	}

	/// Builder method to add a default value
	pub fn with_default(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
		self.defaults.insert(key.into(), value);
		self
	}

	/// Builder method to hide fields
	pub fn with_hidden_fields(mut self, fields: Vec<String>) -> Self {
		self.hide_fields = fields;
		self
	}

	/// Builder method to set output schema
	pub fn with_output_schema(mut self, schema: OutputSchema) -> Self {
		self.output_schema = Some(schema);
		self
	}

	/// Builder method to set version
	pub fn with_version(mut self, version: impl Into<String>) -> Self {
		self.version = Some(version.into());
		self
	}

	/// Builder method to set original name
	pub fn with_original_name(mut self, original_name: impl Into<String>) -> Self {
		self.original_name = Some(original_name.into());
		self
	}
}

impl OutputSchema {
	/// Create an output schema with the given properties
	pub fn new(properties: HashMap<String, OutputField>) -> Self {
		Self {
			schema_type: default_object_type(),
			properties,
		}
	}
}

impl OutputField {
	/// Create a new output field with JSONPath source
	pub fn new(field_type: impl Into<String>, source_field: impl Into<String>) -> Self {
		Self {
			field_type: field_type.into(),
			source_field: Some(source_field.into()),
			description: None,
		}
	}

	/// Create a field without JSONPath (passthrough)
	pub fn passthrough(field_type: impl Into<String>) -> Self {
		Self {
			field_type: field_type.into(),
			source_field: None,
			description: None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_minimal_registry() {
		let json = r#"{
            "tools": []
        }"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.schema_version, "1.0");
		assert!(registry.tools.is_empty());
		assert!(registry.servers.is_empty());
	}

	#[test]
	fn test_parse_registry_with_version() {
		let json = r#"{
            "schemaVersion": "2.0",
            "tools": []
        }"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.schema_version, "2.0");
	}

	#[test]
	fn test_parse_server_stdio() {
		let json = r#"{
            "name": "fetch-server",
            "description": "Web fetch server",
            "stdio": {
                "command": "uvx",
                "args": ["mcp-server-fetch"]
            }
        }"#;

		let server: ServerDef = serde_json::from_str(json).unwrap();
		assert_eq!(server.name, "fetch-server");
		assert_eq!(server.description, Some("Web fetch server".to_string()));
		assert!(server.stdio.is_some());
		let stdio = server.stdio.unwrap();
		assert_eq!(stdio.command, "uvx");
		assert_eq!(stdio.args, vec!["mcp-server-fetch"]);
		assert!(server.url.is_none());
		assert_eq!(server.transport, "sse");
		assert_eq!(server.auth, "none");
	}

	#[test]
	fn test_parse_server_remote() {
		let json = r#"{
            "name": "cloudflare-radar",
            "url": "https://radar.mcp.cloudflare.com/mcp",
            "transport": "streamablehttp",
            "auth": "oauth"
        }"#;

		let server: ServerDef = serde_json::from_str(json).unwrap();
		assert_eq!(server.name, "cloudflare-radar");
		assert_eq!(server.url, Some("https://radar.mcp.cloudflare.com/mcp".to_string()));
		assert_eq!(server.transport, "streamablehttp");
		assert_eq!(server.auth, "oauth");
		assert!(server.stdio.is_none());
	}

	#[test]
	fn test_parse_base_tool() {
		let json = r#"{
            "name": "fetch",
            "server": "fetch-server",
            "description": "Fetch a URL",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": {"type": "string"}
                },
                "required": ["url"]
            },
            "version": "2.1.0"
        }"#;

		let tool: ToolDef = serde_json::from_str(json).unwrap();
		assert_eq!(tool.name, "fetch");
		assert_eq!(tool.server, Some("fetch-server".to_string()));
		assert!(tool.source.is_none());
		assert_eq!(tool.description, Some("Fetch a URL".to_string()));
		assert_eq!(tool.version, Some("2.1.0".to_string()));
		assert!(tool.input_schema.is_some());
	}

	#[test]
	fn test_parse_virtual_tool() {
		let json = r#"{
            "name": "get_webpage",
            "source": "fetch",
            "description": "Get webpage content"
        }"#;

		let tool: ToolDef = serde_json::from_str(json).unwrap();
		assert_eq!(tool.name, "get_webpage");
		assert!(tool.server.is_none());
		assert_eq!(tool.source, Some("fetch".to_string()));
		assert_eq!(tool.description, Some("Get webpage content".to_string()));
	}

	#[test]
	fn test_parse_virtual_tool_with_output_schema() {
		let json = r#"{
            "name": "list_entity_names",
            "source": "read_graph",
            "description": "Extract entity names",
            "outputSchema": {
                "type": "object",
                "properties": {
                    "names": {
                        "type": "array",
                        "sourceField": "$.entities[*].name"
                    }
                }
            }
        }"#;

		let tool: ToolDef = serde_json::from_str(json).unwrap();
		assert_eq!(tool.name, "list_entity_names");
		assert_eq!(tool.source, Some("read_graph".to_string()));

		let output = tool.output_schema.unwrap();
		assert_eq!(output.schema_type, "object");
		let names_field = output.properties.get("names").unwrap();
		assert_eq!(names_field.field_type, "array");
		assert_eq!(names_field.source_field, Some("$.entities[*].name".to_string()));
	}

	#[test]
	fn test_parse_full_registry() {
		let json = r#"{
            "schemaVersion": "1.0",
            "servers": [
                {
                    "name": "fetch-server",
                    "stdio": {"command": "uvx", "args": ["mcp-server-fetch"]}
                },
                {
                    "name": "cloudflare-radar",
                    "url": "https://radar.mcp.cloudflare.com/mcp",
                    "transport": "streamablehttp",
                    "auth": "oauth"
                }
            ],
            "tools": [
                {
                    "name": "fetch",
                    "server": "fetch-server",
                    "version": "2.1.0"
                },
                {
                    "name": "get_webpage",
                    "source": "fetch",
                    "description": "Get webpage content"
                }
            ]
        }"#;

		let registry: Registry = serde_json::from_str(json).unwrap();
		assert_eq!(registry.servers.len(), 2);
		assert_eq!(registry.tools.len(), 2);

		// Check servers
		assert_eq!(registry.servers[0].name, "fetch-server");
		assert!(registry.servers[0].stdio.is_some());
		assert_eq!(registry.servers[1].name, "cloudflare-radar");
		assert_eq!(registry.servers[1].auth, "oauth");

		// Check tools
		assert_eq!(registry.tools[0].name, "fetch");
		assert_eq!(registry.tools[0].server, Some("fetch-server".to_string()));
		assert_eq!(registry.tools[1].name, "get_webpage");
		assert_eq!(registry.tools[1].source, Some("fetch".to_string()));
	}

	#[test]
	fn test_builder_pattern_base_tool() {
		let tool = ToolDef::base("fetch", "fetch-server")
			.with_description("Fetch a URL")
			.with_version("2.1.0");

		assert_eq!(tool.name, "fetch");
		assert_eq!(tool.server, Some("fetch-server".to_string()));
		assert!(tool.source.is_none());
		assert_eq!(tool.description, Some("Fetch a URL".to_string()));
		assert_eq!(tool.version, Some("2.1.0".to_string()));
	}

	#[test]
	fn test_builder_pattern_virtual_tool() {
		let tool = ToolDef::virtual_tool("get_webpage", "fetch")
			.with_description("Get webpage content")
			.with_default("timeout", serde_json::json!(30))
			.with_hidden_fields(vec!["debug".to_string()]);

		assert_eq!(tool.name, "get_webpage");
		assert!(tool.server.is_none());
		assert_eq!(tool.source, Some("fetch".to_string()));
		assert_eq!(tool.description, Some("Get webpage content".to_string()));
		assert_eq!(tool.defaults.get("timeout"), Some(&serde_json::json!(30)));
		assert_eq!(tool.hide_fields, vec!["debug"]);
	}

	#[test]
	fn test_builder_pattern_server() {
		let server = ServerDef::stdio("fetch-server", "uvx", vec!["mcp-server-fetch".to_string()])
			.with_description("Fetch server");

		assert_eq!(server.name, "fetch-server");
		assert_eq!(server.description, Some("Fetch server".to_string()));
		assert!(server.stdio.is_some());
		assert_eq!(server.stdio.as_ref().unwrap().command, "uvx");

		let remote = ServerDef::remote("api", "https://api.example.com/mcp")
			.with_auth("oauth");

		assert_eq!(remote.name, "api");
		assert_eq!(remote.url, Some("https://api.example.com/mcp".to_string()));
		assert_eq!(remote.auth, "oauth");
	}

	#[test]
	fn test_registry_methods() {
		let empty = Registry::new();
		assert!(empty.is_empty());
		assert_eq!(empty.len(), 0);

		let servers = vec![ServerDef::stdio("s1", "cmd", vec![])];
		let tools = vec![
			ToolDef::base("tool1", "s1"),
			ToolDef::virtual_tool("tool2", "tool1"),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);

		assert!(!registry.is_empty());
		assert_eq!(registry.len(), 2);
		assert!(registry.get_server("s1").is_some());
		assert!(registry.get_server("nonexistent").is_none());
		assert!(registry.get_tool("tool1").is_some());
		assert!(registry.is_base_tool("tool1"));
		assert!(!registry.is_base_tool("tool2"));
		assert!(registry.is_virtual_tool("tool2"));
		assert!(!registry.is_virtual_tool("tool1"));
	}

	#[test]
	fn test_serialize_registry() {
		let servers = vec![ServerDef::stdio("fetch-server", "uvx", vec!["mcp-server-fetch".to_string()])];
		let tools = vec![
			ToolDef::base("fetch", "fetch-server").with_version("2.1.0"),
			ToolDef::virtual_tool("get_webpage", "fetch").with_description("Get webpage"),
		];
		let registry = Registry::with_servers_and_tools(servers, tools);

		let json = serde_json::to_string_pretty(&registry).unwrap();
		assert!(json.contains("fetch-server"));
		assert!(json.contains("get_webpage"));
		assert!(json.contains("2.1.0"));

		// Round-trip test
		let parsed: Registry = serde_json::from_str(&json).unwrap();
		assert_eq!(parsed.servers.len(), 1);
		assert_eq!(parsed.tools.len(), 2);
		assert_eq!(parsed.tools[0].name, "fetch");
		assert_eq!(parsed.tools[1].name, "get_webpage");
	}

	#[test]
	fn test_parse_tool_with_versioning() {
		let json = r#"{
            "name": "fetch",
            "server": "fetch-server",
            "version": "2.1.0",
            "expectedSchemaHash": "abc123",
            "validationMode": "strict"
        }"#;

		let tool: ToolDef = serde_json::from_str(json).unwrap();
		assert_eq!(tool.version, Some("2.1.0".to_string()));
		assert_eq!(tool.expected_schema_hash, Some("abc123".to_string()));
		assert_eq!(tool.validation_mode, "strict");
	}

	#[test]
	fn test_parse_virtual_tool_with_source_pin() {
		let json = r#"{
            "name": "get_webpage",
            "source": "fetch",
            "sourceVersionPin": "2.1.0"
        }"#;

		let tool: ToolDef = serde_json::from_str(json).unwrap();
		assert_eq!(tool.source, Some("fetch".to_string()));
		assert_eq!(tool.source_version_pin, Some("2.1.0".to_string()));
	}

	#[test]
	fn test_server_to_mcp_target_stdio() {
		let server = ServerDef::stdio("fetch-server", "uvx", vec!["mcp-server-fetch".to_string()]);

		let target = server.to_mcp_target().expect("should convert to MCP target");
		assert_eq!(target.name.as_str(), "fetch-server");

		match target.spec {
			McpTargetSpec::Stdio { cmd, args, env } => {
				assert_eq!(cmd, "uvx");
				assert_eq!(args, vec!["mcp-server-fetch"]);
				assert!(env.is_empty());
			},
			_ => panic!("expected Stdio target spec"),
		}
	}

	#[test]
	fn test_server_to_mcp_target_remote_sse() {
		let mut server = ServerDef::remote("api", "https://api.example.com:8080/mcp");
		server.transport = "sse".to_string();

		let target = server.to_mcp_target().expect("should convert to MCP target");
		assert_eq!(target.name.as_str(), "api");

		match target.spec {
			McpTargetSpec::Sse(spec) => {
				assert_eq!(spec.path, "/mcp");
				// Backend should be InlineBackend with the host:port
				match spec.backend {
					SimpleBackendReference::InlineBackend(t) => {
						assert_eq!(t.hostport(), "api.example.com:8080");
					},
					_ => panic!("expected InlineBackend"),
				}
			},
			_ => panic!("expected Sse target spec"),
		}
	}

	#[test]
	fn test_server_to_mcp_target_remote_streamablehttp() {
		let server = ServerDef::remote("cloudflare", "https://docs.mcp.cloudflare.com/mcp");

		let target = server.to_mcp_target().expect("should convert to MCP target");
		assert_eq!(target.name.as_str(), "cloudflare");

		match target.spec {
			McpTargetSpec::Mcp(spec) => {
				assert_eq!(spec.path, "/mcp");
				match spec.backend {
					SimpleBackendReference::InlineBackend(t) => {
						// HTTPS default port is 443
						assert_eq!(t.hostport(), "docs.mcp.cloudflare.com:443");
					},
					_ => panic!("expected InlineBackend"),
				}
			},
			_ => panic!("expected Mcp (streamablehttp) target spec"),
		}
	}

	#[test]
	fn test_server_requires_oauth() {
		let server = ServerDef::remote("radar", "https://radar.mcp.cloudflare.com/mcp")
			.with_auth("oauth");

		assert!(server.requires_oauth());

		let non_oauth = ServerDef::remote("docs", "https://docs.mcp.cloudflare.com/mcp");
		assert!(!non_oauth.requires_oauth());
	}

	#[test]
	fn test_server_to_mcp_target_invalid() {
		// Server with neither stdio nor url should return None
		let server = ServerDef {
			name: "invalid".to_string(),
			description: None,
			stdio: None,
			url: None,
			transport: "sse".to_string(),
			env: HashMap::new(),
			auth: "none".to_string(),
		};

		assert!(server.to_mcp_target().is_none());
	}
}
