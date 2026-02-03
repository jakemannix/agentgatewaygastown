// Scatter-Gather pattern executor

use std::time::{Duration, Instant};

use futures::future::join_all;
use opentelemetry::trace::Span;
use serde_json::Value;
use serde_json_path::JsonPath;
use tokio::time::timeout;

use super::context::ExecutionContext;
use super::composition_tracing as exec_tracing;
use super::{CompositionExecutor, ExecutionError};
use crate::mcp::registry::patterns::{AggregationOp, ScatterGatherSpec, ScatterTarget};

/// Executor for scatter-gather patterns
pub struct ScatterGatherExecutor;

impl ScatterGatherExecutor {
	/// Execute a scatter-gather pattern
	pub async fn execute(
		spec: &ScatterGatherSpec,
		input: Value,
		ctx: &ExecutionContext,
		executor: &CompositionExecutor,
	) -> Result<Value, ExecutionError> {
		// Create futures for all targets with tracing
		let futures: Vec<_> = spec
			.targets
			.iter()
			.enumerate()
			.map(|(idx, target)| Self::execute_target_with_tracing(target, idx, input.clone(), ctx, executor))
			.collect();

		// Execute with optional timeout
		let results = if let Some(timeout_ms) = spec.timeout_ms {
			let duration = Duration::from_millis(timeout_ms as u64);
			timeout(duration, join_all(futures))
				.await
				.map_err(|_| ExecutionError::Timeout(timeout_ms))?
		} else {
			join_all(futures).await
		};

		// Handle results based on fail_fast setting
		let (successes, failures): (Vec<_>, Vec<_>) = results.into_iter().partition(|r| r.is_ok());

		if spec.fail_fast && !failures.is_empty() {
			// Return first error
			return Err(failures.into_iter().next().unwrap().unwrap_err());
		}

		if successes.is_empty() {
			return Err(ExecutionError::AllTargetsFailed);
		}

		// Extract successful results
		let values: Vec<Value> = successes.into_iter().map(|r| r.unwrap()).collect();

		// Apply aggregation
		Self::aggregate(values, &spec.aggregation.ops)
	}

	/// Execute a single scatter target with tracing
	async fn execute_target_with_tracing(
		target: &ScatterTarget,
		index: usize,
		input: Value,
		ctx: &ExecutionContext,
		executor: &CompositionExecutor,
	) -> Result<Value, ExecutionError> {
		let target_name = match target {
			ScatterTarget::Tool(tool_ref) => {
				// Include server in name for tracing if present
				if let Some(ref server) = tool_ref.server {
					format!("{}@{}", tool_ref.tool, server)
				} else {
					tool_ref.tool.clone()
				}
			},
			ScatterTarget::Pattern(_) => format!("pattern_{}", index),
		};

		// Create span if tracing is enabled and sampled
		let mut span = ctx
			.tracing
			.as_ref()
			.filter(|t| t.sampled)
			.and_then(|t| exec_tracing::create_target_span(t, &target_name, index));

		let start = Instant::now();

		let result = Self::execute_target(target, input, ctx, executor).await;

		// Record completion in span
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
			s.end();
		}

		result
	}

	/// Execute a single scatter target
	async fn execute_target(
		target: &ScatterTarget,
		input: Value,
		ctx: &ExecutionContext,
		executor: &CompositionExecutor,
	) -> Result<Value, ExecutionError> {
		match target {
			ScatterTarget::Tool(tool_ref) => {
				// TODO: Pass server to execute_tool for multi-server routing
				// For now, use just the tool name - server routing will be handled
				// at a higher level when we add server-aware tool resolution
				executor
					.execute_tool_on_server(&tool_ref.tool, tool_ref.server.as_deref(), input, ctx)
					.await
			},
			ScatterTarget::Pattern(pattern) => {
				let child_ctx = ctx.child(input.clone());
				executor.execute_pattern(pattern, input, &child_ctx).await
			},
		}
	}

	/// Apply aggregation operations to results
	fn aggregate(mut values: Vec<Value>, ops: &[AggregationOp]) -> Result<Value, ExecutionError> {
		let mut result: Value = Value::Array(values.clone());

		for op in ops {
			result = match op {
				AggregationOp::Flatten(_) => Self::flatten(&result)?,
				AggregationOp::Sort(sort) => Self::sort(&result, &sort.field, &sort.order)?,
				AggregationOp::Dedupe(dedupe) => Self::dedupe(&result, &dedupe.field)?,
				AggregationOp::Limit(limit) => Self::limit(&result, limit.count as usize)?,
				AggregationOp::Concat(_) => result, // Already an array, no change
				AggregationOp::Merge(_) => Self::merge(&mut values)?,
				AggregationOp::Wrap(wrap) => Self::wrap(result, &wrap.field),
				AggregationOp::Extract(extract) => Self::extract(&result, &extract.path)?,
			};
		}

		Ok(result)
	}

	/// Wrap a value in an object with the specified field name
	/// Turns any value into {"field": value}
	fn wrap(value: Value, field: &str) -> Value {
		let mut obj = serde_json::Map::new();
		obj.insert(field.to_string(), value);
		Value::Object(obj)
	}

	/// Extract a field from each element using JSONPath
	/// Transforms [obj1, obj2, ...] into [obj1.path, obj2.path, ...]
	fn extract(value: &Value, path: &str) -> Result<Value, ExecutionError> {
		let arr = value.as_array().ok_or_else(|| ExecutionError::TypeError {
			expected: "array".to_string(),
			actual: value_type_name(value),
		})?;

		let jsonpath =
			JsonPath::parse(path).map_err(|e| ExecutionError::JsonPathError(format!("{}: {}", path, e)))?;

		let mut result = Vec::new();
		for item in arr {
			let query_result = jsonpath.query(item);
			// If path matches multiple values, include all; if none, include null
			let extracted: Vec<Value> = query_result.iter().map(|v| (*v).clone()).collect();
			if extracted.is_empty() {
				result.push(Value::Null);
			} else if extracted.len() == 1 {
				result.push(extracted.into_iter().next().unwrap());
			} else {
				result.push(Value::Array(extracted));
			}
		}

		Ok(Value::Array(result))
	}

	/// Flatten nested arrays
	fn flatten(value: &Value) -> Result<Value, ExecutionError> {
		let arr = value.as_array().ok_or_else(|| ExecutionError::TypeError {
			expected: "array".to_string(),
			actual: value_type_name(value),
		})?;

		let mut result = Vec::new();
		for item in arr {
			if let Some(inner_arr) = item.as_array() {
				result.extend(inner_arr.iter().cloned());
			} else {
				result.push(item.clone());
			}
		}

		Ok(Value::Array(result))
	}

	/// Sort array by field
	fn sort(value: &Value, field: &str, order: &str) -> Result<Value, ExecutionError> {
		let arr = value.as_array().ok_or_else(|| ExecutionError::TypeError {
			expected: "array".to_string(),
			actual: value_type_name(value),
		})?;

		let jsonpath = JsonPath::parse(field)
			.map_err(|e| ExecutionError::JsonPathError(format!("{}: {}", field, e)))?;

		let mut items: Vec<_> = arr.to_vec();

		items.sort_by(|a, b| {
			let a_query = jsonpath.query(a);
			let b_query = jsonpath.query(b);
			let a_val = a_query.iter().next().copied();
			let b_val = b_query.iter().next().copied();

			let cmp = compare_values(a_val, b_val);
			if order == "desc" { cmp.reverse() } else { cmp }
		});

		Ok(Value::Array(items))
	}

	/// Deduplicate by field
	fn dedupe(value: &Value, field: &str) -> Result<Value, ExecutionError> {
		let arr = value.as_array().ok_or_else(|| ExecutionError::TypeError {
			expected: "array".to_string(),
			actual: value_type_name(value),
		})?;

		let jsonpath = JsonPath::parse(field)
			.map_err(|e| ExecutionError::JsonPathError(format!("{}: {}", field, e)))?;

		let mut seen = std::collections::HashSet::new();
		let mut result = Vec::new();

		for item in arr {
			let key = jsonpath.query(item).iter().next().map(|v| v.to_string());
			if let Some(k) = key {
				if seen.insert(k) {
					result.push(item.clone());
				}
			} else {
				result.push(item.clone());
			}
		}

		Ok(Value::Array(result))
	}

	/// Limit to N items
	fn limit(value: &Value, count: usize) -> Result<Value, ExecutionError> {
		let arr = value.as_array().ok_or_else(|| ExecutionError::TypeError {
			expected: "array".to_string(),
			actual: value_type_name(value),
		})?;

		Ok(Value::Array(arr.iter().take(count).cloned().collect()))
	}

	/// Merge objects
	fn merge(values: &mut Vec<Value>) -> Result<Value, ExecutionError> {
		let mut result = serde_json::Map::new();

		for value in values {
			if let Some(obj) = value.as_object() {
				for (k, v) in obj {
					result.insert(k.clone(), v.clone());
				}
			}
		}

		Ok(Value::Object(result))
	}
}

/// Get type name for error messages
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

/// Compare two optional JSON values
fn compare_values(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
	match (a, b) {
		(None, None) => std::cmp::Ordering::Equal,
		(None, Some(_)) => std::cmp::Ordering::Less,
		(Some(_), None) => std::cmp::Ordering::Greater,
		(Some(a), Some(b)) => {
			// Try numeric comparison first
			if let (Some(a_num), Some(b_num)) = (a.as_f64(), b.as_f64()) {
				return a_num
					.partial_cmp(&b_num)
					.unwrap_or(std::cmp::Ordering::Equal);
			}
			// Fall back to string comparison
			a.to_string().cmp(&b.to_string())
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mcp::registry::CompiledRegistry;
	use crate::mcp::registry::executor::MockToolInvoker;
	use crate::mcp::registry::patterns::{AggregationStrategy, DedupeOp, LimitOp, SortOp, ToolRef};
	use crate::mcp::registry::types::Registry;
	use serde_json::json;
	use std::sync::Arc;

	fn setup_context_and_executor(
		invoker: MockToolInvoker,
	) -> (ExecutionContext, CompositionExecutor) {
		let registry = Registry::new();
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(invoker);

		let ctx = ExecutionContext::new(json!({}), compiled.clone(), invoker.clone());
		let executor = CompositionExecutor::new(compiled, invoker);

		(ctx, executor)
	}

	#[tokio::test]
	async fn test_scatter_gather_basic() {
		let invoker = MockToolInvoker::new()
			.with_response("search_a", json!({"source": "a", "results": [1, 2]}))
			.with_response("search_b", json!({"source": "b", "results": [3, 4]}));

		let (ctx, executor) = setup_context_and_executor(invoker);

		let spec = ScatterGatherSpec {
			targets: vec![
				ScatterTarget::Tool(ToolRef::new("search_a")),
				ScatterTarget::Tool(ToolRef::new("search_b")),
			],
			aggregation: AggregationStrategy { ops: vec![] },
			timeout_ms: None,
			fail_fast: false,
		};

		let result = ScatterGatherExecutor::execute(&spec, json!({}), &ctx, &executor).await;

		assert!(result.is_ok());
		let arr = result.unwrap();
		assert!(arr.is_array());
		assert_eq!(arr.as_array().unwrap().len(), 2);
	}

	#[tokio::test]
	async fn test_flatten() {
		let value = json!([[1, 2], [3, 4], [5]]);
		let result = ScatterGatherExecutor::flatten(&value).unwrap();

		assert_eq!(result, json!([1, 2, 3, 4, 5]));
	}

	#[tokio::test]
	async fn test_sort_asc() {
		let value = json!([
			{"name": "c", "score": 3},
			{"name": "a", "score": 1},
			{"name": "b", "score": 2}
		]);

		let result = ScatterGatherExecutor::sort(&value, "$.score", "asc").unwrap();
		let arr = result.as_array().unwrap();

		assert_eq!(arr[0]["name"], "a");
		assert_eq!(arr[1]["name"], "b");
		assert_eq!(arr[2]["name"], "c");
	}

	#[tokio::test]
	async fn test_sort_desc() {
		let value = json!([
			{"name": "a", "score": 1},
			{"name": "c", "score": 3},
			{"name": "b", "score": 2}
		]);

		let result = ScatterGatherExecutor::sort(&value, "$.score", "desc").unwrap();
		let arr = result.as_array().unwrap();

		assert_eq!(arr[0]["name"], "c");
		assert_eq!(arr[1]["name"], "b");
		assert_eq!(arr[2]["name"], "a");
	}

	#[tokio::test]
	async fn test_dedupe() {
		let value = json!([
			{"id": 1, "name": "a"},
			{"id": 2, "name": "b"},
			{"id": 1, "name": "a-dup"}
		]);

		let result = ScatterGatherExecutor::dedupe(&value, "$.id").unwrap();
		let arr = result.as_array().unwrap();

		assert_eq!(arr.len(), 2);
	}

	#[tokio::test]
	async fn test_limit() {
		let value = json!([1, 2, 3, 4, 5]);
		let result = ScatterGatherExecutor::limit(&value, 3).unwrap();

		assert_eq!(result, json!([1, 2, 3]));
	}

	#[tokio::test]
	async fn test_aggregate_chain() {
		let values = vec![
			json!([{"score": 3}, {"score": 1}]),
			json!([{"score": 2}, {"score": 1}]),
		];

		let ops = vec![
			AggregationOp::Flatten(true),
			AggregationOp::Dedupe(DedupeOp {
				field: "$.score".to_string(),
			}),
			AggregationOp::Sort(SortOp {
				field: "$.score".to_string(),
				order: "desc".to_string(),
			}),
			AggregationOp::Limit(LimitOp { count: 2 }),
		];

		let result = ScatterGatherExecutor::aggregate(values, &ops).unwrap();
		let arr = result.as_array().unwrap();

		assert_eq!(arr.len(), 2);
		assert_eq!(arr[0]["score"], 3);
		assert_eq!(arr[1]["score"], 2);
	}

	#[tokio::test]
	async fn test_merge() {
		let mut values = vec![json!({"a": 1}), json!({"b": 2}), json!({"c": 3})];

		let result = ScatterGatherExecutor::merge(&mut values).unwrap();

		assert_eq!(result["a"], 1);
		assert_eq!(result["b"], 2);
		assert_eq!(result["c"], 3);
	}

	// =============================================================================
	// MCP Compliance Tests - structuredContent must match outputSchema
	// =============================================================================
	//
	// Per MCP spec:
	// - Tools MUST return `content` (text representation)
	// - Tools MAY return `structuredContent` (JSON matching outputSchema)
	// - If a tool has `outputSchema`, `structuredContent` MUST match that schema
	//
	// Current behavior: scatter-gather returns flat array [item1, item2, ...]
	// Expected behavior: if outputSchema is {"results": [...]}, result should be wrapped
	// =============================================================================

	/// Test that scatter-gather with wrap aggregation op produces wrapped output
	///
	/// Given:
	/// - Two backend tools each returning `{"results": [item1, item2]}`
	/// - Aggregation: extract($.results) -> flatten -> wrap("results")
	///
	/// Expected:
	/// - Result is `{"results": [item1, item2, item3, item4]}` (wrapped in object)
	///
	/// This tests the new `wrap` aggregation operation that wraps an array
	/// in an object with a specified field name, enabling MCP outputSchema compliance.
	#[tokio::test]
	async fn test_scatter_gather_wrap_aggregation_for_output_schema() {
		// Setup mock backends that return normalized search results
		let invoker = MockToolInvoker::new()
			.with_response(
				"normalized_github",
				json!({"results": [
					{"title": "repo1", "url": "https://github.com/a/1", "source": "github"},
					{"title": "repo2", "url": "https://github.com/a/2", "source": "github"}
				]}),
			)
			.with_response(
				"normalized_huggingface",
				json!({"results": [
					{"title": "model1", "url": "https://huggingface.co/m/1", "source": "huggingface"},
					{"title": "model2", "url": "https://huggingface.co/m/2", "source": "huggingface"}
				]}),
			);

		let (ctx, executor) = setup_context_and_executor(invoker);

		// Scatter-gather with wrap aggregation to match outputSchema
		let spec = ScatterGatherSpec {
			targets: vec![
				ScatterTarget::Tool(ToolRef::new("normalized_github")),
				ScatterTarget::Tool(ToolRef::new("normalized_huggingface")),
			],
			aggregation: AggregationStrategy {
				ops: vec![
					// Extract the results array from each response
					AggregationOp::Extract(crate::mcp::registry::patterns::ExtractOp {
						path: "$.results".to_string(),
					}),
					// Flatten into single array
					AggregationOp::Flatten(true),
					// Wrap the array in an object to match outputSchema
					AggregationOp::Wrap(crate::mcp::registry::patterns::WrapOp {
						field: "results".to_string(),
					}),
				],
			},
			timeout_ms: None,
			fail_fast: false,
		};

		let result = ScatterGatherExecutor::execute(&spec, json!({"query": "test"}), &ctx, &executor)
			.await
			.expect("scatter-gather should succeed");

		// Result should be wrapped in {"results": [...]} to match outputSchema
		assert!(
			result.is_object(),
			"Result should be an object, not array. Got: {:?}",
			result
		);

		let results_array = result
			.get("results")
			.expect("Result should have 'results' field");

		assert!(
			results_array.is_array(),
			"results field should be an array"
		);

		let arr = results_array.as_array().unwrap();
		assert_eq!(
			arr.len(),
			4,
			"Should have 4 items (2 from each source)"
		);

		// Verify items from both sources are present
		let sources: Vec<&str> = arr
			.iter()
			.filter_map(|item| item.get("source").and_then(|s| s.as_str()))
			.collect();

		assert!(
			sources.contains(&"github"),
			"Should contain github results"
		);
		assert!(
			sources.contains(&"huggingface"),
			"Should contain huggingface results"
		);
	}

	/// Test that without wrap, scatter-gather returns flat array (current behavior)
	///
	/// This documents the current behavior that doesn't match outputSchema.
	/// When outputSchema expects {"results": [...]}, the flat array breaks MCP compliance.
	#[tokio::test]
	async fn test_scatter_gather_without_wrap_returns_flat_array() {
		let invoker = MockToolInvoker::new()
			.with_response(
				"search_a",
				json!({"results": [{"id": 1}, {"id": 2}]}),
			)
			.with_response(
				"search_b",
				json!({"results": [{"id": 3}, {"id": 4}]}),
			);

		let (ctx, executor) = setup_context_and_executor(invoker);

		// Scatter-gather WITHOUT wrap - current behavior
		let spec = ScatterGatherSpec {
			targets: vec![
				ScatterTarget::Tool(ToolRef::new("search_a")),
				ScatterTarget::Tool(ToolRef::new("search_b")),
			],
			aggregation: AggregationStrategy {
				ops: vec![
					AggregationOp::Extract(crate::mcp::registry::patterns::ExtractOp {
						path: "$.results".to_string(),
					}),
					AggregationOp::Flatten(true),
					// NOTE: No wrap op - this is the problem case
				],
			},
			timeout_ms: None,
			fail_fast: false,
		};

		let result = ScatterGatherExecutor::execute(&spec, json!({}), &ctx, &executor)
			.await
			.expect("scatter-gather should succeed");

		// Current behavior: returns flat array, NOT wrapped object
		// This breaks MCP compliance when outputSchema expects {"results": [...]}
		assert!(
			result.is_array(),
			"Without wrap, result is flat array (breaks outputSchema compliance)"
		);
		assert_eq!(
			result.as_array().unwrap().len(),
			4,
			"Should have 4 items total"
		);
	}
}
