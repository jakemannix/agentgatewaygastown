# Agentgateway Registry Integration Design

## Overview

Integrate tool registry functionality from [mcp-gateway-prototype](https://github.com/jakemannix/mcp-gateway-prototype) into agentgateway, enabling:

- **Tool Adaptation**: Rename tools, hide fields, inject defaults, transform outputs
- **Registry as External Service**: Load registry from `file://` or `http://` URIs
- **Hot Reload**: Periodic refresh without gateway restart

---

# Part 1: Target State Design

## 1.1 Configuration Schema

### Gateway Config (config.yaml)

```yaml
# Global registry (applies to all MCP backends)
registry:
  source: http://registry.corp.com/api/v1/tools
  # OR for local development:
  # source: file:///path/to/registry.json
  refreshInterval: 5m
  auth:
    bearer: ${REGISTRY_TOKEN}  # Optional

# Existing backend config unchanged
backends:
  - mcp:
      targets:
        - name: weather
          stdio:
            cmd: uvx
            args: ["mcp-server-weather"]
        - name: fetch
          stdio:
            cmd: npx
            args: ["mcp-server-fetch"]
```

### Registry JSON Schema

```json
{
  "$schema": "https://agentgateway.dev/schemas/registry/v1.json",
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
          "city": {
            "type": "string",
            "description": "City name"
          }
        },
        "required": ["city"]
      },
      "defaults": {
        "api_key": "${WEATHER_API_KEY}",
        "units": "metric"
      },
      "hideFields": ["debug_mode", "raw_output", "station_code"],
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
    }
  ]
}
```

## 1.2 Data Structures

### Core Types

```rust
// crates/agentgateway/src/mcp/registry/types.rs

/// Parsed registry from JSON
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    pub schema_version: String,
    pub tools: Vec<VirtualToolDef>,
}

/// Virtual tool definition from registry
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualToolDef {
    /// Name exposed to agents
    pub name: String,

    /// Source backend tool
    pub source: ToolSource,

    /// Override description (optional)
    pub description: Option<String>,

    /// Override input schema (optional, inherits from source if not set)
    pub input_schema: Option<serde_json::Value>,

    /// Fields to inject at call time
    #[serde(default)]
    pub defaults: HashMap<String, String>,

    /// Fields to remove from schema
    #[serde(default)]
    pub hide_fields: Vec<String>,

    /// Output transformation
    pub output_schema: Option<OutputSchema>,

    /// Semantic version
    pub version: Option<String>,

    /// Arbitrary metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolSource {
    /// Target name (MCP server)
    pub target: String,
    /// Original tool name on that target
    pub tool: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: HashMap<String, OutputField>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputField {
    #[serde(rename = "type")]
    pub field_type: String,
    /// JSONPath expression to extract value
    pub source_field: Option<String>,
}
```

### Runtime Types

```rust
// crates/agentgateway/src/mcp/registry/mod.rs

/// Compiled registry ready for runtime use
#[derive(Debug)]
pub struct CompiledRegistry {
    /// Virtual tool name -> compiled tool
    tools_by_name: HashMap<String, CompiledVirtualTool>,
    /// (target, source_tool) -> virtual tool names (for reverse lookup)
    tools_by_source: HashMap<(String, String), Vec<String>>,
}

#[derive(Debug)]
pub struct CompiledVirtualTool {
    pub def: VirtualToolDef,
    /// Pre-compiled JSONPath expressions for output projection
    pub output_paths: Option<HashMap<String, JsonPath>>,
    /// Merged schema (source schema with hideFields applied)
    pub effective_schema: Option<serde_json::Value>,
}

impl CompiledRegistry {
    /// Look up virtual tool by exposed name
    pub fn get_tool(&self, name: &str) -> Option<&CompiledVirtualTool> { ... }

    /// Check if a backend tool is virtualized
    pub fn is_virtualized(&self, target: &str, tool: &str) -> bool { ... }

    /// Transform backend tool list to virtual tool list
    pub fn transform_tools(&self, backend_tools: Vec<Tool>) -> Vec<Tool> { ... }

    /// Prepare arguments for backend call (inject defaults)
    pub fn prepare_call_args(
        &self,
        virtual_name: &str,
        args: serde_json::Value,
    ) -> Result<(String, String, serde_json::Value), RegistryError> { ... }

    /// Transform backend response to virtual response
    pub fn transform_output(
        &self,
        virtual_name: &str,
        response: serde_json::Value,
    ) -> Result<serde_json::Value, RegistryError> { ... }
}
```

## 1.3 RegistryClient and RegistryStore

```rust
// crates/agentgateway/src/mcp/registry/client.rs

pub struct RegistryClient {
    source: RegistrySource,
    client: Client,
    refresh_interval: Duration,
}

#[derive(Debug, Clone)]
pub enum RegistrySource {
    File(PathBuf),
    Http { url: http::Uri, auth: Option<AuthConfig> },
}

impl RegistryClient {
    pub async fn fetch(&self) -> Result<Registry, RegistryError> {
        match &self.source {
            RegistrySource::File(path) => {
                let content = fs_err::tokio::read_to_string(path).await?;
                Ok(serde_json::from_str(&content)?)
            }
            RegistrySource::Http { url, auth } => {
                let mut req = http::Request::builder().uri(url);
                if let Some(auth) = auth {
                    req = req.header("Authorization", auth.to_header_value());
                }
                let resp = self.client.simple_call(req.body(Body::empty())?).await?;
                crate::json::from_response_body(resp).await
            }
        }
    }

    pub fn refresh_interval(&self) -> Duration {
        self.refresh_interval
    }
}

// crates/agentgateway/src/store/registry.rs

pub struct RegistryStore {
    /// Current compiled registry (atomically swappable)
    current: Arc<ArcSwap<Option<CompiledRegistry>>>,
    /// Client for fetching updates
    client: Option<RegistryClient>,
}

impl RegistryStore {
    pub fn new() -> Self {
        Self {
            current: Arc::new(ArcSwap::new(Arc::new(None))),
            client: None,
        }
    }

    pub fn with_client(mut self, client: RegistryClient) -> Self {
        self.client = Some(client);
        self
    }

    /// Get current registry (returns None if no registry configured)
    pub fn get(&self) -> Option<Arc<CompiledRegistry>> {
        self.current.load().as_ref().clone()
    }

    /// Update registry (called by refresh loop or file watcher)
    pub fn update(&self, registry: Registry) -> Result<(), RegistryError> {
        let compiled = CompiledRegistry::compile(registry)?;
        self.current.store(Arc::new(Some(compiled)));
        Ok(())
    }

    /// Start background refresh (for HTTP sources)
    pub fn spawn_refresh_loop(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let Some(client) = &self.client else { return };
            let interval = client.refresh_interval();

            loop {
                tokio::time::sleep(interval).await;
                match client.fetch().await {
                    Ok(registry) => {
                        if let Err(e) = self.update(registry) {
                            warn!("Failed to compile registry: {}", e);
                        } else {
                            info!("Registry refreshed successfully");
                        }
                    }
                    Err(e) => {
                        warn!("Failed to fetch registry: {}", e);
                    }
                }
            }
        })
    }

    /// Start file watcher (for file:// sources)
    pub fn spawn_file_watcher(self: Arc<Self>, path: PathBuf) -> Result<(), anyhow::Error> {
        // Similar to LocalClient::watch_config_file pattern
        // On change: fetch + update
    }
}
```

## 1.4 Integration Points

### Stores Integration

```rust
// crates/agentgateway/src/store/mod.rs

pub struct Stores {
    pub binds: BindStore,
    pub discovery: DiscoveryStore,
    pub registry: Arc<RegistryStore>,  // NEW
}

impl Stores {
    pub fn new() -> Self {
        Self {
            binds: BindStore::new(),
            discovery: DiscoveryStore::new(),
            registry: Arc::new(RegistryStore::new()),
        }
    }
}
```

### StateManager Integration

```rust
// crates/agentgateway/src/state_manager.rs

impl StateManager {
    pub async fn new(...) -> anyhow::Result<Self> {
        let stores = Stores::new();

        // NEW: Initialize registry if configured
        if let Some(registry_cfg) = &config.registry {
            let client = RegistryClient::new(
                registry_cfg.source.clone(),
                client.clone(),
                registry_cfg.refresh_interval,
            );

            // Initial load
            let registry = client.fetch().await?;
            stores.registry.update(registry)?;

            // Start refresh loop or file watcher
            match &registry_cfg.source {
                RegistrySource::File(path) => {
                    stores.registry.clone().spawn_file_watcher(path.clone())?;
                }
                RegistrySource::Http { .. } => {
                    stores.registry.clone().spawn_refresh_loop();
                }
            }
        }

        // ... rest of initialization
    }
}
```

### Relay Integration

```rust
// crates/agentgateway/src/mcp/handler.rs

pub struct Relay {
    upstreams: Arc<UpstreamGroup>,
    policies: McpAuthorizationSet,
    registry: Option<Arc<RegistryStore>>,  // NEW
    default_target_name: Option<String>,
    is_multiplexing: bool,
}

impl Relay {
    pub fn new(
        backend: McpBackendGroup,
        policies: McpAuthorizationSet,
        client: PolicyClient,
        registry: Option<Arc<RegistryStore>>,  // NEW parameter
    ) -> anyhow::Result<Self> {
        // ... existing logic
        Ok(Self {
            upstreams: Arc::new(UpstreamGroup::new(client, backend)?),
            policies,
            registry,
            default_target_name,
            is_multiplexing,
        })
    }

    pub fn merge_tools(&self, cel: Arc<ContextBuilder>) -> Box<MergeFn> {
        let policies = self.policies.clone();
        let default_target_name = self.default_target_name.clone();
        let registry = self.registry.as_ref().and_then(|r| r.get());

        Box::new(move |streams| {
            let mut tools = streams
                .into_iter()
                .flat_map(|(server_name, s)| {
                    // ... existing collection logic
                })
                .collect_vec();

            // NEW: Apply registry transformations
            if let Some(reg) = &registry {
                tools = reg.transform_tools(tools);
            }

            // Existing RBAC filtering
            tools = tools
                .into_iter()
                .filter(|t| policies.validate(...))
                .collect_vec();

            Ok(ListToolsResult { tools, next_cursor: None, meta: None }.into())
        })
    }
}
```

### Session Integration (CallTool)

```rust
// crates/agentgateway/src/mcp/session.rs

impl Session {
    async fn handle_call_tool(
        &self,
        request: CallToolRequest,
    ) -> Result<ServerResult, UpstreamError> {
        let tool_name = &request.name;
        let mut args = request.arguments.clone();
        let mut target_tool = tool_name.clone();
        let mut target_name: Option<&str> = None;

        // NEW: Check if this is a virtual tool
        if let Some(registry) = self.relay.registry.as_ref().and_then(|r| r.get()) {
            if let Some(virtual_tool) = registry.get_tool(tool_name) {
                // Resolve to backend tool
                target_name = Some(&virtual_tool.def.source.target);
                target_tool = virtual_tool.def.source.tool.clone();

                // Inject defaults
                args = registry.prepare_call_args(tool_name, args)?;
            }
        }

        // Route to backend (existing logic)
        let (target, resource) = if let Some(t) = target_name {
            (t, &target_tool)
        } else {
            self.relay.parse_resource_name(tool_name)?
        };

        let response = self.send_to_upstream(target, resource, args).await?;

        // NEW: Transform output if virtual tool
        if let Some(registry) = self.relay.registry.as_ref().and_then(|r| r.get()) {
            if let Some(_) = registry.get_tool(tool_name) {
                return Ok(registry.transform_output(tool_name, response)?);
            }
        }

        Ok(response)
    }
}
```

## 1.5 Request Flow Summary

### ListTools Flow

```
1. Client sends ListToolsRequest
2. Session.send_fanout() -> queries all upstream MCP servers
3. MergeStream collects responses
4. Relay.merge_tools() is called with backend tool lists:
   a. Collect all backend tools
   b. If registry exists:
      - For each virtual tool in registry:
        - Find source tool in backend list
        - Create transformed tool (new name, modified schema, hidden fields)
        - Add to output, remove source from "pass-through" list
      - Pass through non-virtualized tools unchanged
   c. Apply RBAC filtering
   d. Apply multiplexing prefix if needed
5. Return merged ListToolsResult to client
```

### CallTool Flow

```
1. Client sends CallToolRequest(name="get_weather", args={city: "Seattle"})
2. Session.handle_call_tool():
   a. Look up "get_weather" in registry
   b. Found: virtual tool mapping to weather/fetch_weather
   c. Inject defaults: args += {api_key: "<from env>", units: "metric"}
   d. Route to "weather" target, call "fetch_weather"
3. Backend returns complex response:
   {data: {current: {temp_f: 52.3, condition: {text: "Cloudy"}, ...}}}
4. Apply output projection (JSONPath):
   - $.data.current.temp_f -> temperature: 52.3
   - $.data.current.condition.text -> conditions: "Cloudy"
5. Return transformed response:
   {temperature: 52.3, conditions: "Cloudy"}
```

---

# Part 2: Implementation Plan

## Phase 0: Test Infrastructure Setup

### 0.1 Port Unit Tests from mcp-gateway-prototype

Source: `~/src/open_src/mcp-gateway-prototype/tests/`

| Source Test File | Lines | Target Location | Notes |
|------------------|-------|-----------------|-------|
| `test_config_loader.py` | 634 | `registry/tests/config_tests.rs` | Schema resolution, inheritance |
| `test_output_transformer.py` | 245 | `registry/tests/output_tests.rs` | JSONPath extraction |
| `test_json_detector.py` | 402 | `registry/tests/json_detector_tests.rs` | JSON-in-text extraction |
| `test_output_schema_jsonpath.py` | 568 | `registry/tests/jsonpath_tests.rs` | Complex JSONPath scenarios |
| `test_tool_versioning.py` | 557 | `registry/tests/versioning_tests.rs` | Schema hashing, drift |
| `test_mcp_server.py` | 688 | Integration tests | End-to-end routing |
| `test_defaults.py` | ~200 | `registry/tests/defaults_tests.rs` | Default injection |

**Approach**:
1. Create `crates/agentgateway/src/mcp/registry/tests/` module
2. Translate Python test cases to Rust, preserving test names and scenarios
3. Use `insta` for snapshot testing where appropriate
4. Use existing test patterns from agentgateway

### 0.2 Integration Test Framework

Create new integration test infrastructure:

```
crates/agentgateway/tests/
  integration/
    mod.rs
    helpers/
      mod.rs
      gateway.rs      # Start/stop gateway process
      mcp_client.rs   # MCP client for testing
      mock_server.rs  # Mock MCP server
    registry_tests.rs   # Registry integration tests
    e2e_tests.rs        # Full end-to-end tests
```

**Gateway Test Harness**:
```rust
// tests/integration/helpers/gateway.rs

pub struct TestGateway {
    process: Child,
    pub port: u16,
    pub config_path: PathBuf,
    _temp_dir: TempDir,
}

impl TestGateway {
    pub async fn start(config: &str) -> Result<Self, anyhow::Error> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.yaml");
        fs::write(&config_path, config)?;

        let port = get_available_port();
        let process = Command::new(env!("CARGO_BIN_EXE_agentgateway"))
            .arg("-f")
            .arg(&config_path)
            .env("ADMIN_PORT", port.to_string())
            .spawn()?;

        // Wait for ready
        wait_for_port(port).await?;

        Ok(Self { process, port, config_path, _temp_dir: temp_dir })
    }

    pub async fn update_config(&self, config: &str) -> Result<(), anyhow::Error> {
        fs::write(&self.config_path, config)?;
        // Gateway should hot-reload
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }
}

impl Drop for TestGateway {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}
```

**MCP Test Client**:
```rust
// tests/integration/helpers/mcp_client.rs

pub struct TestMcpClient {
    client: reqwest::Client,
    base_url: String,
}

impl TestMcpClient {
    pub fn new(port: u16) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: format!("http://localhost:{}/mcp", port),
        }
    }

    pub async fn list_tools(&self) -> Result<ListToolsResult, anyhow::Error> { ... }
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, anyhow::Error> { ... }
    pub async fn initialize(&self) -> Result<InitializeResult, anyhow::Error> { ... }
}
```

**Mock MCP Server**:
```rust
// tests/integration/helpers/mock_server.rs

pub struct MockMcpServer {
    tools: Vec<Tool>,
    responses: HashMap<String, Value>,
    server: Option<JoinHandle<()>>,
    pub port: u16,
}

impl MockMcpServer {
    pub fn new() -> Self { ... }

    pub fn with_tool(mut self, tool: Tool, response: Value) -> Self {
        self.tools.push(tool);
        self.responses.insert(tool.name.to_string(), response);
        self
    }

    pub async fn start(mut self) -> Result<Self, anyhow::Error> {
        // Start HTTP server implementing MCP StreamableHTTP
        ...
    }
}
```

## Phase 1: Core Data Structures

**Files to create**:
- `crates/agentgateway/src/mcp/registry/mod.rs`
- `crates/agentgateway/src/mcp/registry/types.rs`
- `crates/agentgateway/src/mcp/registry/compiled.rs`
- `crates/agentgateway/src/mcp/registry/error.rs`

**Deliverables**:
1. `Registry`, `VirtualToolDef`, `OutputSchema` types (deserializable)
2. `CompiledRegistry`, `CompiledVirtualTool` types
3. `RegistryError` error type
4. Unit tests for parsing various registry JSON formats

**Test coverage**:
- Valid registry parsing
- Missing required fields
- Schema inheritance scenarios
- Malformed JSON handling

## Phase 2: JSONPath Output Projection

**Dependencies**: Add `serde_json_path` or `jsonpath-rust` to Cargo.toml

**Files to create/modify**:
- `crates/agentgateway/src/mcp/registry/jsonpath.rs`
- `crates/agentgateway/src/mcp/registry/transform.rs`

**Deliverables**:
1. JSONPath compilation and caching
2. `transform_output()` implementation
3. JSON-in-text detection (port from mcp-gateway-prototype)

**Test coverage** (ported from mcp-gateway-prototype):
- Simple path extraction: `$.field`
- Nested paths: `$.data.nested.value`
- Array access: `$.items[0]`, `$.items[*].name`
- Wildcard: `$..name`
- JSON embedded in text responses

## Phase 3: Default Injection and Schema Transformation

**Files to create/modify**:
- `crates/agentgateway/src/mcp/registry/defaults.rs`
- `crates/agentgateway/src/mcp/registry/schema.rs`

**Deliverables**:
1. Environment variable substitution (`${VAR_NAME}`)
2. `prepare_call_args()` - merge user args with defaults
3. `transform_schema()` - apply hideFields, compute effective schema
4. Schema merging (virtual overrides source)

**Test coverage**:
- Env var substitution (present, missing, default)
- Default injection (override vs merge)
- Field hiding in schema
- Required field validation after hiding

## Phase 4: RegistryClient

**Files to create**:
- `crates/agentgateway/src/mcp/registry/client.rs`

**Deliverables**:
1. `RegistrySource` enum (File, Http)
2. `RegistryClient::fetch()` for both sources
3. Authentication support for HTTP
4. Error handling and retry logic

**Test coverage**:
- File loading (valid, missing, invalid JSON)
- HTTP loading (mock server)
- Auth header injection
- Refresh on file change (file watcher)

## Phase 5: RegistryStore and Stores Integration

**Files to create/modify**:
- `crates/agentgateway/src/store/registry.rs`
- `crates/agentgateway/src/store/mod.rs`
- `crates/agentgateway/src/state_manager.rs`

**Deliverables**:
1. `RegistryStore` with `ArcSwap` for hot-swap
2. Integration into `Stores`
3. Initialization in `StateManager`
4. Background refresh loop
5. File watcher for `file://` sources

**Test coverage**:
- Atomic registry swap
- Concurrent read during update
- Refresh loop timing
- File watcher triggers update

## Phase 6: Relay Integration

**Files to modify**:
- `crates/agentgateway/src/mcp/handler.rs`
- `crates/agentgateway/src/mcp/session.rs`
- `crates/agentgateway/src/mcp/router.rs`

**Deliverables**:
1. Add `registry` field to `Relay`
2. Modify `merge_tools()` to apply registry transformations
3. Modify `handle_call_tool()` for virtual tool routing
4. Wire registry through service factory

**Test coverage** (integration tests):
- ListTools returns virtual tools
- CallTool routes through virtual mapping
- Output transformation applied
- Non-virtualized tools pass through

## Phase 7: Configuration Schema

**Files to modify**:
- `crates/agentgateway/src/types/local.rs` (LocalConfig)
- `crates/agentgateway/src/types/agent.rs` (runtime types)
- `crates/xtask/src/schema.rs` (schema generation)

**Deliverables**:
1. `LocalRegistryConfig` type
2. Config parsing and validation
3. JSON Schema generation for registry config
4. Update example configs

## Phase 8: End-to-End Integration Tests

**Test scenarios**:

```rust
#[tokio::test]
async fn test_virtual_tool_basic() {
    // Start mock MCP server with "fetch_weather" tool
    let mock = MockMcpServer::new()
        .with_tool(
            Tool { name: "fetch_weather", ... },
            json!({"data": {"current": {"temp_f": 52.3}}})
        )
        .start().await.unwrap();

    // Create registry JSON
    let registry = json!({
        "tools": [{
            "name": "get_weather",
            "source": {"target": "weather", "tool": "fetch_weather"},
            "outputSchema": {
                "properties": {
                    "temperature": {"sourceField": "$.data.current.temp_f"}
                }
            }
        }]
    });

    // Start gateway with registry
    let gateway = TestGateway::start(&format!(r#"
        registry:
          source: file://{registry_path}
        backends:
          - mcp:
              targets:
                - name: weather
                  mcp:
                    host: localhost:{mock_port}
    "#)).await.unwrap();

    // Connect client and verify
    let client = TestMcpClient::new(gateway.port);

    let tools = client.list_tools().await.unwrap();
    assert!(tools.iter().any(|t| t.name == "get_weather"));
    assert!(tools.iter().all(|t| t.name != "fetch_weather"));

    let result = client.call_tool("get_weather", json!({"city": "Seattle"})).await.unwrap();
    assert_eq!(result["temperature"], 52.3);
}

#[tokio::test]
async fn test_default_injection() { ... }

#[tokio::test]
async fn test_field_hiding() { ... }

#[tokio::test]
async fn test_registry_hot_reload() { ... }

#[tokio::test]
async fn test_passthrough_non_virtualized() { ... }
```

## Phase 9: Documentation and Examples

**Deliverables**:
1. Update `schema/config.md` with registry options
2. Create `examples/registry/` with sample configs
3. Update `architecture/` docs
4. Add registry JSON schema to `schema/`

---

# Implementation Order

```
Phase 0 ------------------------------------------------------
   |  Test infrastructure (can run in parallel with Phase 1-3)
   |
Phase 1 --- Phase 2 --- Phase 3 ------------------------------
   |           |           |     Core types, JSONPath, defaults
   |           |           |
   +-----------+-----------+--> Phase 4 --- Phase 5 -----------
                                   |           |   Client, Store
                                   |           |
                                   +-----------+--> Phase 6 ----
                                                       |   Relay
                                                       |
                                                       v
                                                   Phase 7 ------
                                                       |   Config
                                                       |
                                                       v
                                                   Phase 8 ------
                                                       |   E2E tests
                                                       |
                                                       v
                                                   Phase 9
                                                       Docs
```

---

# Testing

The registry integration includes comprehensive test coverage at multiple levels:

## Running Tests

### Unit Tests

Run all registry unit tests (36 tests covering core functionality):

```bash
# Run all registry unit tests
cargo test --package agentgateway registry

# Run with output for debugging
cargo test --package agentgateway registry -- --nocapture

# Run a specific test
cargo test --package agentgateway test_compile_simple_registry
```

Unit tests are located in:
- `crates/agentgateway/src/mcp/registry/types.rs` - Type parsing tests
- `crates/agentgateway/src/mcp/registry/compiled.rs` - Compilation and transformation tests
- `crates/agentgateway/src/mcp/registry/client.rs` - Client and auth tests
- `crates/agentgateway/src/mcp/registry/store.rs` - Store and hot-reload tests

### Integration Tests

Run registry integration tests (8 tests covering end-to-end functionality):

```bash
# Run all registry integration tests
cargo test --package agentgateway --test integration registry

# Run with output
cargo test --package agentgateway --test integration registry -- --nocapture
```

Integration tests are located in:
- `crates/agentgateway/tests/tests/registry.rs`

### All Tests

Run both unit and integration tests:

```bash
# Run all registry-related tests
cargo test --package agentgateway registry
```

## Manual Testing

### Setup a Test Registry File

1. Create a registry JSON file (`test-registry.json`):

```json
{
  "schemaVersion": "1.0",
  "tools": [
    {
      "name": "get_weather",
      "source": {
        "target": "weather-server",
        "tool": "fetch_weather"
      },
      "description": "Get current weather for a city",
      "defaults": {
        "units": "metric"
      },
      "hideFields": ["debug_mode"],
      "outputSchema": {
        "type": "object",
        "properties": {
          "temperature": {
            "type": "number",
            "sourceField": "$.data.temp"
          },
          "conditions": {
            "type": "string",
            "sourceField": "$.data.weather"
          }
        }
      }
    }
  ]
}
```

2. Create a gateway config (`config.yaml`):

```yaml
registry:
  source: file:///path/to/test-registry.json
  refresh_interval: 30s

backends:
  - mcp:
      targets:
        - name: weather-server
          stdio:
            cmd: uvx
            args: ["mcp-server-weather"]
```

3. Start the gateway:

```bash
./target/release/agentgateway -f config.yaml
```

4. Test with an MCP client:

```bash
# Using curl with StreamableHTTP
curl -X POST http://localhost:15001/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

### Testing Hot Reload

1. Start gateway with file-based registry
2. Modify the registry JSON file
3. Observe logs for reload confirmation:
   ```
   INFO Config file changed, reloading...
   INFO Registry refreshed successfully
   ```
4. Re-query tools to verify changes took effect

### Testing HTTP Registry Source

1. Host a registry JSON file on an HTTP server:
   ```bash
   python -m http.server 8080
   ```

2. Configure gateway with HTTP source:
   ```yaml
   registry:
     source: http://localhost:8080/registry.json
     refresh_interval: 1m
     auth:
       bearer: ${REGISTRY_TOKEN}  # Optional
   ```

3. Verify periodic refresh in logs

### Testing Default Injection

1. Add a tool with defaults using environment variables:
   ```json
   {
     "name": "api_call",
     "source": {"target": "api-server", "tool": "call"},
     "defaults": {
       "api_key": "${API_KEY}",
       "timeout": 30
     }
   }
   ```

2. Set the environment variable:
   ```bash
   export API_KEY="my-secret-key"
   ```

3. Call the tool without providing `api_key` - verify it's injected

### Testing Output Transformation

1. Configure output schema with JSONPath:
   ```json
   {
     "outputSchema": {
       "properties": {
         "result": {
           "type": "string",
           "sourceField": "$.deeply.nested.value"
         }
       }
     }
   }
   ```

2. Backend returns: `{"deeply": {"nested": {"value": "hello"}}}`
3. Client receives: `{"result": "hello"}`

## Test Coverage Summary

| Component | Unit Tests | Integration Tests |
|-----------|------------|-------------------|
| Types/Parsing | 7 tests | 2 tests |
| Compilation | 15 tests | 1 test |
| JSONPath | 6 tests | 1 test |
| Default Injection | 4 tests | 1 test |
| Client | 6 tests | 1 test |
| Store | 4 tests | 1 test |
| **Total** | **36 tests** | **8 tests** |

---

# Open Questions

1. **Tool Conflict Resolution**: If registry defines `get_weather` but a backend also exposes `get_weather`, which wins?
   - Proposal: Registry wins, backend tool is shadowed (with warning log)

2. **Partial Registry Failures**: If registry fetch fails on refresh, keep old data or clear?
   - Proposal: Keep old data, log warning, retry

3. **Schema Validation**: Validate virtual tool schemas against source tool schemas at compile time?
   - Proposal: Yes, warn on incompatible overrides

4. **Multi-Registry**: Support multiple registries per gateway?
   - Proposal: Start with single global registry, extend later if needed

---

# References

- mcp-gateway-prototype source: `~/src/open_src/mcp-gateway-prototype/`
- agentgateway MCP handler: `crates/agentgateway/src/mcp/handler.rs`
- agentgateway session: `crates/agentgateway/src/mcp/session.rs`
- agentgateway state manager: `crates/agentgateway/src/state_manager.rs`
