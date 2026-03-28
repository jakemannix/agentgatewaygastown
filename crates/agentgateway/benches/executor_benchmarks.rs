// Executor hot-path benchmarks
//
// Measures the clone-heavy paths in each pattern executor.
// Run BEFORE and AFTER refactoring to quantify improvements.
//
// Usage:
//   cargo bench -p agentgateway -F internal_benches --bench executor_benchmarks
//
// For specific benchmarks:
//   cargo bench -p agentgateway -F internal_benches --bench executor_benchmarks -- pipeline
//   cargo bench -p agentgateway -F internal_benches --bench executor_benchmarks -- scatter
//   cargo bench -p agentgateway -F internal_benches --bench executor_benchmarks -- schema_map
//   cargo bench -p agentgateway -F internal_benches --bench executor_benchmarks -- filter
//   cargo bench -p agentgateway -F internal_benches --bench executor_benchmarks -- map_each

fn main() {
	#[cfg(all(not(test), not(feature = "internal_benches")))]
	panic!("benches must have -F internal_benches");
	use agentgateway as _;
	divan::main();
}

#[cfg(feature = "internal_benches")]
mod executor_benches {
	use agentgateway::mcp::registry::{
		AggregationOp, AggregationStrategy, CompiledRegistry, DedupeOp, ExtractOp, FieldPredicate,
		FieldSource, FilterSpec, LimitOp, LiteralValue, MapEachInner, MapEachSpec, PatternSpec,
		PipelineSpec, PipelineStep, PredicateValue, Registry, ScatterGatherSpec, ScatterTarget,
		SchemaMapSpec, SortOp, StepBinding, StepOperation, ToolCall, ToolDefinition, ToolRef, WrapOp,
	};
	use agentgateway::mcp::registry::executor::{
		CompositionExecutor, ExecutionContext, ExecutionError, FilterExecutor, MapEachExecutor,
		PipelineExecutor, ScatterGatherExecutor, SchemaMapExecutor, ToolInvoker,
	};
	use agentgateway::mcp::registry::patterns::DataBinding;
	use divan::{Bencher, black_box};
	use serde_json::{Value, json};
	use std::collections::HashMap;
	use std::sync::Arc;

	// =========================================================================
	// Payload generators
	// =========================================================================

	/// Generate a flat JSON object with N string fields (~40 bytes per field)
	fn flat_payload(n: usize) -> Value {
		let mut m = serde_json::Map::with_capacity(n);
		for i in 0..n {
			m.insert(format!("field_{}", i), Value::String(format!("value_{:032}", i)));
		}
		Value::Object(m)
	}

	/// Generate an array of N objects, each with a few fields (~100 bytes per item)
	fn array_payload(n: usize) -> Value {
		let items: Vec<Value> = (0..n)
			.map(|i| {
				json!({
					"id": i,
					"title": format!("Item {} title with some realistic length text", i),
					"url": format!("https://example.com/items/{}", i),
					"score": (i as f64) * 0.1,
					"source": if i % 2 == 0 { "github" } else { "huggingface" }
				})
			})
			.collect();
		Value::Array(items)
	}

	/// Generate nested search results (like a backend response)
	fn search_response(n: usize) -> Value {
		json!({
			"query": "test query",
			"total": n,
			"results": array_payload(n)
		})
	}

	// =========================================================================
	// Mock invoker for benchmarks
	// =========================================================================

	/// A mock invoker that returns configurable responses.
	/// Unlike FastMockInvoker (which echoes args), this returns realistic
	/// backend payloads to stress the clone/aggregation paths.
	struct BenchInvoker {
		responses: HashMap<String, Value>,
	}

	impl BenchInvoker {
		fn new() -> Self {
			Self {
				responses: HashMap::new(),
			}
		}

		fn with_response(mut self, tool: &str, response: Value) -> Self {
			self.responses.insert(tool.to_string(), response);
			self
		}

		/// Register N tools that each return a search response with `items_per` results
		fn with_search_tools(mut self, count: usize, items_per: usize) -> Self {
			for i in 0..count {
				self.responses
					.insert(format!("search_{}", i), search_response(items_per));
			}
			self
		}

		/// Register a tool that echoes its input (for pipeline pass-through benchmarks)
		fn with_echo(mut self, tool: &str) -> Self {
			// We can't actually echo at bench time, so return a fixed response.
			// The key cost we're measuring is the clone of the *input*, not the response.
			self.responses.insert(
				tool.to_string(),
				json!({"echoed": true, "status": "ok"}),
			);
			self
		}
	}

	#[async_trait::async_trait]
	impl ToolInvoker for BenchInvoker {
		async fn invoke(&self, tool_name: &str, _args: Value) -> Result<Value, ExecutionError> {
			self.responses
				.get(tool_name)
				.cloned()
				.ok_or_else(|| ExecutionError::ToolNotFound(tool_name.to_string()))
		}
	}

	fn setup(invoker: BenchInvoker) -> (ExecutionContext, CompositionExecutor) {
		let registry = Registry::new();
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(invoker);
		let ctx = ExecutionContext::new(json!({}), compiled.clone(), invoker.clone());
		let executor = CompositionExecutor::new(compiled, invoker);
		(ctx, executor)
	}

	// =========================================================================
	// Pipeline benchmarks
	// =========================================================================

	/// Pipeline with N sequential steps, each cloning input as current_result.
	/// Measures: input.clone() (line 24), current_result.clone() (line 32),
	/// result.clone() for step_results store (line 99).
	#[divan::bench(args = [1, 3, 5, 10])]
	fn pipeline_sequential_steps(bencher: Bencher, step_count: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let mut invoker = BenchInvoker::new();
		for i in 0..step_count {
			invoker = invoker.with_echo(&format!("step_{}", i));
		}
		let (ctx, executor) = setup(invoker);

		let spec = PipelineSpec {
			steps: (0..step_count)
				.map(|i| PipelineStep {
					id: format!("s{}", i),
					operation: StepOperation::Tool(ToolCall::new(format!("step_{}", i))),
					input: None, // Each step uses previous result (triggers current_result.clone())
				})
				.collect(),
		};

		let input = flat_payload(50);

		bencher.bench_local(|| {
			rt.block_on(async {
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	/// Pipeline with step bindings (triggers resolve_binding + step_result clones).
	/// The "DAG join" pattern from the audit.
	#[divan::bench(args = [10, 50, 200])]
	fn pipeline_with_step_bindings(bencher: Bencher, payload_fields: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let invoker = BenchInvoker::new()
			.with_echo("tool_a")
			.with_echo("tool_b")
			.with_echo("tool_join");
		let (ctx, executor) = setup(invoker);

		let spec = PipelineSpec {
			steps: vec![
				PipelineStep {
					id: "a".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_a")),
					input: None,
				},
				PipelineStep {
					id: "b".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_b")),
					input: None,
				},
				PipelineStep {
					id: "join".to_string(),
					operation: StepOperation::Tool(ToolCall::new("tool_join")),
					input: Some(DataBinding::Step(StepBinding {
						step_id: "a".to_string(),
						path: "$".to_string(), // Root path triggers value.clone()
					})),
				},
			],
		};

		let input = flat_payload(payload_fields);

		bencher.bench_local(|| {
			rt.block_on(async {
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	/// Measures just the payload clone cost (baseline for comparison).
	#[divan::bench(args = [1, 10, 50, 200, 1000])]
	fn baseline_value_clone(bencher: Bencher, fields: usize) {
		let payload = flat_payload(fields);
		bencher.bench_local(|| black_box(payload.clone()));
	}

	/// Measures clone cost for array payloads (used by filter/map_each/scatter).
	#[divan::bench(args = [10, 50, 200, 1000])]
	fn baseline_array_clone(bencher: Bencher, items: usize) {
		let payload = array_payload(items);
		bencher.bench_local(|| black_box(payload.clone()));
	}

	// =========================================================================
	// Scatter-Gather benchmarks
	// =========================================================================

	/// Scatter-gather with N targets, measuring input.clone() per target (line 32)
	/// and values.clone() in aggregate (line 138).
	#[divan::bench(args = [2, 4, 8, 16])]
	fn scatter_gather_fan_out(bencher: Bencher, target_count: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let invoker = BenchInvoker::new().with_search_tools(target_count, 10);
		let (ctx, executor) = setup(invoker);

		let spec = ScatterGatherSpec {
			targets: (0..target_count)
				.map(|i| ScatterTarget::Tool(ToolRef::new(format!("search_{}", i))))
				.collect(),
			aggregation: AggregationStrategy { ops: vec![] },
			timeout_ms: None,
			fail_fast: false,
		};

		let input = json!({"query": "test", "num_results": 10});

		bencher.bench_local(|| {
			rt.block_on(async {
				ScatterGatherExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	/// Scatter-gather varying payload size with fixed 4 targets.
	/// Shows how clone cost scales with payload size.
	#[divan::bench(args = [10, 50, 200, 1000])]
	fn scatter_gather_payload_scaling(bencher: Bencher, payload_fields: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let invoker = BenchInvoker::new().with_search_tools(4, 5);
		let (ctx, executor) = setup(invoker);

		let spec = ScatterGatherSpec {
			targets: (0..4)
				.map(|i| ScatterTarget::Tool(ToolRef::new(format!("search_{}", i))))
				.collect(),
			aggregation: AggregationStrategy { ops: vec![] },
			timeout_ms: None,
			fail_fast: false,
		};

		let input = flat_payload(payload_fields);

		bencher.bench_local(|| {
			rt.block_on(async {
				ScatterGatherExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	/// Scatter-gather with full aggregation pipeline: extract + flatten + dedupe + sort + limit + wrap.
	/// This is the realistic "multi-source search" pattern from the research-assistant demo.
	#[divan::bench(args = [5, 20, 50])]
	fn scatter_gather_full_aggregation(bencher: Bencher, items_per_source: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let invoker = BenchInvoker::new().with_search_tools(4, items_per_source);
		let (ctx, executor) = setup(invoker);

		let spec = ScatterGatherSpec {
			targets: (0..4)
				.map(|i| ScatterTarget::Tool(ToolRef::new(format!("search_{}", i))))
				.collect(),
			aggregation: AggregationStrategy {
				ops: vec![
					AggregationOp::Extract(ExtractOp {
						path: "$.results".to_string(),
					}),
					AggregationOp::Flatten(true),
					AggregationOp::Dedupe(DedupeOp {
						field: "$.url".to_string(),
					}),
					AggregationOp::Sort(SortOp {
						field: "$.score".to_string(),
						order: "desc".to_string(),
					}),
					AggregationOp::Limit(LimitOp { count: 20 }),
					AggregationOp::Wrap(WrapOp {
						field: "results".to_string(),
					}),
				],
			},
			timeout_ms: None,
			fail_fast: false,
		};

		let input = json!({"query": "test"});

		bencher.bench_local(|| {
			rt.block_on(async {
				ScatterGatherExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	// =========================================================================
	// Schema-Map benchmarks
	// =========================================================================

	/// Schema-map with N field mappings (all Path sources).
	/// Measures JSONPath extraction + clone costs.
	#[divan::bench(args = [3, 10, 25])]
	fn schema_map_path_fields(bencher: Bencher, field_count: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();

		let mut mappings = HashMap::new();
		for i in 0..field_count {
			mappings.insert(
				format!("out_{}", i),
				FieldSource::Path(format!("$.field_{}", i)),
			);
		}
		let spec = SchemaMapSpec { mappings };
		let input = flat_payload(field_count + 5); // A few extra fields to extract from

		bencher.bench_local(|| {
			rt.block_on(async { SchemaMapExecutor::execute(&spec, black_box(input.clone())).await })
		});
	}

	/// Schema-map with mixed field sources: Path, Literal, Coalesce, Template.
	#[divan::bench]
	fn schema_map_mixed_sources(bencher: Bencher) {
		let rt = tokio::runtime::Runtime::new().unwrap();

		let spec = SchemaMapSpec::new(HashMap::from([
			(
				"title".to_string(),
				FieldSource::Path("$.name".to_string()),
			),
			(
				"url".to_string(),
				FieldSource::Path("$.html_url".to_string()),
			),
			(
				"source".to_string(),
				FieldSource::Literal(LiteralValue::StringValue("github".to_string())),
			),
			(
				"source_type".to_string(),
				FieldSource::Literal(LiteralValue::StringValue("repo".to_string())),
			),
			(
				"snippet".to_string(),
				FieldSource::Path("$.description".to_string()),
			),
		]));

		let input = json!({
			"name": "agentgateway",
			"html_url": "https://github.com/agentgateway/agentgateway",
			"description": "Open source data plane for agentic AI connectivity",
			"stargazers_count": 1500,
			"language": "Rust"
		});

		bencher.bench_local(|| {
			rt.block_on(async { SchemaMapExecutor::execute(&spec, black_box(input.clone())).await })
		});
	}

	/// Schema-map with ArrayMap — the array transform pattern from normalized search.
	/// Exercises the now_or_never() path and per-element clone.
	#[divan::bench(args = [5, 20, 100])]
	fn schema_map_array_map(bencher: Bencher, array_size: usize) {
		use agentgateway::mcp::registry::patterns::ArrayMapSource;
		let rt = tokio::runtime::Runtime::new().unwrap();

		let spec = SchemaMapSpec::new(HashMap::from([(
			"results".to_string(),
			FieldSource::ArrayMap(ArrayMapSource {
				over: "$.items".to_string(),
				each: HashMap::from([
					(
						"title".to_string(),
						FieldSource::Path("$.name".to_string()),
					),
					(
						"url".to_string(),
						FieldSource::Path("$.link".to_string()),
					),
					(
						"source".to_string(),
						FieldSource::Literal(LiteralValue::StringValue("test".to_string())),
					),
				]),
			}),
		)]));

		let items: Vec<Value> = (0..array_size)
			.map(|i| {
				json!({
					"name": format!("Item {}", i),
					"link": format!("https://example.com/{}", i),
					"extra": "ignored field"
				})
			})
			.collect();
		let input = json!({ "items": items });

		bencher.bench_local(|| {
			rt.block_on(async { SchemaMapExecutor::execute(&spec, black_box(input.clone())).await })
		});
	}

	// =========================================================================
	// Filter benchmarks
	// =========================================================================

	/// Filter an array of N items, keeping ~50%.
	/// Measures item.clone() for passing items (line 30).
	#[divan::bench(args = [10, 50, 200, 1000])]
	fn filter_half_pass(bencher: Bencher, array_size: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();

		let spec = FilterSpec {
			predicate: FieldPredicate {
				field: "$.score".to_string(),
				op: "gte".to_string(),
				value: PredicateValue::NumberValue(0.5),
			},
		};

		// Generate array where ~50% have score >= 0.5
		let items: Vec<Value> = (0..array_size)
			.map(|i| {
				json!({
					"id": i,
					"title": format!("Document {} with some realistic content", i),
					"score": (i as f64) / (array_size as f64),
					"url": format!("https://example.com/doc/{}", i)
				})
			})
			.collect();
		let input = Value::Array(items);

		bencher.bench_local(|| {
			rt.block_on(async { FilterExecutor::execute(&spec, black_box(input.clone())).await })
		});
	}

	/// Filter keeping all items (worst case for cloning).
	#[divan::bench(args = [10, 50, 200, 1000])]
	fn filter_all_pass(bencher: Bencher, array_size: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();

		let spec = FilterSpec {
			predicate: FieldPredicate {
				field: "$.score".to_string(),
				op: "gte".to_string(),
				value: PredicateValue::NumberValue(0.0),
			},
		};

		let input = array_payload(array_size);

		bencher.bench_local(|| {
			rt.block_on(async { FilterExecutor::execute(&spec, black_box(input.clone())).await })
		});
	}

	/// Filter keeping no items (best case — no cloning of items).
	#[divan::bench(args = [10, 50, 200, 1000])]
	fn filter_none_pass(bencher: Bencher, array_size: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();

		let spec = FilterSpec {
			predicate: FieldPredicate {
				field: "$.score".to_string(),
				op: "gt".to_string(),
				value: PredicateValue::NumberValue(999.0),
			},
		};

		let input = array_payload(array_size);

		bencher.bench_local(|| {
			rt.block_on(async { FilterExecutor::execute(&spec, black_box(input.clone())).await })
		});
	}

	// =========================================================================
	// Map-Each benchmarks
	// =========================================================================

	/// Map-each with tool inner — clones each array item.
	#[divan::bench(args = [5, 20, 100])]
	fn map_each_tool(bencher: Bencher, array_size: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let invoker = BenchInvoker::new().with_echo("transform");
		let (ctx, executor) = setup(invoker);

		let spec = MapEachSpec {
			inner: MapEachInner::Tool("transform".to_string()),
		};

		let input = array_payload(array_size);

		bencher.bench_local(|| {
			rt.block_on(async {
				MapEachExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	/// Map-each with schema-map inner pattern — clones item twice (child ctx + pattern arg).
	#[divan::bench(args = [5, 20, 100])]
	fn map_each_schema_map_pattern(bencher: Bencher, array_size: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let invoker = BenchInvoker::new();
		let (ctx, executor) = setup(invoker);

		let inner_pattern = PatternSpec::SchemaMap(SchemaMapSpec::new(HashMap::from([
			(
				"name".to_string(),
				FieldSource::Path("$.title".to_string()),
			),
			(
				"link".to_string(),
				FieldSource::Path("$.url".to_string()),
			),
			(
				"source".to_string(),
				FieldSource::Literal(LiteralValue::StringValue("test".to_string())),
			),
		])));

		let spec = MapEachSpec {
			inner: MapEachInner::Pattern(Box::new(inner_pattern)),
		};

		let input = array_payload(array_size);

		bencher.bench_local(|| {
			rt.block_on(async {
				MapEachExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	// =========================================================================
	// End-to-end composition benchmarks
	// =========================================================================

	/// Full composition: scatter-gather -> pipeline with schema-map.
	/// This exercises the complete hot path as a real gateway request would.
	#[divan::bench]
	fn e2e_multi_source_search(bencher: Bencher) {
		let rt = tokio::runtime::Runtime::new().unwrap();

		// Build a composition tool in the registry
		let scatter_spec = PatternSpec::ScatterGather(ScatterGatherSpec {
			targets: (0..4)
				.map(|i| ScatterTarget::Tool(ToolRef::new(format!("search_{}", i))))
				.collect(),
			aggregation: AggregationStrategy {
				ops: vec![
					AggregationOp::Extract(ExtractOp {
						path: "$.results".to_string(),
					}),
					AggregationOp::Flatten(true),
					AggregationOp::Dedupe(DedupeOp {
						field: "$.url".to_string(),
					}),
					AggregationOp::Sort(SortOp {
						field: "$.score".to_string(),
						order: "desc".to_string(),
					}),
					AggregationOp::Limit(LimitOp { count: 20 }),
					AggregationOp::Wrap(WrapOp {
						field: "results".to_string(),
					}),
				],
			},
			timeout_ms: None,
			fail_fast: false,
		});

		let composition = ToolDefinition::composition("multi_search", scatter_spec);
		let registry = Registry::with_tool_definitions(vec![composition]);
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(BenchInvoker::new().with_search_tools(4, 15));
		let executor = CompositionExecutor::new(compiled, invoker);

		let input = json!({"query": "rust async programming", "num_results": 15});

		bencher.bench_local(|| {
			rt.block_on(async {
				executor
					.execute("multi_search", black_box(input.clone()))
					.await
			})
		});
	}

	/// Full 3-step pipeline composition exercising step_results store/retrieve.
	#[divan::bench(args = [10, 50, 200])]
	fn e2e_pipeline_with_data_flow(bencher: Bencher, payload_fields: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();

		let pipeline_spec = PatternSpec::Pipeline(PipelineSpec {
			steps: vec![
				PipelineStep {
					id: "fetch".to_string(),
					operation: StepOperation::Tool(ToolCall::new("fetch_tool")),
					input: None,
				},
				PipelineStep {
					id: "process".to_string(),
					operation: StepOperation::Tool(ToolCall::new("process_tool")),
					input: Some(DataBinding::Step(StepBinding {
						step_id: "fetch".to_string(),
						path: "$".to_string(),
					})),
				},
				PipelineStep {
					id: "format".to_string(),
					operation: StepOperation::Tool(ToolCall::new("format_tool")),
					input: Some(DataBinding::Step(StepBinding {
						step_id: "process".to_string(),
						path: "$".to_string(),
					})),
				},
			],
		});

		let composition = ToolDefinition::composition("data_pipeline", pipeline_spec);
		let registry = Registry::with_tool_definitions(vec![composition]);
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(
			BenchInvoker::new()
				.with_echo("fetch_tool")
				.with_echo("process_tool")
				.with_echo("format_tool"),
		);
		let executor = CompositionExecutor::new(compiled, invoker);

		let input = flat_payload(payload_fields);

		bencher.bench_local(|| {
			rt.block_on(async {
				executor
					.execute("data_pipeline", black_box(input.clone()))
					.await
			})
		});
	}

	// =========================================================================
	// Aggregation-only benchmarks (isolate clone cost in aggregate ops)
	// =========================================================================

	/// Benchmark the aggregate() function directly via scatter-gather.
	/// Uses a pre-built result set to focus on aggregation overhead.
	#[divan::bench(args = [10, 50, 200])]
	fn aggregation_extract_flatten_dedupe(bencher: Bencher, items_per_source: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();

		// Each "source" returns results wrapped in an object
		let invoker = BenchInvoker::new().with_search_tools(4, items_per_source);
		let (ctx, executor) = setup(invoker);

		let spec = ScatterGatherSpec {
			targets: (0..4)
				.map(|i| ScatterTarget::Tool(ToolRef::new(format!("search_{}", i))))
				.collect(),
			aggregation: AggregationStrategy {
				ops: vec![
					AggregationOp::Extract(ExtractOp {
						path: "$.results".to_string(),
					}),
					AggregationOp::Flatten(true),
					AggregationOp::Dedupe(DedupeOp {
						field: "$.url".to_string(),
					}),
				],
			},
			timeout_ms: None,
			fail_fast: false,
		};

		let input = json!({"query": "test"});

		bencher.bench_local(|| {
			rt.block_on(async {
				ScatterGatherExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}
}
