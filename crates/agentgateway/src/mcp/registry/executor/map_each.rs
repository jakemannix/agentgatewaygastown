// Map Each pattern executor

use serde_json::Value;

use super::context::ExecutionContext;
use super::{CompositionExecutor, ExecutionError};
use crate::mcp::registry::patterns::{MapEachInner, MapEachSpec};

/// Executor for map-each patterns
pub struct MapEachExecutor;

impl MapEachExecutor {
	/// Execute a map-each pattern
	pub async fn execute(
		spec: &MapEachSpec,
		input: Value,
		ctx: &ExecutionContext,
		executor: &CompositionExecutor,
	) -> Result<Value, ExecutionError> {
		let arr = input.as_array().ok_or_else(|| ExecutionError::TypeError {
			expected: "array".to_string(),
			actual: Self::value_type_name(&input),
		})?;

		let mut results = Vec::with_capacity(arr.len());

		for item in arr {
			let result = Self::execute_inner(&spec.inner, item.clone(), ctx, executor).await?;
			results.push(result);
		}

		Ok(Value::Array(results))
	}

	/// Execute the inner operation for one element
	async fn execute_inner(
		inner: &MapEachInner,
		item: Value,
		ctx: &ExecutionContext,
		executor: &CompositionExecutor,
	) -> Result<Value, ExecutionError> {
		match inner {
			MapEachInner::Tool(name) => executor.execute_tool(name, item, ctx).await,
			MapEachInner::Pattern(pattern) => {
				let child_ctx = ctx.child(item.clone());
				executor.execute_pattern(pattern, item, &child_ctx).await
			},
		}
	}

	fn value_type_name(value: &Value) -> String {
		match value {
			Value::Null => "null",
			Value::Bool(_) => "boolean",
			Value::Number(_) => "number",
			Value::String(_) => "string",
			Value::Array(_) => "array",
			Value::Object(_) => "object",
		}
		.to_string()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mcp::registry::executor::MockToolInvoker;
	use crate::mcp::registry::patterns::{FieldSource, LiteralValue, PatternSpec, SchemaMapSpec};
	use crate::mcp::registry::types::Registry;
	use crate::mcp::registry::CompiledRegistry;
	use serde_json::json;
	use std::collections::HashMap;
	use std::sync::Arc;

	fn setup_context_and_executor(invoker: MockToolInvoker) -> (ExecutionContext, CompositionExecutor) {
		let registry = Registry::new();
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(invoker);

		let ctx = ExecutionContext::new(json!({}), compiled.clone(), invoker.clone());
		let executor = CompositionExecutor::new(compiled, invoker);

		(ctx, executor)
	}

	#[tokio::test]
	async fn test_map_each_tool() {
		// Create invoker that echoes input with a marker
		let invoker = MockToolInvoker::new()
			.with_response("process", json!({"processed": true}));

		let (ctx, executor) = setup_context_and_executor(invoker);

		let spec = MapEachSpec { inner: MapEachInner::Tool("process".to_string()) };

		let input = json!([{"id": 1}, {"id": 2}, {"id": 3}]);
		let result = MapEachExecutor::execute(&spec, input, &ctx, &executor).await;

		assert!(result.is_ok());
		let arr = result.unwrap();
		assert!(arr.is_array());
		assert_eq!(arr.as_array().unwrap().len(), 3);
	}

	#[tokio::test]
	async fn test_map_each_pattern() {
		let invoker = MockToolInvoker::new();
		let (ctx, executor) = setup_context_and_executor(invoker);

		// Use a schema map pattern to transform each element
		let inner_pattern = PatternSpec::SchemaMap(SchemaMapSpec {
			mappings: HashMap::from([
				("name".to_string(), FieldSource::Path("$.title".to_string())),
				("source".to_string(), FieldSource::Literal(LiteralValue::StringValue("processed".to_string()))),
			]),
		});

		let spec = MapEachSpec { inner: MapEachInner::Pattern(Box::new(inner_pattern)) };

		let input = json!([
			{"title": "Item 1"},
			{"title": "Item 2"}
		]);

		let result = MapEachExecutor::execute(&spec, input, &ctx, &executor).await;

		assert!(result.is_ok());
		let arr = result.unwrap();
		let items = arr.as_array().unwrap();

		assert_eq!(items.len(), 2);
		assert_eq!(items[0]["name"], "Item 1");
		assert_eq!(items[0]["source"], "processed");
		assert_eq!(items[1]["name"], "Item 2");
	}

	#[tokio::test]
	async fn test_map_each_non_array_error() {
		let invoker = MockToolInvoker::new();
		let (ctx, executor) = setup_context_and_executor(invoker);

		let spec = MapEachSpec { inner: MapEachInner::Tool("tool".to_string()) };

		let input = json!({"not": "an array"});
		let result = MapEachExecutor::execute(&spec, input, &ctx, &executor).await;

		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), ExecutionError::TypeError { .. }));
	}

	#[tokio::test]
	async fn test_map_each_empty_array() {
		let invoker = MockToolInvoker::new();
		let (ctx, executor) = setup_context_and_executor(invoker);

		let spec = MapEachSpec { inner: MapEachInner::Tool("tool".to_string()) };

		let input = json!([]);
		let result = MapEachExecutor::execute(&spec, input, &ctx, &executor).await;

		assert!(result.is_ok());
		let arr = result.unwrap();
		assert!(arr.as_array().unwrap().is_empty());
	}
}

