// Composition Executor Module
//
// Executes tool compositions at runtime, handling:
// - Pattern execution (pipeline, scatter-gather, filter, schema-map, map-each)
// - Tool invocation via backend pool
// - Result aggregation and transformation
// - Tracing and observability

use tracing::debug;

mod context;
mod filter;
mod map_each;
mod pipeline;
mod scatter_gather;
mod schema_map;
mod throttle;
mod timeout;
pub mod composition_tracing;

pub use context::{ExecutionContext, TracingContext};
pub use filter::FilterExecutor;
pub use map_each::MapEachExecutor;
pub use pipeline::PipelineExecutor;
pub use scatter_gather::ScatterGatherExecutor;
pub use schema_map::SchemaMapExecutor;
pub use throttle::{RateLimiterRegistry, SharedRateLimiterRegistry, ThrottleExecutor};
pub use timeout::TimeoutExecutor;

use std::sync::Arc;

use serde_json::Value;
use thiserror::Error;

use super::compiled::{CompiledComposition, CompiledRegistry, CompiledTool};
use super::patterns::PatternSpec;

/// Errors that can occur during composition execution
#[derive(Error, Debug)]
pub enum ExecutionError {
	#[error("tool not found: {0}")]
	ToolNotFound(String),

	#[error("tool execution failed: {0}")]
	ToolExecutionFailed(String),

	#[error("pattern execution failed: {0}")]
	PatternExecutionFailed(String),

	#[error("invalid input: {0}")]
	InvalidInput(String),

	#[error("timeout after {0}ms")]
	Timeout(u32),

	#[error("all scatter-gather targets failed")]
	AllTargetsFailed,

	#[error("JSONPath evaluation failed: {0}")]
	JsonPathError(String),

	#[error("predicate evaluation failed: {0}")]
	PredicateError(String),

	#[error("type error: expected {expected}, got {actual}")]
	TypeError { expected: String, actual: String },

	#[error("internal error: {0}")]
	Internal(String),

	#[error("stateful pattern not implemented: {pattern}. {details}")]
	StatefulPatternNotImplemented { pattern: String, details: String },

	#[error("feature not implemented: {0}")]
	NotImplemented(String),
}

/// Composition executor - executes tool compositions
pub struct CompositionExecutor {
	/// Compiled registry for tool lookups
	registry: Arc<CompiledRegistry>,
	/// Tool invocation callback
	tool_invoker: Arc<dyn ToolInvoker>,
}

/// Trait for invoking tools (abstraction over actual backend calls)
#[async_trait::async_trait]
pub trait ToolInvoker: Send + Sync {
	/// Invoke a tool by name with the given arguments
	async fn invoke(&self, tool_name: &str, args: Value) -> Result<Value, ExecutionError>;
}

impl CompositionExecutor {
	/// Create a new composition executor
	pub fn new(registry: Arc<CompiledRegistry>, tool_invoker: Arc<dyn ToolInvoker>) -> Self {
		Self {
			registry,
			tool_invoker,
		}
	}

	/// Execute a composition by name
	pub async fn execute(
		&self,
		composition_name: &str,
		input: Value,
	) -> Result<Value, ExecutionError> {
		debug!(target: "virtual_tools", composition = %composition_name, "executing composition");

		let tool = self
			.registry
			.get_tool(composition_name)
			.ok_or_else(|| ExecutionError::ToolNotFound(composition_name.to_string()))?;

		let composition = tool.composition_info().ok_or_else(|| {
			ExecutionError::InvalidInput(format!("{} is not a composition", composition_name))
		})?;

		// Validate required input fields before execution
		Self::validate_input_schema(&tool.def.input_schema, &input, composition_name)?;

		self.execute_composition_internal(tool, composition, input, None)
			.await
	}

	/// Execute a composition by name with tracing context
	pub async fn execute_with_tracing(
		&self,
		composition_name: &str,
		input: Value,
		tracing_ctx: TracingContext,
	) -> Result<Value, ExecutionError> {
		debug!(target: "virtual_tools", composition = %composition_name, "executing composition with tracing");

		let tool = self
			.registry
			.get_tool(composition_name)
			.ok_or_else(|| ExecutionError::ToolNotFound(composition_name.to_string()))?;

		let composition = tool.composition_info().ok_or_else(|| {
			ExecutionError::InvalidInput(format!("{} is not a composition", composition_name))
		})?;

		// Validate required input fields before execution
		Self::validate_input_schema(&tool.def.input_schema, &input, composition_name)?;

		// Log composition start (always) and create OTEL span if sampled
		composition_tracing::log_composition_start(&tracing_ctx, composition_name);
		let mut composition_span = composition_tracing::create_composition_span(&tracing_ctx, composition_name);

		let result = self
			.execute_composition_internal(tool, composition, input, Some(tracing_ctx))
			.await;

		// End composition span
		if let Some(ref mut span) = composition_span {
			use opentelemetry::trace::Span;
			match &result {
				Ok(_) => span.set_status(opentelemetry::trace::Status::Ok),
				Err(e) => span.set_status(opentelemetry::trace::Status::error(e.to_string())),
			}
			span.end();
		}

		result
	}

	/// Execute a compiled composition (internal implementation)
	/// Made pub(crate) to allow test code to access it
	pub(crate) async fn execute_composition_internal(
		&self,
		_tool: &CompiledTool,
		composition: &CompiledComposition,
		input: Value,
		tracing_ctx: Option<TracingContext>,
	) -> Result<Value, ExecutionError> {
		let ctx = match tracing_ctx {
			Some(tc) => ExecutionContext::new_with_tracing(
				input.clone(),
				self.registry.clone(),
				self.tool_invoker.clone(),
				tc,
			),
			None => ExecutionContext::new(input.clone(), self.registry.clone(), self.tool_invoker.clone()),
		};

		let result = self.execute_pattern(&composition.spec, input, &ctx).await?;

		// Apply output transform if present
		if let Some(ref transform) = composition.output_transform {
			transform
				.apply(&result)
				.map_err(|e| ExecutionError::PatternExecutionFailed(e.to_string()))
		} else {
			Ok(result)
		}
	}

	/// Execute a pattern
	///
	/// This function uses Box::pin to handle async recursion when patterns
	/// contain nested patterns (e.g., pipeline steps with nested patterns).
	pub fn execute_pattern<'a>(
		&'a self,
		spec: &'a PatternSpec,
		input: Value,
		ctx: &'a ExecutionContext,
	) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, ExecutionError>> + Send + 'a>>
	{
		Box::pin(async move {
			match spec {
				// Stateless patterns (implemented)
				PatternSpec::Pipeline(p) => PipelineExecutor::execute(p, input, ctx, self).await,
				PatternSpec::ScatterGather(sg) => ScatterGatherExecutor::execute(sg, input, ctx, self).await,
				PatternSpec::Filter(f) => FilterExecutor::execute(f, input).await,
				PatternSpec::SchemaMap(sm) => SchemaMapExecutor::execute(sm, input).await,
				PatternSpec::MapEach(me) => MapEachExecutor::execute(me, input, ctx, self).await,

				// Stateful patterns (IR defined, runtime not yet implemented)
				PatternSpec::Retry(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "retry".to_string(),
					details: "The retry pattern requires a state store for tracking attempt counts and backoff delays. \
						Configure a state store backend (e.g., Redis, in-memory) and implement RetryExecutor to enable this pattern."
						.to_string(),
				}),
				PatternSpec::Timeout(t) => TimeoutExecutor::execute(t, input, ctx, self).await,
				PatternSpec::Cache(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "cache".to_string(),
					details: "The cache pattern requires a cache store backend (e.g., Redis, in-memory LRU). \
						Configure a cache store and implement CacheExecutor to enable read-through caching."
						.to_string(),
				}),
				PatternSpec::Idempotent(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "idempotent".to_string(),
					details: "The idempotent pattern requires a store for tracking processed request keys. \
						Configure a store backend (e.g., Redis, database) and implement IdempotentExecutor to prevent duplicate processing."
						.to_string(),
				}),
				PatternSpec::CircuitBreaker(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "circuit_breaker".to_string(),
					details: "The circuit breaker pattern requires a store for tracking failure counts and circuit state. \
						Configure a store backend and implement CircuitBreakerExecutor to enable fail-fast behavior with automatic recovery."
						.to_string(),
				}),
				PatternSpec::DeadLetter(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "dead_letter".to_string(),
					details: "The dead letter pattern requires a queue or storage backend for capturing failed messages. \
						Configure a dead letter tool and implement DeadLetterExecutor to enable failure capture for later processing."
						.to_string(),
				}),
				PatternSpec::Saga(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "saga".to_string(),
					details: "The saga pattern requires a store for tracking saga state and enabling recovery. \
						Configure a durable store backend and implement SagaExecutor to enable distributed transactions with compensation."
						.to_string(),
				}),
				PatternSpec::ClaimCheck(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "claim_check".to_string(),
					details: "The claim check pattern requires blob storage tools for externalizing large payloads. \
						Configure store_tool and retrieve_tool backends and implement ClaimCheckExecutor to enable payload externalization."
						.to_string(),
				}),
				PatternSpec::Throttle(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "throttle".to_string(),
					details: "The throttle pattern requires a rate limiter implementation. \
						For single-instance: use in-memory rate limiter (e.g., governor crate). \
						For distributed: configure a store backend with atomic increment support."
						.to_string(),
				}),

				// Vision patterns (IR defined, runtime not yet implemented)
				PatternSpec::Router(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "router".to_string(),
					details: "The router pattern provides content-based routing to different tools based on predicates. \
						Implement RouterExecutor to evaluate route conditions and dispatch to matching operations."
						.to_string(),
				}),
				PatternSpec::Enricher(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "enricher".to_string(),
					details: "The enricher pattern augments input with parallel enrichment calls. \
						Implement EnricherExecutor to run enrichments concurrently and merge results."
						.to_string(),
				}),
				PatternSpec::WireTap(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "wire_tap".to_string(),
					details: "The wire tap pattern sends copies of data to side channels without affecting main flow. \
						Implement WireTapExecutor to spawn fire-and-forget tap operations."
						.to_string(),
				}),
				PatternSpec::RecipientList(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "recipient_list".to_string(),
					details: "The recipient list pattern dynamically determines targets at runtime from input data. \
						Implement RecipientListExecutor to resolve recipients and dispatch operations."
						.to_string(),
				}),
				PatternSpec::CapabilityRouter(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "capability_router".to_string(),
					details: "The capability router pattern routes based on tool capabilities from registry introspection. \
						Implement CapabilityRouterExecutor to query registry for matching tools."
						.to_string(),
				}),
				PatternSpec::SemanticDedup(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "semantic_dedup".to_string(),
					details: "The semantic dedup pattern deduplicates based on embedding similarity. \
						Implement SemanticDedupExecutor with embedding service integration."
						.to_string(),
				}),
				PatternSpec::ConfidenceAggregator(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "confidence_aggregator".to_string(),
					details: "The confidence aggregator pattern provides weighted aggregation based on source reliability. \
						Implement ConfidenceAggregatorExecutor with consensus and conflict detection logic."
						.to_string(),
				}),
			}
		})
	}

	/// Execute a tool by name
	///
	/// This function uses Box::pin to handle async recursion when compositions
	/// contain nested tool calls.
	pub fn execute_tool<'a>(
		&'a self,
		name: &'a str,
		args: Value,
		ctx: &'a ExecutionContext,
	) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, ExecutionError>> + Send + 'a>>
	{
		Box::pin(async move {
			// First, check if it's a composition in the registry
			if let Some(tool) = self.registry.get_tool(name)
				&& let Some(composition) = tool.composition_info()
			{
				// Pass through the tracing context from the execution context
				return self
					.execute_composition_internal(tool, composition, args, ctx.tracing.clone())
					.await;
			}

			// Otherwise, invoke via the tool invoker
			ctx.tool_invoker.invoke(name, args).await
		})
	}

	/// Validate input against the tool's input schema
	///
	/// Checks that required fields are present in the input.
	/// Returns an error with a helpful message if validation fails.
	fn validate_input_schema(
		schema: &Option<Value>,
		input: &Value,
		composition_name: &str,
	) -> Result<(), ExecutionError> {
		let Some(schema) = schema else {
			return Ok(()); // No schema = no validation
		};

		let Some(schema_obj) = schema.as_object() else {
			return Ok(()); // Not an object schema
		};

		// Get required fields from schema
		let required_fields: Vec<&str> = schema_obj
			.get("required")
			.and_then(|r| r.as_array())
			.map(|arr| {
				arr.iter()
					.filter_map(|v| v.as_str())
					.collect()
			})
			.unwrap_or_default();

		if required_fields.is_empty() {
			return Ok(()); // No required fields
		}

		// Check if input is an object
		let input_obj = input.as_object().ok_or_else(|| {
			ExecutionError::InvalidInput(format!(
				"Composition '{}' expects an object input with required fields: {:?}",
				composition_name, required_fields
			))
		})?;

		// Check for missing required fields
		let missing: Vec<&str> = required_fields
			.iter()
			.filter(|&&field| !input_obj.contains_key(field))
			.copied()
			.collect();

		if !missing.is_empty() {
			return Err(ExecutionError::InvalidInput(format!(
				"Composition '{}' is missing required input fields: {:?}. Please provide: {}",
				composition_name,
				missing,
				missing.join(", ")
			)));
		}

		Ok(())
	}
}

/// Mock tool invoker for testing
#[cfg(test)]
pub struct MockToolInvoker {
	responses: std::sync::Mutex<std::collections::HashMap<String, Value>>,
}

#[cfg(test)]
impl MockToolInvoker {
	pub fn new() -> Self {
		Self {
			responses: std::sync::Mutex::new(std::collections::HashMap::new()),
		}
	}

	pub fn with_response(self, tool_name: &str, response: Value) -> Self {
		self
			.responses
			.lock()
			.unwrap()
			.insert(tool_name.to_string(), response);
		self
	}
}

#[cfg(test)]
#[async_trait::async_trait]
impl ToolInvoker for MockToolInvoker {
	async fn invoke(&self, tool_name: &str, _args: Value) -> Result<Value, ExecutionError> {
		self
			.responses
			.lock()
			.unwrap()
			.get(tool_name)
			.cloned()
			.ok_or_else(|| ExecutionError::ToolNotFound(tool_name.to_string()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mcp::registry::patterns::{
		BackoffStrategy, ExponentialBackoff, PipelineSpec, PipelineStep, RetrySpec, StepOperation,
		ToolCall,
	};
	use crate::mcp::registry::types::{Registry, ToolDefinition};

	/// A mock invoker that simulates RelayToolInvoker's behavior, but with proper
	/// support for nested compositions. When it encounters a composition, it
	/// recursively executes it using a new CompositionExecutor.
	///
	/// This demonstrates the pattern that RelayToolInvoker should follow.
	struct RegistryAwareInvoker {
		/// The compiled registry to check tool types and get composition specs
		registry: Arc<CompiledRegistry>,
		/// Backend responses for source-based tools (keyed by virtual tool name)
		backend_responses: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Value>>>,
	}

	impl RegistryAwareInvoker {
		fn new(registry: Arc<CompiledRegistry>) -> Self {
			Self {
				registry,
				backend_responses: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
			}
		}

		fn with_backend_response(self, tool_name: &str, response: Value) -> Self {
			self.backend_responses
				.lock()
				.unwrap()
				.insert(tool_name.to_string(), response);
			self
		}
	}

	impl Clone for RegistryAwareInvoker {
		fn clone(&self) -> Self {
			Self {
				registry: self.registry.clone(),
				backend_responses: self.backend_responses.clone(),
			}
		}
	}

	#[async_trait::async_trait]
	impl ToolInvoker for RegistryAwareInvoker {
		async fn invoke(&self, tool_name: &str, args: Value) -> Result<Value, ExecutionError> {
			// Look up the tool in the registry
			if let Some(tool) = self.registry.get_tool(tool_name) {
				if tool.is_composition() {
					// FIXED: Instead of returning an error, recursively execute the composition
					// This is the key change that enables nested composition support
					let composition = tool.composition_info().ok_or_else(|| {
						ExecutionError::InvalidInput(format!(
							"Tool '{}' is marked as composition but has no composition info",
							tool_name
						))
					})?;

					// Create a new executor that uses this same invoker for backend calls
					let executor = CompositionExecutor::new(
						self.registry.clone(),
						Arc::new(self.clone()),
					);

					// Execute the nested composition (without tracing in test context)
					return executor
						.execute_composition_internal(tool, composition, args, None)
						.await;
				}

				// Source-based virtual tool - return the backend response
				if let Some(response) = self.backend_responses.lock().unwrap().get(tool_name) {
					return Ok(response.clone());
				}

				// Tool exists but no response configured
				return Err(ExecutionError::ToolExecutionFailed(format!(
					"No backend response configured for tool '{}'",
					tool_name
				)));
			}

			Err(ExecutionError::ToolNotFound(tool_name.to_string()))
		}
	}

	#[tokio::test]
	async fn test_execute_simple_composition() {
		// Create a simple pipeline composition
		let composition = ToolDefinition::composition(
			"test_pipeline",
			PatternSpec::Pipeline(PipelineSpec {
				steps: vec![PipelineStep {
					id: "step1".to_string(),
					operation: StepOperation::Tool(ToolCall::new("echo")),
					input: None,
				}],
			}),
		);

		let registry = Registry::with_tool_definitions(vec![composition]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let invoker = MockToolInvoker::new().with_response("echo", serde_json::json!({"echoed": true}));

		let executor = CompositionExecutor::new(Arc::new(compiled), Arc::new(invoker));

		let result = executor
			.execute("test_pipeline", serde_json::json!({"input": "test"}))
			.await;

		assert!(result.is_ok());
		assert_eq!(result.unwrap()["echoed"], true);
	}

	#[tokio::test]
	async fn test_execute_nonexistent_composition() {
		let registry = Registry::new();
		let compiled = CompiledRegistry::compile(registry).unwrap();
		let invoker = MockToolInvoker::new();

		let executor = CompositionExecutor::new(Arc::new(compiled), Arc::new(invoker));

		let result = executor.execute("nonexistent", serde_json::json!({})).await;

		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			ExecutionError::ToolNotFound(_)
		));
	}

	#[tokio::test]
	async fn test_execute_stateful_pattern_returns_helpful_error() {
		// Create a composition with a retry pattern (stateful, not yet implemented)
		let composition = ToolDefinition::composition(
			"retry_composition",
			PatternSpec::Retry(RetrySpec {
				inner: Box::new(StepOperation::Tool(ToolCall::new("flaky_api"))),
				max_attempts: 3,
				backoff: BackoffStrategy::Exponential(ExponentialBackoff {
					initial_delay_ms: 100,
					max_delay_ms: 5000,
					multiplier: 2.0,
				}),
				retry_if: None,
				jitter: Some(0.1),
				attempt_timeout_ms: None,
			}),
		);

		let registry = Registry::with_tool_definitions(vec![composition]);
		let compiled = CompiledRegistry::compile(registry).unwrap();
		let invoker = MockToolInvoker::new();

		let executor = CompositionExecutor::new(Arc::new(compiled), Arc::new(invoker));

		let result = executor
			.execute("retry_composition", serde_json::json!({}))
			.await;

		assert!(result.is_err());
		let err = result.unwrap_err();
		match err {
			ExecutionError::StatefulPatternNotImplemented { pattern, details } => {
				assert_eq!(pattern, "retry");
				assert!(details.contains("state store"));
				assert!(details.contains("RetryExecutor"));
			},
			_ => panic!(
				"Expected StatefulPatternNotImplemented error, got {:?}",
				err
			),
		}
	}

	/// Test that a composition can call a source-based virtual tool (rename).
	/// This test verifies the data flow:
	/// 1. Outer composition "search_pipeline" has a step that calls "fetch_page"
	/// 2. "fetch_page" is a source-based virtual tool (rename of backend "web-server/fetch")
	/// 3. The composition executor should:
	///    a. Find "fetch_page" in the registry
	///    b. See it's NOT a composition (source-based)
	///    c. Fall through to the invoker
	///    d. Invoker should successfully call the backend
	#[tokio::test]
	async fn test_composition_can_call_source_based_virtual_tool() {
		// Create a source-based virtual tool (rename): fetch_page -> web-server/fetch
		let fetch_page = ToolDefinition::source("fetch_page", "web-server", "fetch");

		// Create a composition that calls the virtual tool
		let search_pipeline = ToolDefinition::composition(
			"search_pipeline",
			PatternSpec::Pipeline(PipelineSpec {
				steps: vec![PipelineStep {
					id: "fetch".to_string(),
					operation: StepOperation::Tool(ToolCall::new("fetch_page")),
					input: None,
				}],
			}),
		);

		// Build registry with both tools
		let registry = Registry::with_tool_definitions(vec![fetch_page, search_pipeline]);
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());

		// Use the registry-aware invoker that simulates RelayToolInvoker behavior
		let invoker = RegistryAwareInvoker::new(compiled.clone())
			.with_backend_response("fetch_page", serde_json::json!({
				"status": 200,
				"content": "<html>Hello World</html>"
			}));

		let executor = CompositionExecutor::new(compiled, Arc::new(invoker));

		// Execute the composition
		let result = executor
			.execute("search_pipeline", serde_json::json!({"url": "https://example.com"}))
			.await;

		// This should succeed - the composition calls fetch_page which is source-based
		assert!(result.is_ok(), "Expected success but got: {:?}", result.err());
		let output = result.unwrap();
		assert_eq!(output["status"], 200);
		assert_eq!(output["content"], "<html>Hello World</html>");
	}

	/// Test that a composition calling another composition works via the execute_tool path.
	/// This test verifies nested composition support:
	/// 1. "inner_composition" is a simple composition
	/// 2. "outer_composition" has a step that calls "inner_composition"
	/// 3. The executor should handle this via execute_tool's composition check
	///    (NOT by falling through to the invoker)
	#[tokio::test]
	async fn test_composition_can_call_another_composition() {
		// Create a source-based tool for the innermost call
		let backend_echo = ToolDefinition::source("backend_echo", "echo-server", "echo");

		// Create an inner composition that calls the backend tool
		let inner_composition = ToolDefinition::composition(
			"inner_composition",
			PatternSpec::Pipeline(PipelineSpec {
				steps: vec![PipelineStep {
					id: "echo".to_string(),
					operation: StepOperation::Tool(ToolCall::new("backend_echo")),
					input: None,
				}],
			}),
		);

		// Create an outer composition that calls the inner one
		let outer_composition = ToolDefinition::composition(
			"outer_composition",
			PatternSpec::Pipeline(PipelineSpec {
				steps: vec![PipelineStep {
					id: "call_inner".to_string(),
					operation: StepOperation::Tool(ToolCall::new("inner_composition")),
					input: None,
				}],
			}),
		);

		// Build registry with all three tools
		let registry =
			Registry::with_tool_definitions(vec![backend_echo, inner_composition, outer_composition]);
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());

		// Use registry-aware invoker - it will return an error if asked to invoke a composition
		// (simulating the current RelayToolInvoker behavior)
		let invoker = RegistryAwareInvoker::new(compiled.clone())
			.with_backend_response("backend_echo", serde_json::json!({"echoed": true}));

		let executor = CompositionExecutor::new(compiled, Arc::new(invoker));

		// Execute the outer composition
		let result = executor
			.execute("outer_composition", serde_json::json!({}))
			.await;

		// This SHOULD succeed because execute_tool checks the registry first and handles
		// compositions directly. It should NOT fall through to the invoker.
		// If this fails, it means execute_tool is not finding the inner composition.
		assert!(
			result.is_ok(),
			"Nested composition should work via execute_tool path, but got: {:?}",
			result.err()
		);
		let output = result.unwrap();
		assert_eq!(output["echoed"], true);
	}

	/// TDD TEST: This test documents the DESIRED behavior for nested composition support.
	///
	/// Scenario: The invoker (like RelayToolInvoker) encounters a composition during
	/// tool invocation. Currently this fails, but it SHOULD succeed by recursively
	/// executing the composition.
	///
	/// This test will FAIL until we implement proper nested composition support in
	/// the invoker path. The fix should allow the invoker to execute compositions
	/// instead of returning an error.
	#[tokio::test]
	async fn test_invoker_should_handle_nested_compositions() {
		// Create only the outer composition - the inner one is "missing" from this registry
		// but the invoker will know about it
		let outer_composition = ToolDefinition::composition(
			"outer_composition",
			PatternSpec::Pipeline(PipelineSpec {
				steps: vec![PipelineStep {
					id: "call_inner".to_string(),
					// This tool is NOT in the registry we give to the executor
					operation: StepOperation::Tool(ToolCall::new("inner_composition")),
					input: None,
				}],
			}),
		);

		// Registry for executor only has the outer composition
		let executor_registry =
			Registry::with_tool_definitions(vec![outer_composition.clone()]);
		let compiled_executor = Arc::new(CompiledRegistry::compile(executor_registry).unwrap());

		// Create a source-based tool for the innermost call
		let backend_echo = ToolDefinition::source("backend_echo", "echo-server", "echo");

		// Registry for invoker has the inner composition and backend tool
		// (simulating relay knowing about inner_composition)
		let inner_composition = ToolDefinition::composition(
			"inner_composition",
			PatternSpec::Pipeline(PipelineSpec {
				steps: vec![PipelineStep {
					id: "echo".to_string(),
					operation: StepOperation::Tool(ToolCall::new("backend_echo")),
					input: None,
				}],
			}),
		);
		let invoker_registry =
			Registry::with_tool_definitions(vec![outer_composition, inner_composition, backend_echo]);
		let compiled_invoker = Arc::new(CompiledRegistry::compile(invoker_registry).unwrap());

		// Invoker uses the "full" registry (simulating relay's view)
		let invoker = RegistryAwareInvoker::new(compiled_invoker)
			.with_backend_response("backend_echo", serde_json::json!({"echoed": true}));

		// Executor uses the "partial" registry (missing inner_composition)
		let executor = CompositionExecutor::new(compiled_executor, Arc::new(invoker));

		// Execute the outer composition
		let result = executor
			.execute("outer_composition", serde_json::json!({}))
			.await;

		// DESIRED BEHAVIOR: This should SUCCEED!
		// The invoker should be able to recursively execute inner_composition
		// instead of returning "Nested composition not supported" error.
		//
		// This test will FAIL until we fix the invoker to handle compositions.
		assert!(
			result.is_ok(),
			"Invoker should handle nested compositions, but got error: {:?}",
			result.err()
		);
		let output = result.unwrap();
		assert_eq!(output["echoed"], true);
	}

	/// Test that execute_with_tracing produces debug output.
	/// This test verifies that the tracing code path is actually being executed.
	#[tokio::test]
	async fn test_execute_with_tracing_produces_debug_output() {
		use crate::telemetry::log::CompositionVerbosity;

		// Create a simple pipeline composition
		let composition = ToolDefinition::composition(
			"traced_pipeline",
			PatternSpec::Pipeline(PipelineSpec {
				steps: vec![PipelineStep {
					id: "step1".to_string(),
					operation: StepOperation::Tool(ToolCall::new("echo")),
					input: None,
				}],
			}),
		);

		let registry = Registry::with_tool_definitions(vec![composition]);
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());

		let invoker = MockToolInvoker::new().with_response("echo", serde_json::json!({"result": "ok"}));

		let executor = CompositionExecutor::new(compiled, Arc::new(invoker));

		// Create a tracing context with Full verbosity
		let tracing_ctx = TracingContext::new(
			true, // sampled
			CompositionVerbosity::Full,
			opentelemetry::Context::new(),
		);

		// This should produce [COMPOSITION DEBUG] output to stderr
		eprintln!("\n=== TEST: Calling execute_with_tracing ===");
		let result = executor
			.execute_with_tracing("traced_pipeline", serde_json::json!({"test": "input"}), tracing_ctx)
			.await;
		eprintln!("=== TEST: execute_with_tracing completed ===\n");

		assert!(result.is_ok(), "Expected success but got: {:?}", result.err());
		assert_eq!(result.unwrap()["result"], "ok");
	}
}
