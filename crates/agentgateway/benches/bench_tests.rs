fn main() {
	#[cfg(all(not(test), not(feature = "internal_benches")))]
	panic!("benches must have -F internal_benches");
	use agentgateway as _;
	divan::main();
}

#[cfg(feature = "internal_benches")]
mod composition_benchmarks {
	use agentgateway::mcp::registry::{
		CompiledRegistry, PatternSpec, PipelineSpec, PipelineStep, Registry, StepOperation, ToolCall,
		ToolDefinition, VirtualToolDef,
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
}

// =============================================================================
// Pipeline Execution Benchmarks
// =============================================================================

#[cfg(feature = "internal_benches")]
mod pipeline_benchmarks {
	use agentgateway::mcp::registry::{
		CompiledRegistry, CompositionExecutor, ExecutionContext, MockToolInvoker, PipelineExecutor,
		PipelineSpec, PipelineStep, Registry, StepOperation, ToolCall,
	};
	use divan::{black_box, Bencher};
	use std::sync::Arc;

	/// Create a pipeline spec with N steps
	fn create_pipeline_spec(num_steps: usize) -> PipelineSpec {
		let steps = (0..num_steps)
			.map(|i| PipelineStep {
				id: format!("step_{}", i),
				operation: StepOperation::Tool(ToolCall {
					name: format!("tool_{}", i),
				}),
				input: None,
			})
			.collect();
		PipelineSpec { steps }
	}

	/// Create mock invoker with responses for N tools
	fn create_mock_invoker(num_tools: usize) -> MockToolInvoker {
		let mut invoker = MockToolInvoker::new();
		for i in 0..num_tools {
			invoker = invoker.with_response(
				&format!("tool_{}", i),
				serde_json::json!({"step": i, "data": "processed"}),
			);
		}
		invoker
	}

	/// Setup executor and context for pipeline execution
	fn setup_executor(
		invoker: MockToolInvoker,
	) -> (Arc<CompiledRegistry>, Arc<MockToolInvoker>, CompositionExecutor) {
		let registry = Registry::new();
		let compiled = Arc::new(CompiledRegistry::compile(registry).unwrap());
		let invoker = Arc::new(invoker);
		let executor = CompositionExecutor::new(compiled.clone(), invoker.clone());
		(compiled, invoker, executor)
	}

	// -------------------------------------------------------------------------
	// Pipeline Execution: Varying Pipeline Depth
	// -------------------------------------------------------------------------

	#[divan::bench]
	fn pipeline_1_step(bencher: Bencher) {
		let rt = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap();

		let spec = create_pipeline_spec(1);
		let invoker = create_mock_invoker(1);
		let (compiled, invoker_arc, executor) = setup_executor(invoker);
		let input = serde_json::json!({"initial": "data"});

		bencher.bench_local(|| {
			rt.block_on(async {
				let ctx = ExecutionContext::new(
					serde_json::json!({}),
					compiled.clone(),
					invoker_arc.clone(),
				);
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn pipeline_3_steps(bencher: Bencher) {
		let rt = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap();

		let spec = create_pipeline_spec(3);
		let invoker = create_mock_invoker(3);
		let (compiled, invoker_arc, executor) = setup_executor(invoker);
		let input = serde_json::json!({"initial": "data"});

		bencher.bench_local(|| {
			rt.block_on(async {
				let ctx = ExecutionContext::new(
					serde_json::json!({}),
					compiled.clone(),
					invoker_arc.clone(),
				);
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn pipeline_5_steps(bencher: Bencher) {
		let rt = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap();

		let spec = create_pipeline_spec(5);
		let invoker = create_mock_invoker(5);
		let (compiled, invoker_arc, executor) = setup_executor(invoker);
		let input = serde_json::json!({"initial": "data"});

		bencher.bench_local(|| {
			rt.block_on(async {
				let ctx = ExecutionContext::new(
					serde_json::json!({}),
					compiled.clone(),
					invoker_arc.clone(),
				);
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn pipeline_10_steps(bencher: Bencher) {
		let rt = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap();

		let spec = create_pipeline_spec(10);
		let invoker = create_mock_invoker(10);
		let (compiled, invoker_arc, executor) = setup_executor(invoker);
		let input = serde_json::json!({"initial": "data"});

		bencher.bench_local(|| {
			rt.block_on(async {
				let ctx = ExecutionContext::new(
					serde_json::json!({}),
					compiled.clone(),
					invoker_arc.clone(),
				);
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	// -------------------------------------------------------------------------
	// Pipeline Execution: Data Binding Overhead
	// -------------------------------------------------------------------------

	#[divan::bench]
	fn pipeline_with_input_binding(bencher: Bencher) {
		use agentgateway::mcp::registry::{DataBinding, InputBinding};

		let rt = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap();

		// Pipeline with explicit input binding using JSONPath
		let spec = PipelineSpec {
			steps: vec![
				PipelineStep {
					id: "step_0".to_string(),
					operation: StepOperation::Tool(ToolCall {
						name: "tool_0".to_string(),
					}),
					input: Some(DataBinding::Input(InputBinding {
						path: "$.query".to_string(),
					})),
				},
				PipelineStep {
					id: "step_1".to_string(),
					operation: StepOperation::Tool(ToolCall {
						name: "tool_1".to_string(),
					}),
					input: None, // Uses previous step output
				},
			],
		};

		let invoker = create_mock_invoker(2);
		let (compiled, invoker_arc, executor) = setup_executor(invoker);
		let input = serde_json::json!({"query": "search term", "extra": "ignored"});

		bencher.bench_local(|| {
			rt.block_on(async {
				let ctx = ExecutionContext::new(
					serde_json::json!({}),
					compiled.clone(),
					invoker_arc.clone(),
				);
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn pipeline_with_step_binding(bencher: Bencher) {
		use agentgateway::mcp::registry::{DataBinding, StepBinding};

		let rt = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap();

		// Pipeline where step 2 references step 0's output via JSONPath
		let spec = PipelineSpec {
			steps: vec![
				PipelineStep {
					id: "fetch".to_string(),
					operation: StepOperation::Tool(ToolCall {
						name: "tool_0".to_string(),
					}),
					input: None,
				},
				PipelineStep {
					id: "transform".to_string(),
					operation: StepOperation::Tool(ToolCall {
						name: "tool_1".to_string(),
					}),
					input: None,
				},
				PipelineStep {
					id: "enrich".to_string(),
					operation: StepOperation::Tool(ToolCall {
						name: "tool_2".to_string(),
					}),
					input: Some(DataBinding::Step(StepBinding {
						step_id: "fetch".to_string(),
						path: "$.data".to_string(),
					})),
				},
			],
		};

		let invoker = create_mock_invoker(3);
		let (compiled, invoker_arc, executor) = setup_executor(invoker);
		let input = serde_json::json!({"initial": "data"});

		bencher.bench_local(|| {
			rt.block_on(async {
				let ctx = ExecutionContext::new(
					serde_json::json!({}),
					compiled.clone(),
					invoker_arc.clone(),
				);
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	// -------------------------------------------------------------------------
	// Pipeline Execution: Payload Size Scaling
	// -------------------------------------------------------------------------

	fn create_large_json(size_kb: usize) -> serde_json::Value {
		let data: String = "x".repeat(size_kb * 1024);
		serde_json::json!({
			"payload": data,
			"metadata": {"size_kb": size_kb}
		})
	}

	#[divan::bench]
	fn pipeline_small_payload_1kb(bencher: Bencher) {
		let rt = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap();

		let spec = create_pipeline_spec(3);
		let invoker = create_mock_invoker(3);
		let (compiled, invoker_arc, executor) = setup_executor(invoker);
		let input = create_large_json(1);

		bencher.bench_local(|| {
			rt.block_on(async {
				let ctx = ExecutionContext::new(
					serde_json::json!({}),
					compiled.clone(),
					invoker_arc.clone(),
				);
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn pipeline_medium_payload_10kb(bencher: Bencher) {
		let rt = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap();

		let spec = create_pipeline_spec(3);
		let invoker = create_mock_invoker(3);
		let (compiled, invoker_arc, executor) = setup_executor(invoker);
		let input = create_large_json(10);

		bencher.bench_local(|| {
			rt.block_on(async {
				let ctx = ExecutionContext::new(
					serde_json::json!({}),
					compiled.clone(),
					invoker_arc.clone(),
				);
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}

	#[divan::bench]
	fn pipeline_large_payload_100kb(bencher: Bencher) {
		let rt = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap();

		let spec = create_pipeline_spec(3);
		let invoker = create_mock_invoker(3);
		let (compiled, invoker_arc, executor) = setup_executor(invoker);
		let input = create_large_json(100);

		bencher.bench_local(|| {
			rt.block_on(async {
				let ctx = ExecutionContext::new(
					serde_json::json!({}),
					compiled.clone(),
					invoker_arc.clone(),
				);
				PipelineExecutor::execute(&spec, black_box(input.clone()), &ctx, &executor).await
			})
		});
	}
}
