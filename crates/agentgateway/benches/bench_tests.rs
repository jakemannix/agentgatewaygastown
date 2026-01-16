fn main() {
	#[cfg(all(not(test), not(feature = "internal_benches")))]
	panic!("benches must have -F internal_benches");
	use agentgateway as _;
	divan::main();
}

#[cfg(feature = "internal_benches")]
mod composition_benchmarks {
	use agentgateway::mcp::registry::{
		AggregationOp, AggregationStrategy, CompiledRegistry, PatternSpec, PipelineSpec, PipelineStep,
		Registry, ScatterGatherSpec, ScatterTarget, StepOperation, ToolCall, ToolDefinition,
		VirtualToolDef,
	};
	use divan::{Bencher, black_box};

	// =========================================================================
	// Compilation Benchmarks
	// =========================================================================

	fn create_source_tools(count: usize) -> Vec<ToolDefinition> {
		(0..count)
			.map(|i| ToolDefinition::source(format!("tool_{}", i), "backend", format!("original_{}", i)))
			.collect()
	}

	fn create_composition_tools(count: usize) -> Vec<ToolDefinition> {
		(0..count)
			.map(|i| {
				ToolDefinition::composition(
					format!("composition_{}", i),
					PatternSpec::Pipeline(PipelineSpec {
						steps: vec![
							PipelineStep {
								id: "step1".to_string(),
								operation: StepOperation::Tool(ToolCall {
									name: format!("tool_{}", i % 10),
								}),
								input: None,
							},
							PipelineStep {
								id: "step2".to_string(),
								operation: StepOperation::Tool(ToolCall {
									name: "process".to_string(),
								}),
								input: None,
							},
						],
					}),
				)
			})
			.collect()
	}

	fn create_mixed_tools(source_count: usize, composition_count: usize) -> Vec<ToolDefinition> {
		let mut tools = create_source_tools(source_count);
		tools.extend(create_composition_tools(composition_count));
		tools
	}

	#[divan::bench]
	fn compile_10_source_tools(bencher: Bencher) {
		let tools = create_source_tools(10);
		bencher.bench_local(|| {
			let registry = Registry::with_tool_definitions(black_box(tools.clone()));
			CompiledRegistry::compile(registry).unwrap()
		});
	}

	#[divan::bench]
	fn compile_100_source_tools(bencher: Bencher) {
		let tools = create_source_tools(100);
		bencher.bench_local(|| {
			let registry = Registry::with_tool_definitions(black_box(tools.clone()));
			CompiledRegistry::compile(registry).unwrap()
		});
	}

	#[divan::bench]
	fn compile_1000_source_tools(bencher: Bencher) {
		let tools = create_source_tools(1000);
		bencher.bench_local(|| {
			let registry = Registry::with_tool_definitions(black_box(tools.clone()));
			CompiledRegistry::compile(registry).unwrap()
		});
	}

	#[divan::bench]
	fn compile_10_compositions(bencher: Bencher) {
		let tools = create_composition_tools(10);
		bencher.bench_local(|| {
			let registry = Registry::with_tool_definitions(black_box(tools.clone()));
			CompiledRegistry::compile(registry).unwrap()
		});
	}

	#[divan::bench]
	fn compile_100_compositions(bencher: Bencher) {
		let tools = create_composition_tools(100);
		bencher.bench_local(|| {
			let registry = Registry::with_tool_definitions(black_box(tools.clone()));
			CompiledRegistry::compile(registry).unwrap()
		});
	}

	#[divan::bench]
	fn compile_mixed_50_50(bencher: Bencher) {
		let tools = create_mixed_tools(50, 50);
		bencher.bench_local(|| {
			let registry = Registry::with_tool_definitions(black_box(tools.clone()));
			CompiledRegistry::compile(registry).unwrap()
		});
	}

	#[divan::bench]
	fn compile_mixed_500_500(bencher: Bencher) {
		let tools = create_mixed_tools(500, 500);
		bencher.bench_local(|| {
			let registry = Registry::with_tool_definitions(black_box(tools.clone()));
			CompiledRegistry::compile(registry).unwrap()
		});
	}

	// =========================================================================
	// Tool Lookup Benchmarks
	// =========================================================================

	#[divan::bench]
	fn lookup_tool_100_registry(bencher: Bencher) {
		let tools = create_source_tools(100);
		let registry = Registry::with_tool_definitions(tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		bencher.bench_local(|| compiled.get_tool(black_box("tool_50")));
	}

	#[divan::bench]
	fn lookup_tool_1000_registry(bencher: Bencher) {
		let tools = create_source_tools(1000);
		let registry = Registry::with_tool_definitions(tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		bencher.bench_local(|| compiled.get_tool(black_box("tool_500")));
	}

	// =========================================================================
	// Output Transform Benchmarks
	// =========================================================================

	#[divan::bench]
	fn transform_output_simple(bencher: Bencher) {
		use agentgateway::mcp::registry::{OutputField, OutputSchema};
		use std::collections::HashMap;

		let mut properties = HashMap::new();
		properties.insert(
			"field1".to_string(),
			OutputField::new("string", "$.data.field1"),
		);
		properties.insert(
			"field2".to_string(),
			OutputField::new("number", "$.data.field2"),
		);

		let tool = VirtualToolDef::new("test", "backend", "original")
			.with_output_schema(OutputSchema::new(properties));

		let registry = Registry::with_tools(vec![tool]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let input = serde_json::json!({
			"data": {
				"field1": "value1",
				"field2": 42,
				"field3": "ignored"
			}
		});

		bencher.bench_local(|| {
			compiled
				.transform_output("test", black_box(input.clone()))
				.unwrap()
		});
	}

	#[divan::bench]
	fn transform_output_deep_path(bencher: Bencher) {
		use agentgateway::mcp::registry::{OutputField, OutputSchema};
		use std::collections::HashMap;

		let mut properties = HashMap::new();
		properties.insert(
			"deep_value".to_string(),
			OutputField::new("string", "$.level1.level2.level3.level4.value"),
		);

		let tool = VirtualToolDef::new("test", "backend", "original")
			.with_output_schema(OutputSchema::new(properties));

		let registry = Registry::with_tools(vec![tool]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let input = serde_json::json!({
			"level1": {
				"level2": {
					"level3": {
						"level4": {
							"value": "deep_value"
						}
					}
				}
			}
		});

		bencher.bench_local(|| {
			compiled
				.transform_output("test", black_box(input.clone()))
				.unwrap()
		});
	}

	// =========================================================================
	// Default Injection Benchmarks
	// =========================================================================

	#[divan::bench]
	fn inject_defaults_few(bencher: Bencher) {
		let tool = VirtualToolDef::new("test", "backend", "original")
			.with_default("key1", serde_json::json!("value1"))
			.with_default("key2", serde_json::json!(42));

		let registry = Registry::with_tools(vec![tool]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let args = serde_json::json!({
			"user_key": "user_value"
		});

		bencher.bench_local(|| {
			compiled
				.prepare_call_args("test", black_box(args.clone()))
				.unwrap()
		});
	}

	#[divan::bench]
	fn inject_defaults_many(bencher: Bencher) {
		let mut tool = VirtualToolDef::new("test", "backend", "original");
		for i in 0..20 {
			tool = tool.with_default(
				format!("key{}", i),
				serde_json::json!(format!("value{}", i)),
			);
		}

		let registry = Registry::with_tools(vec![tool]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let args = serde_json::json!({
			"user_key": "user_value"
		});

		bencher.bench_local(|| {
			compiled
				.prepare_call_args("test", black_box(args.clone()))
				.unwrap()
		});
	}

	// =========================================================================
	// Timeout Pattern Benchmarks
	// =========================================================================

	use agentgateway::mcp::registry::executor::{
		CompositionExecutor, ExecutionContext, ExecutionError, ScatterGatherExecutor, TimeoutExecutor,
		ToolInvoker,
	};
	use agentgateway::mcp::registry::patterns::TimeoutSpec;
	use serde_json::Value;
	use std::sync::Arc;

	/// Fast mock invoker for benchmarking - returns immediately
	struct FastMockInvoker;

	#[async_trait::async_trait]
	impl ToolInvoker for FastMockInvoker {
		async fn invoke(&self, _tool_name: &str, args: Value) -> Result<Value, ExecutionError> {
			// Minimal work - just return the input
			Ok(args)
		}
	}

	fn setup_timeout_benchmark() -> (ExecutionContext, CompositionExecutor) {
		let registry = Registry::new();
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(FastMockInvoker);

		let ctx = ExecutionContext::new(serde_json::json!({}), compiled.clone(), invoker.clone());
		let executor = CompositionExecutor::new(compiled, invoker);

		(ctx, executor)
	}

	#[divan::bench]
	fn timeout_pattern_success(bencher: Bencher) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let (ctx, executor) = setup_timeout_benchmark();

		let spec = TimeoutSpec {
			inner: Box::new(StepOperation::Tool(ToolCall {
				name: "fast_tool".to_string(),
			})),
			duration_ms: 5000, // Long timeout - won't trigger
			fallback: None,
			message: None,
		};

		let input = serde_json::json!({"data": "test"});

		bencher.bench_local(|| {
			rt.block_on(async {
				TimeoutExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn timeout_pattern_with_fallback_spec(bencher: Bencher) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let (ctx, executor) = setup_timeout_benchmark();

		// Spec with fallback configured (but won't trigger due to long timeout)
		let spec = TimeoutSpec {
			inner: Box::new(StepOperation::Tool(ToolCall {
				name: "fast_tool".to_string(),
			})),
			duration_ms: 5000,
			fallback: Some(Box::new(StepOperation::Tool(ToolCall {
				name: "fallback_tool".to_string(),
			}))),
			message: Some("Custom timeout message".to_string()),
		};

		let input = serde_json::json!({"data": "test"});

		bencher.bench_local(|| {
			rt.block_on(async {
				TimeoutExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench(args = [1, 10, 100, 1000])]
	fn timeout_pattern_varying_payload_size(bencher: Bencher, payload_fields: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let (ctx, executor) = setup_timeout_benchmark();

		let spec = TimeoutSpec {
			inner: Box::new(StepOperation::Tool(ToolCall {
				name: "fast_tool".to_string(),
			})),
			duration_ms: 5000,
			fallback: None,
			message: None,
		};

		// Create payload with varying number of fields
		let mut payload = serde_json::Map::new();
		for i in 0..payload_fields {
			payload.insert(
				format!("field_{}", i),
				serde_json::json!(format!("value_{}", i)),
			);
		}
		let input = Value::Object(payload);

		bencher.bench_local(|| {
			rt.block_on(async {
				TimeoutExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench(args = [100, 1000, 5000, 30000])]
	fn timeout_spec_creation_varying_duration(bencher: Bencher, duration_ms: u32) {
		bencher.bench_local(|| TimeoutSpec {
			inner: Box::new(StepOperation::Tool(ToolCall {
				name: black_box("tool".to_string()),
			})),
			duration_ms: black_box(duration_ms),
			fallback: None,
			message: None,
		});
	}

	// =========================================================================
	// ScatterGather Pattern Benchmarks
	// =========================================================================

	fn setup_scatter_gather_benchmark() -> (ExecutionContext, CompositionExecutor) {
		let registry = Registry::new();
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(FastMockInvoker);

		let ctx = ExecutionContext::new(serde_json::json!({}), compiled.clone(), invoker.clone());
		let executor = CompositionExecutor::new(compiled, invoker);

		(ctx, executor)
	}

	#[divan::bench(args = [2, 5, 10, 20])]
	fn scatter_gather_varying_targets(bencher: Bencher, target_count: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let (ctx, executor) = setup_scatter_gather_benchmark();

		// Create scatter-gather spec with varying number of targets
		let targets: Vec<ScatterTarget> = (0..target_count)
			.map(|i| ScatterTarget::Tool(format!("tool_{}", i)))
			.collect();

		let spec = ScatterGatherSpec {
			targets,
			aggregation: AggregationStrategy { ops: vec![] },
			timeout_ms: None,
			fail_fast: false,
		};

		let input = serde_json::json!({"query": "test"});

		bencher.bench_local(|| {
			rt.block_on(async {
				ScatterGatherExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn scatter_gather_with_flatten(bencher: Bencher) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let (ctx, executor) = setup_scatter_gather_benchmark();

		let spec = ScatterGatherSpec {
			targets: vec![
				ScatterTarget::Tool("tool_1".to_string()),
				ScatterTarget::Tool("tool_2".to_string()),
				ScatterTarget::Tool("tool_3".to_string()),
			],
			aggregation: AggregationStrategy {
				ops: vec![AggregationOp::Flatten(true)],
			},
			timeout_ms: None,
			fail_fast: false,
		};

		let input = serde_json::json!({"query": "test"});

		bencher.bench_local(|| {
			rt.block_on(async {
				ScatterGatherExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn scatter_gather_with_sort_and_limit(bencher: Bencher) {
		use agentgateway::mcp::registry::patterns::{LimitOp, SortOp};

		let rt = tokio::runtime::Runtime::new().unwrap();
		let (ctx, executor) = setup_scatter_gather_benchmark();

		let spec = ScatterGatherSpec {
			targets: vec![
				ScatterTarget::Tool("tool_1".to_string()),
				ScatterTarget::Tool("tool_2".to_string()),
				ScatterTarget::Tool("tool_3".to_string()),
			],
			aggregation: AggregationStrategy {
				ops: vec![
					AggregationOp::Flatten(true),
					AggregationOp::Sort(SortOp {
						field: "$.score".to_string(),
						order: "desc".to_string(),
					}),
					AggregationOp::Limit(LimitOp { count: 10 }),
				],
			},
			timeout_ms: None,
			fail_fast: false,
		};

		let input = serde_json::json!({"query": "test"});

		bencher.bench_local(|| {
			rt.block_on(async {
				ScatterGatherExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn scatter_gather_with_timeout(bencher: Bencher) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let (ctx, executor) = setup_scatter_gather_benchmark();

		let spec = ScatterGatherSpec {
			targets: vec![
				ScatterTarget::Tool("tool_1".to_string()),
				ScatterTarget::Tool("tool_2".to_string()),
			],
			aggregation: AggregationStrategy { ops: vec![] },
			timeout_ms: Some(5000), // 5 second timeout
			fail_fast: false,
		};

		let input = serde_json::json!({"query": "test"});

		bencher.bench_local(|| {
			rt.block_on(async {
				ScatterGatherExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn scatter_gather_fail_fast(bencher: Bencher) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let (ctx, executor) = setup_scatter_gather_benchmark();

		let spec = ScatterGatherSpec {
			targets: vec![
				ScatterTarget::Tool("tool_1".to_string()),
				ScatterTarget::Tool("tool_2".to_string()),
				ScatterTarget::Tool("tool_3".to_string()),
			],
			aggregation: AggregationStrategy { ops: vec![] },
			timeout_ms: None,
			fail_fast: true, // Stop on first failure
		};

		let input = serde_json::json!({"query": "test"});

		bencher.bench_local(|| {
			rt.block_on(async {
				ScatterGatherExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench(args = [1, 10, 100, 1000])]
	fn scatter_gather_varying_payload_size(bencher: Bencher, payload_fields: usize) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		let (ctx, executor) = setup_scatter_gather_benchmark();

		let spec = ScatterGatherSpec {
			targets: vec![
				ScatterTarget::Tool("tool_1".to_string()),
				ScatterTarget::Tool("tool_2".to_string()),
			],
			aggregation: AggregationStrategy { ops: vec![] },
			timeout_ms: None,
			fail_fast: false,
		};

		// Create payload with varying number of fields
		let mut payload = serde_json::Map::new();
		for i in 0..payload_fields {
			payload.insert(
				format!("field_{}", i),
				serde_json::json!(format!("value_{}", i)),
			);
		}
		let input = Value::Object(payload);

		bencher.bench_local(|| {
			rt.block_on(async {
				ScatterGatherExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}
}
