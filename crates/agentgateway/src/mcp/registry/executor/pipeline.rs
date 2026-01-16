// Pipeline pattern executor

use serde_json::Value;
use serde_json_path::JsonPath;

use super::context::ExecutionContext;
use super::{CompositionExecutor, ExecutionError};
use crate::mcp::registry::patterns::{DataBinding, PipelineSpec, StepOperation};

/// Executor for pipeline patterns
pub struct PipelineExecutor;

impl PipelineExecutor {
	/// Execute a pipeline pattern
	pub async fn execute(
		spec: &PipelineSpec,
		input: Value,
		ctx: &ExecutionContext,
		executor: &CompositionExecutor,
	) -> Result<Value, ExecutionError> {
		let mut current_result = input.clone();

		for step in &spec.steps {
			// Resolve input for this step
			let step_input = if let Some(ref binding) = step.input {
				Self::resolve_binding(binding, &input, ctx).await?
			} else {
				// Default: use previous step's output (or composition input for first step)
				current_result.clone()
			};

			// Execute the step operation
			let result = match &step.operation {
				StepOperation::Tool(tc) => executor.execute_tool(&tc.name, step_input, ctx).await?,
				StepOperation::Pattern(pattern) => {
					let child_ctx = ctx.child(step_input.clone());
					executor.execute_pattern(pattern, step_input, &child_ctx).await?
				},
			};

			// Store result for potential reference by later steps
			ctx.store_step_result(&step.id, result.clone()).await;
			current_result = result;
		}

		Ok(current_result)
	}

	/// Resolve a data binding to a value
	async fn resolve_binding(binding: &DataBinding, input: &Value, ctx: &ExecutionContext) -> Result<Value, ExecutionError> {
		match binding {
			DataBinding::Input(ib) => Self::apply_jsonpath(&ib.path, input),
			DataBinding::Step(sb) => {
				let step_result = ctx
					.get_step_result(&sb.step_id)
					.await
					.ok_or_else(|| ExecutionError::InvalidInput(format!("step {} not found", sb.step_id)))?;
				Self::apply_jsonpath(&sb.path, &step_result)
			},
			DataBinding::Constant(value) => Ok(value.clone()),
			DataBinding::Construct(cb) => {
				// Build an object by resolving each field's binding
				let mut obj = serde_json::Map::new();
				for (field_name, field_binding) in &cb.fields {
					let field_value = Box::pin(Self::resolve_binding(field_binding, input, ctx)).await?;
					obj.insert(field_name.clone(), field_value);
				}
				Ok(Value::Object(obj))
			},
		}
	}

	/// Apply a JSONPath to extract a value
	fn apply_jsonpath(path: &str, value: &Value) -> Result<Value, ExecutionError> {
		// Handle root path specially
		if path == "$" {
			return Ok(value.clone());
		}

		let jsonpath =
			JsonPath::parse(path).map_err(|e| ExecutionError::JsonPathError(format!("{}: {}", path, e)))?;

		let nodes = jsonpath.query(value);
		let results: Vec<_> = nodes.iter().map(|v| (*v).clone()).collect();

		Ok(match results.len() {
			0 => Value::Null,
			1 => results.into_iter().next().unwrap(),
			_ => Value::Array(results),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mcp::registry::executor::MockToolInvoker;
	use crate::mcp::registry::patterns::{InputBinding, PipelineStep, StepBinding, ToolCall};
	use crate::mcp::registry::types::Registry;
	use crate::mcp::registry::CompiledRegistry;
	use std::sync::Arc;

	fn setup_context_and_executor(invoker: MockToolInvoker) -> (ExecutionContext, CompositionExecutor) {
		let registry = Registry::new();
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(invoker);

		let ctx = ExecutionContext::new(serde_json::json!({}), compiled.clone(), invoker.clone());
		let executor = CompositionExecutor::new(compiled, invoker);

		(ctx, executor)
	}

	#[tokio::test]
	async fn test_simple_pipeline() {
		let invoker = MockToolInvoker::new()
			.with_response("step1_tool", serde_json::json!({"step1": "done"}))
			.with_response("step2_tool", serde_json::json!({"step2": "done"}));

		let (ctx, executor) = setup_context_and_executor(invoker);

		let spec = PipelineSpec {
			steps: vec![
				PipelineStep {
					id: "s1".to_string(),
					operation: StepOperation::Tool(ToolCall { name: "step1_tool".to_string() }),
					input: None,
				},
				PipelineStep {
					id: "s2".to_string(),
					operation: StepOperation::Tool(ToolCall { name: "step2_tool".to_string() }),
					input: None,
				},
			],
		};

		let result = PipelineExecutor::execute(&spec, serde_json::json!({}), &ctx, &executor).await;

		assert!(result.is_ok());
		assert_eq!(result.unwrap()["step2"], "done");
	}

	#[tokio::test]
	async fn test_pipeline_with_input_binding() {
		let invoker = MockToolInvoker::new().with_response("search", serde_json::json!({"results": ["a", "b"]}));

		let (ctx, executor) = setup_context_and_executor(invoker);

		let spec = PipelineSpec {
			steps: vec![PipelineStep {
				id: "search".to_string(),
				operation: StepOperation::Tool(ToolCall { name: "search".to_string() }),
				input: Some(DataBinding::Input(InputBinding { path: "$.query".to_string() })),
			}],
		};

		let input = serde_json::json!({"query": "test query"});
		let result = PipelineExecutor::execute(&spec, input, &ctx, &executor).await;

		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_pipeline_with_step_binding() {
		let invoker = MockToolInvoker::new()
			.with_response("search", serde_json::json!({"results": ["a", "b"]}))
			.with_response("process", serde_json::json!({"processed": true}));

		let (ctx, executor) = setup_context_and_executor(invoker);

		let spec = PipelineSpec {
			steps: vec![
				PipelineStep {
					id: "search".to_string(),
					operation: StepOperation::Tool(ToolCall { name: "search".to_string() }),
					input: None,
				},
				PipelineStep {
					id: "process".to_string(),
					operation: StepOperation::Tool(ToolCall { name: "process".to_string() }),
					input: Some(DataBinding::Step(StepBinding {
						step_id: "search".to_string(),
						path: "$.results".to_string(),
					})),
				},
			],
		};

		let result = PipelineExecutor::execute(&spec, serde_json::json!({}), &ctx, &executor).await;

		assert!(result.is_ok());
		assert_eq!(result.unwrap()["processed"], true);
	}

	#[tokio::test]
	async fn test_apply_jsonpath() {
		let value = serde_json::json!({
			"data": {
				"items": [1, 2, 3]
			}
		});

		// Root path
		let result = PipelineExecutor::apply_jsonpath("$", &value).unwrap();
		assert_eq!(result, value);

		// Nested path
		let result = PipelineExecutor::apply_jsonpath("$.data.items", &value).unwrap();
		assert_eq!(result, serde_json::json!([1, 2, 3]));

		// Single value
		let result = PipelineExecutor::apply_jsonpath("$.data.items[0]", &value).unwrap();
		assert_eq!(result, serde_json::json!(1));
	}
}

