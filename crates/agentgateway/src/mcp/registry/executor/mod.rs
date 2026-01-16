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

pub use context::ExecutionContext;
pub use filter::FilterExecutor;
pub use map_each::MapEachExecutor;
pub use pipeline::PipelineExecutor;
pub use scatter_gather::ScatterGatherExecutor;
pub use schema_map::SchemaMapExecutor;

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
		Self { registry, tool_invoker }
	}

	/// Execute a composition by name
	pub async fn execute(&self, composition_name: &str, input: Value) -> Result<Value, ExecutionError> {
		debug!(target: "virtual_tools", composition = %composition_name, "executing composition");

		let tool = self
			.registry
			.get_tool(composition_name)
			.ok_or_else(|| ExecutionError::ToolNotFound(composition_name.to_string()))?;

		let composition = tool
			.composition_info()
			.ok_or_else(|| ExecutionError::InvalidInput(format!("{} is not a composition", composition_name)))?;

		self.execute_composition(tool, composition, input).await
	}

	/// Execute a compiled composition
	async fn execute_composition(
		&self,
		_tool: &CompiledTool,
		composition: &CompiledComposition,
		input: Value,
	) -> Result<Value, ExecutionError> {
		let ctx = ExecutionContext::new(input.clone(), self.registry.clone(), self.tool_invoker.clone());

		let result = self.execute_pattern(&composition.spec, input, &ctx).await?;

		// Apply output transform if present
		if let Some(ref transform) = composition.output_transform {
			transform.apply(&result).map_err(|e| ExecutionError::PatternExecutionFailed(e.to_string()))
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
	) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, ExecutionError>> + Send + 'a>> {
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
				PatternSpec::Timeout(_) => Err(ExecutionError::StatefulPatternNotImplemented {
					pattern: "timeout".to_string(),
					details: "The timeout pattern requires async cancellation support. \
						Implement TimeoutExecutor with tokio::time::timeout to enable this pattern."
						.to_string(),
				}),
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
	) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, ExecutionError>> + Send + 'a>> {
		Box::pin(async move {
			// First, check if it's a composition in the registry
			if let Some(tool) = self.registry.get_tool(name) {
				if let Some(composition) = tool.composition_info() {
					return self.execute_composition(tool, composition, args).await;
				}
			}

			// Otherwise, invoke via the tool invoker
			ctx.tool_invoker.invoke(name, args).await
		})
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
		Self { responses: std::sync::Mutex::new(std::collections::HashMap::new()) }
	}

	pub fn with_response(self, tool_name: &str, response: Value) -> Self {
		self.responses.lock().unwrap().insert(tool_name.to_string(), response);
		self
	}
}

#[cfg(test)]
#[async_trait::async_trait]
impl ToolInvoker for MockToolInvoker {
	async fn invoke(&self, tool_name: &str, _args: Value) -> Result<Value, ExecutionError> {
		self.responses
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
		BackoffStrategy, ExponentialBackoff, PipelineSpec, PipelineStep, RetrySpec, StepOperation, ToolCall,
	};
	use crate::mcp::registry::types::{Registry, ToolDefinition};

	#[tokio::test]
	async fn test_execute_simple_composition() {
		// Create a simple pipeline composition
		let composition = ToolDefinition::composition(
			"test_pipeline",
			PatternSpec::Pipeline(PipelineSpec {
				steps: vec![PipelineStep {
					id: "step1".to_string(),
					operation: StepOperation::Tool(ToolCall { name: "echo".to_string() }),
					input: None,
				}],
			}),
		);

		let registry = Registry::with_tool_definitions(vec![composition]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let invoker = MockToolInvoker::new().with_response("echo", serde_json::json!({"echoed": true}));

		let executor = CompositionExecutor::new(Arc::new(compiled), Arc::new(invoker));

		let result = executor.execute("test_pipeline", serde_json::json!({"input": "test"})).await;

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
		assert!(matches!(result.unwrap_err(), ExecutionError::ToolNotFound(_)));
	}

	#[tokio::test]
	async fn test_execute_stateful_pattern_returns_helpful_error() {
		// Create a composition with a retry pattern (stateful, not yet implemented)
		let composition = ToolDefinition::composition(
			"retry_composition",
			PatternSpec::Retry(RetrySpec {
				inner: Box::new(StepOperation::Tool(ToolCall { name: "flaky_api".to_string() })),
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

		let result = executor.execute("retry_composition", serde_json::json!({})).await;

		assert!(result.is_err());
		let err = result.unwrap_err();
		match err {
			ExecutionError::StatefulPatternNotImplemented { pattern, details } => {
				assert_eq!(pattern, "retry");
				assert!(details.contains("state store"));
				assert!(details.contains("RetryExecutor"));
			}
			_ => panic!("Expected StatefulPatternNotImplemented error, got {:?}", err),
		}
	}
}

