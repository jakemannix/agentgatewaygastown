// Execution context for composition execution

use std::collections::HashMap;
use std::sync::Arc;

use opentelemetry::Context as OtelContext;
use serde_json::Value;
use tokio::sync::RwLock;

use super::ToolInvoker;
use crate::mcp::registry::compiled::CompiledRegistry;
use crate::telemetry::log::CompositionVerbosity;

/// Tracing context for composition execution observability.
/// Captures whether the trace is sampled and the verbosity level.
#[derive(Clone)]
pub struct TracingContext {
	/// Whether this trace is sampled (actually being recorded)
	pub sampled: bool,
	/// Verbosity level for composition tracing
	pub verbosity: CompositionVerbosity,
	/// OpenTelemetry context for span propagation
	pub parent_context: OtelContext,
}

impl TracingContext {
	/// Create a new tracing context
	pub fn new(sampled: bool, verbosity: CompositionVerbosity, parent_context: OtelContext) -> Self {
		Self {
			sampled,
			verbosity,
			parent_context,
		}
	}

	/// Create a tracing context that indicates "not sampled" (no tracing)
	pub fn not_sampled() -> Self {
		Self {
			sampled: false,
			verbosity: CompositionVerbosity::default(),
			parent_context: OtelContext::new(),
		}
	}
}

/// Execution context passed through composition execution
pub struct ExecutionContext {
	/// Original composition input
	pub input: Value,

	/// Step results (step_id -> result)
	step_results: Arc<RwLock<HashMap<String, Value>>>,

	/// Registry for tool lookups
	pub registry: Arc<CompiledRegistry>,

	/// Tool invoker for backend calls
	pub tool_invoker: Arc<dyn ToolInvoker>,

	/// Optional tracing context for observability
	pub tracing: Option<TracingContext>,
}

impl ExecutionContext {
	/// Create a new execution context without tracing
	pub fn new(
		input: Value,
		registry: Arc<CompiledRegistry>,
		tool_invoker: Arc<dyn ToolInvoker>,
	) -> Self {
		Self {
			input,
			step_results: Arc::new(RwLock::new(HashMap::new())),
			registry,
			tool_invoker,
			tracing: None,
		}
	}

	/// Create a new execution context with tracing enabled
	pub fn new_with_tracing(
		input: Value,
		registry: Arc<CompiledRegistry>,
		tool_invoker: Arc<dyn ToolInvoker>,
		tracing: TracingContext,
	) -> Self {
		Self {
			input,
			step_results: Arc::new(RwLock::new(HashMap::new())),
			registry,
			tool_invoker,
			tracing: Some(tracing),
		}
	}

	/// Store a step result
	pub async fn store_step_result(&self, step_id: &str, result: Value) {
		self
			.step_results
			.write()
			.await
			.insert(step_id.to_string(), result);
	}

	/// Get a step result
	pub async fn get_step_result(&self, step_id: &str) -> Option<Value> {
		self.step_results.read().await.get(step_id).cloned()
	}

	/// Create a child context (for nested patterns)
	/// Preserves the tracing context from the parent
	pub fn child(&self, input: Value) -> Self {
		Self {
			input,
			step_results: Arc::new(RwLock::new(HashMap::new())),
			registry: self.registry.clone(),
			tool_invoker: self.tool_invoker.clone(),
			tracing: self.tracing.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mcp::registry::executor::MockToolInvoker;
	use crate::mcp::registry::types::Registry;

	#[tokio::test]
	async fn test_store_and_get_step_result() {
		let registry = Registry::new();
		let compiled =
			Arc::new(crate::mcp::registry::compiled::CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(MockToolInvoker::new());

		let ctx = ExecutionContext::new(serde_json::json!({}), compiled, invoker);

		ctx
			.store_step_result("step1", serde_json::json!({"result": 42}))
			.await;

		let result = ctx.get_step_result("step1").await;
		assert!(result.is_some());
		assert_eq!(result.unwrap()["result"], 42);
	}

	#[tokio::test]
	async fn test_child_context() {
		let registry = Registry::new();
		let compiled =
			Arc::new(crate::mcp::registry::compiled::CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(MockToolInvoker::new());

		let parent_ctx = ExecutionContext::new(
			serde_json::json!({"parent": true}),
			compiled.clone(),
			invoker,
		);

		parent_ctx
			.store_step_result("parent_step", serde_json::json!({}))
			.await;

		let child_ctx = parent_ctx.child(serde_json::json!({"child": true}));

		// Child has its own step results
		assert!(child_ctx.get_step_result("parent_step").await.is_none());

		// Child has different input
		assert_eq!(child_ctx.input["child"], true);
	}
}
