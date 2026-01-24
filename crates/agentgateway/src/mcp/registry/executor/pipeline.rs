// Pipeline pattern executor

use std::time::Instant;

use serde_json::Value;
use serde_json_path::JsonPath;

use super::context::ExecutionContext;
use super::composition_tracing as exec_tracing;
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

			// Determine operation type for tracing
			let operation_type = match &step.operation {
				StepOperation::Tool(_) => "tool",
				StepOperation::Pattern(_) => "pattern",
				StepOperation::Agent(_) => "agent",
			};

			// Log step start and create OTEL span if sampled
			// Debug logging happens regardless of OTEL sampling
			if let Some(ref tracing_ctx) = ctx.tracing {
				exec_tracing::log_step_start(tracing_ctx, &step.id, operation_type, Some(&step_input));
			}
			let mut span = ctx
				.tracing
				.as_ref()
				.filter(|t| t.sampled)
				.and_then(|t| exec_tracing::create_step_span(t, &step.id, operation_type, Some(&step_input)));

			let start = Instant::now();

			// Execute the step operation
			let result = match &step.operation {
				StepOperation::Tool(tc) => executor.execute_tool(&tc.qualified_name(), step_input, ctx).await,
				StepOperation::Pattern(pattern) => {
					let child_ctx = ctx.child(step_input.clone());
					executor
						.execute_pattern(pattern, step_input, &child_ctx)
						.await
				}
				StepOperation::Agent(ac) => {
					// Agent-as-tool execution (WP13) - not yet implemented
					Err(ExecutionError::NotImplemented(format!(
						"agent-as-tool execution not yet implemented: agent={}, skill={:?}",
						ac.name, ac.skill
					)))
				}
			};

			// Log completion (always) and record in OTEL span (if sampled)
			if let Some(ref tracing_ctx) = ctx.tracing {
				match &result {
					Ok(output) => exec_tracing::log_step_complete(tracing_ctx, &step.id, start.elapsed(), Ok(output)),
					Err(e) => exec_tracing::log_step_complete(tracing_ctx, &step.id, start.elapsed(), Err(&e.to_string())),
				}
			}
			if let Some(ref mut s) = span {
				if let Some(ref tracing_ctx) = ctx.tracing {
					match &result {
						Ok(output) => exec_tracing::record_step_completion(s, tracing_ctx, start.elapsed(), Ok(output)),
						Err(e) => exec_tracing::record_step_completion(s, tracing_ctx, start.elapsed(), Err(&e.to_string())),
					}
				}
			}

			// End span explicitly
			if let Some(mut s) = span {
				use opentelemetry::trace::Span;
				s.end();
			}

			// Propagate error after recording
			let result = result?;

			// Store result for potential reference by later steps
			ctx.store_step_result(&step.id, result.clone()).await;
			current_result = result;
		}

		Ok(current_result)
	}

	/// Resolve a data binding to a value
	async fn resolve_binding(
		binding: &DataBinding,
		input: &Value,
		ctx: &ExecutionContext,
	) -> Result<Value, ExecutionError> {
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

		let jsonpath = JsonPath::parse(path)
			.map_err(|e| ExecutionError::JsonPathError(format!("{}: {}", path, e)))?;

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
	use crate::mcp::registry::CompiledRegistry;
	use crate::mcp::registry::executor::MockToolInvoker;
	use crate::mcp::registry::patterns::{InputBinding, PipelineStep, StepBinding, ToolCall};
	use crate::mcp::registry::types::Registry;
	use std::sync::Arc;

	fn setup_context_and_executor(
		invoker: MockToolInvoker,
	) -> (ExecutionContext, CompositionExecutor) {
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
					operation: StepOperation::Tool(ToolCall::new("step1_tool")),
					input: None,
				},
				PipelineStep {
					id: "s2".to_string(),
					operation: StepOperation::Tool(ToolCall::new("step2_tool")),
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
		let invoker =
			MockToolInvoker::new().with_response("search", serde_json::json!({"results": ["a", "b"]}));

		let (ctx, executor) = setup_context_and_executor(invoker);

		let spec = PipelineSpec {
			steps: vec![PipelineStep {
				id: "search".to_string(),
				operation: StepOperation::Tool(ToolCall::new("search")),
				input: Some(DataBinding::Input(InputBinding {
					path: "$.query".to_string(),
				})),
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
					operation: StepOperation::Tool(ToolCall::new("search")),
					input: None,
				},
				PipelineStep {
					id: "process".to_string(),
					operation: StepOperation::Tool(ToolCall::new("process")),
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

	/// A mock invoker that tracks call order and validates inputs.
	/// Used to verify DAG execution correctness and detect race conditions.
	struct OrderTrackingInvoker {
		responses: std::collections::HashMap<String, Value>,
		/// Records (tool_name, input) for each invocation in order
		call_log: std::sync::Arc<std::sync::Mutex<Vec<(String, Value)>>>,
	}

	impl OrderTrackingInvoker {
		fn new() -> Self {
			Self {
				responses: std::collections::HashMap::new(),
				call_log: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
			}
		}

		fn with_response(mut self, tool_name: &str, response: Value) -> Self {
			self.responses.insert(tool_name.to_string(), response);
			self
		}

		fn get_call_log(&self) -> Vec<(String, Value)> {
			self.call_log.lock().unwrap().clone()
		}

		fn get_call_order(&self) -> Vec<String> {
			self.call_log.lock().unwrap().iter().map(|(name, _)| name.clone()).collect()
		}
	}

	#[async_trait::async_trait]
	impl super::super::ToolInvoker for OrderTrackingInvoker {
		async fn invoke(&self, tool_name: &str, args: Value) -> Result<Value, super::super::ExecutionError> {
			// Record the call
			self.call_log.lock().unwrap().push((tool_name.to_string(), args.clone()));

			self.responses
				.get(tool_name)
				.cloned()
				.ok_or_else(|| super::super::ExecutionError::ToolNotFound(tool_name.to_string()))
		}
	}

	#[tokio::test]
	async fn test_dag_pattern_with_parallel_branches() {
		// This test demonstrates that the IR already supports DAG structures.
		// Steps "prefs" and "embed" both only depend on the original input,
		// so they COULD run in parallel. Step "search" depends on BOTH of them.
		//
		// DAG structure:
		//
		//     input
		//     /   \
		//    v     v
		// prefs   embed
		//    \     /
		//     v   v
		//    search
		//
		// Currently executes sequentially, but produces correct results.
		// A future DAG-aware executor could run prefs and embed in parallel.

		use crate::mcp::registry::patterns::ConstructBinding;
		use std::collections::HashMap;

		let invoker = MockToolInvoker::new()
			.with_response("get_user_prefs", serde_json::json!({
				"category_weights": {"tech": 2.0, "news": 1.0},
				"content_filter": "recent"
			}))
			.with_response("generate_embedding", serde_json::json!({
				"embedding": [0.1, 0.2, 0.3, 0.4]
			}))
			.with_response("vector_search", serde_json::json!({
				"results": [
					{"id": "doc1", "score": 0.95},
					{"id": "doc2", "score": 0.87}
				]
			}));

		let (ctx, executor) = setup_context_and_executor(invoker);

		let spec = PipelineSpec {
			steps: vec![
				// Branch 1: get user preferences (depends only on input)
				PipelineStep {
					id: "prefs".to_string(),
					operation: StepOperation::Tool(ToolCall::new("get_user_prefs")),
					input: Some(DataBinding::Input(InputBinding {
						path: "$.user_id".to_string(),
					})),
				},
				// Branch 2: generate embedding (depends only on input)
				PipelineStep {
					id: "embed".to_string(),
					operation: StepOperation::Tool(ToolCall::new("generate_embedding")),
					input: Some(DataBinding::Input(InputBinding {
						path: "$.query".to_string(),
					})),
				},
				// Join: vector search (depends on BOTH prefs and embed)
				PipelineStep {
					id: "search".to_string(),
					operation: StepOperation::Tool(ToolCall::new("vector_search")),
					input: Some(DataBinding::Construct(ConstructBinding {
						fields: HashMap::from([
							(
								"embedding".to_string(),
								DataBinding::Step(StepBinding {
									step_id: "embed".to_string(),
									path: "$.embedding".to_string(),
								}),
							),
							(
								"filter".to_string(),
								DataBinding::Step(StepBinding {
									step_id: "prefs".to_string(),
									path: "$.content_filter".to_string(),
								}),
							),
							(
								"boost".to_string(),
								DataBinding::Step(StepBinding {
									step_id: "prefs".to_string(),
									path: "$.category_weights".to_string(),
								}),
							),
						]),
					})),
				},
			],
		};

		let input = serde_json::json!({
			"user_id": "user_123",
			"query": "rust async programming"
		});

		let result = PipelineExecutor::execute(&spec, input, &ctx, &executor).await;

		assert!(result.is_ok(), "DAG execution should succeed");
		let output = result.unwrap();

		// Verify the final search results came through
		assert_eq!(output["results"][0]["id"], "doc1");
		assert_eq!(output["results"][0]["score"], 0.95);
		assert_eq!(output["results"][1]["id"], "doc2");

		// Verify intermediate results were stored and accessible
		let prefs_result = ctx.get_step_result("prefs").await;
		assert!(prefs_result.is_some(), "prefs step result should be stored");
		assert_eq!(prefs_result.unwrap()["content_filter"], "recent");

		let embed_result = ctx.get_step_result("embed").await;
		assert!(embed_result.is_some(), "embed step result should be stored");
		assert_eq!(embed_result.unwrap()["embedding"], serde_json::json!([0.1, 0.2, 0.3, 0.4]));
	}

	#[tokio::test]
	async fn test_complex_dag_with_multi_parent_joins_and_skip_wave_deps() {
		// This test creates a complex DAG that would expose race conditions
		// if parallelization is not done correctly.
		//
		// DAG structure:
		//
		//            input
		//          /   |   \
		//         v    v    v
		//         a    b    c           <- Wave 0: all depend only on input
		//         |\   |   /|
		//         | \  |  / |
		//         |  \ | /  |
		//         v   vvv   v
		//         d    e    f           <- Wave 1: d(a), e(a+b+c), f(c)
		//          \   |   /
		//           \  |  /
		//            v v v
		//              g                <- Wave 2: g(d+e+f)
		//              |
		//              v
		//              h                <- Wave 3: h(g + b) <- SKIP-WAVE dep on b!
		//
		// Race condition risks:
		// 1. e starts before a, b, c all complete
		// 2. g starts before d, e, f all complete
		// 3. h starts before g completes, or forgets b is still needed
		//
		// The skip-wave dependency (h -> b) is critical: b is from wave 0,
		// but h is in wave 3. A naive impl might not preserve b's result.

		use crate::mcp::registry::patterns::ConstructBinding;
		use crate::mcp::registry::CompiledRegistry;
		use crate::mcp::registry::types::Registry;
		use std::collections::HashMap;

		let invoker = OrderTrackingInvoker::new()
			// Wave 0 tools - each returns a unique marker
			.with_response("tool_a", serde_json::json!({"id": "a", "value": 10}))
			.with_response("tool_b", serde_json::json!({"id": "b", "value": 20, "extra": "b_data"}))
			.with_response("tool_c", serde_json::json!({"id": "c", "value": 30}))
			// Wave 1 tools
			.with_response("tool_d", serde_json::json!({"id": "d", "sum": 100}))
			.with_response("tool_e", serde_json::json!({"id": "e", "combined": 60}))
			.with_response("tool_f", serde_json::json!({"id": "f", "result": 300}))
			// Wave 2 tool
			.with_response("tool_g", serde_json::json!({"id": "g", "aggregated": 999}))
			// Wave 3 tool - final output
			.with_response("tool_h", serde_json::json!({"id": "h", "final": "complete"}));

		let invoker = std::sync::Arc::new(invoker);

		let registry = Registry::new();
		let compiled = std::sync::Arc::new(CompiledRegistry::compile(registry).unwrap());
		let ctx = ExecutionContext::new(serde_json::json!({}), compiled.clone(), invoker.clone());
		let executor = super::super::CompositionExecutor::new(compiled, invoker.clone());

		let spec = PipelineSpec {
			steps: vec![
				// === Wave 0: Independent steps, all depend only on input ===
				PipelineStep {
					id: "a".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_a")),
					input: Some(DataBinding::Input(InputBinding { path: "$.x".to_string() })),
				},
				PipelineStep {
					id: "b".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_b")),
					input: Some(DataBinding::Input(InputBinding { path: "$.y".to_string() })),
				},
				PipelineStep {
					id: "c".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_c")),
					input: Some(DataBinding::Input(InputBinding { path: "$.z".to_string() })),
				},

				// === Wave 1: Various dependency patterns ===
				// d depends only on a
				PipelineStep {
					id: "d".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_d")),
					input: Some(DataBinding::Step(StepBinding {
						step_id: "a".to_string(),
						path: "$.value".to_string(),
					})),
				},
				// e depends on a, b, AND c (multi-parent join)
				PipelineStep {
					id: "e".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_e")),
					input: Some(DataBinding::Construct(ConstructBinding {
						fields: HashMap::from([
							("from_a".to_string(), DataBinding::Step(StepBinding {
								step_id: "a".to_string(),
								path: "$.value".to_string(),
							})),
							("from_b".to_string(), DataBinding::Step(StepBinding {
								step_id: "b".to_string(),
								path: "$.value".to_string(),
							})),
							("from_c".to_string(), DataBinding::Step(StepBinding {
								step_id: "c".to_string(),
								path: "$.value".to_string(),
							})),
						]),
					})),
				},
				// f depends only on c
				PipelineStep {
					id: "f".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_f")),
					input: Some(DataBinding::Step(StepBinding {
						step_id: "c".to_string(),
						path: "$.value".to_string(),
					})),
				},

				// === Wave 2: Join of d, e, f ===
				PipelineStep {
					id: "g".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_g")),
					input: Some(DataBinding::Construct(ConstructBinding {
						fields: HashMap::from([
							("d_result".to_string(), DataBinding::Step(StepBinding {
								step_id: "d".to_string(),
								path: "$.sum".to_string(),
							})),
							("e_result".to_string(), DataBinding::Step(StepBinding {
								step_id: "e".to_string(),
								path: "$.combined".to_string(),
							})),
							("f_result".to_string(), DataBinding::Step(StepBinding {
								step_id: "f".to_string(),
								path: "$.result".to_string(),
							})),
						]),
					})),
				},

				// === Wave 3: Skip-wave dependency ===
				// h depends on g (wave 2) AND b (wave 0!) - tests that early results persist
				PipelineStep {
					id: "h".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_h")),
					input: Some(DataBinding::Construct(ConstructBinding {
						fields: HashMap::from([
							("g_aggregated".to_string(), DataBinding::Step(StepBinding {
								step_id: "g".to_string(),
								path: "$.aggregated".to_string(),
							})),
							// Critical: reference back to wave 0's step b
							("b_extra".to_string(), DataBinding::Step(StepBinding {
								step_id: "b".to_string(),
								path: "$.extra".to_string(),
							})),
						]),
					})),
				},
			],
		};

		let input = serde_json::json!({
			"x": "input_for_a",
			"y": "input_for_b",
			"z": "input_for_c"
		});

		let result = PipelineExecutor::execute(&spec, input, &ctx, &executor).await;

		// Verify execution succeeded
		assert!(result.is_ok(), "Complex DAG execution should succeed: {:?}", result.err());
		let output = result.unwrap();
		assert_eq!(output["id"], "h", "Final output should be from step h");
		assert_eq!(output["final"], "complete");

		// Verify call order respects dependencies
		let call_order = invoker.get_call_order();
		assert_eq!(call_order.len(), 8, "All 8 tools should be called");

		// Helper to find position of a tool in call order
		let pos = |tool: &str| -> usize {
			call_order.iter().position(|t| t == tool).expect(&format!("{} should be in call order", tool))
		};

		// Wave 0 tools must complete before their dependents
		// (In sequential execution, they'll be in order a, b, c, but that's fine)
		assert!(pos("tool_a") < pos("tool_d"), "a must complete before d");
		assert!(pos("tool_a") < pos("tool_e"), "a must complete before e");
		assert!(pos("tool_b") < pos("tool_e"), "b must complete before e");
		assert!(pos("tool_c") < pos("tool_e"), "c must complete before e");
		assert!(pos("tool_c") < pos("tool_f"), "c must complete before f");

		// Wave 1 tools must complete before g
		assert!(pos("tool_d") < pos("tool_g"), "d must complete before g");
		assert!(pos("tool_e") < pos("tool_g"), "e must complete before g");
		assert!(pos("tool_f") < pos("tool_g"), "f must complete before g");

		// g must complete before h (and b must still be accessible for h)
		assert!(pos("tool_g") < pos("tool_h"), "g must complete before h");
		assert!(pos("tool_b") < pos("tool_h"), "b must complete before h (skip-wave dep)");

		// Verify the inputs were correctly constructed for multi-dependency steps
		let call_log = invoker.get_call_log();

		// Find the call to tool_e and verify its input was constructed from a, b, c
		let e_call = call_log.iter().find(|(name, _)| name == "tool_e").expect("tool_e should be called");
		assert_eq!(e_call.1["from_a"], 10, "e should receive a's value");
		assert_eq!(e_call.1["from_b"], 20, "e should receive b's value");
		assert_eq!(e_call.1["from_c"], 30, "e should receive c's value");

		// Find the call to tool_g and verify its input was constructed from d, e, f
		let g_call = call_log.iter().find(|(name, _)| name == "tool_g").expect("tool_g should be called");
		assert_eq!(g_call.1["d_result"], 100, "g should receive d's sum");
		assert_eq!(g_call.1["e_result"], 60, "g should receive e's combined");
		assert_eq!(g_call.1["f_result"], 300, "g should receive f's result");

		// Find the call to tool_h and verify skip-wave dependency worked
		let h_call = call_log.iter().find(|(name, _)| name == "tool_h").expect("tool_h should be called");
		assert_eq!(h_call.1["g_aggregated"], 999, "h should receive g's aggregated");
		assert_eq!(h_call.1["b_extra"], "b_data", "h should receive b's extra (skip-wave dep)");

		// Verify all intermediate results are still accessible
		assert!(ctx.get_step_result("a").await.is_some(), "a result should persist");
		assert!(ctx.get_step_result("b").await.is_some(), "b result should persist");
		assert!(ctx.get_step_result("c").await.is_some(), "c result should persist");
		assert!(ctx.get_step_result("d").await.is_some(), "d result should persist");
		assert!(ctx.get_step_result("e").await.is_some(), "e result should persist");
		assert!(ctx.get_step_result("f").await.is_some(), "f result should persist");
		assert!(ctx.get_step_result("g").await.is_some(), "g result should persist");
		assert!(ctx.get_step_result("h").await.is_some(), "h result should persist");
	}
}
