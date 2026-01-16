fn main() {
	#[cfg(all(not(test), not(feature = "internal_benches")))]
	panic!("benches must have -F internal_benches");
	use agentgateway as _;
	divan::main();
}

#[cfg(feature = "internal_benches")]
mod composition_benchmarks {
	use agentgateway::mcp::registry::{
		AggregationOp, AggregationStrategy, CompiledRegistry, PatternSpec, PipelineSpec,
		PipelineStep, Registry, ScatterGatherSpec, ScatterTarget, StepOperation, ToolCall,
		ToolDefinition, VirtualToolDef,
	};
	use divan::{black_box, Bencher};

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
								operation: StepOperation::Tool(ToolCall { name: "process".to_string() }),
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

		bencher.bench_local(|| {
			compiled.get_tool(black_box("tool_50"))
		});
	}

	#[divan::bench]
	fn lookup_tool_1000_registry(bencher: Bencher) {
		let tools = create_source_tools(1000);
		let registry = Registry::with_tool_definitions(tools);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		bencher.bench_local(|| {
			compiled.get_tool(black_box("tool_500"))
		});
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
			compiled.transform_output("test", black_box(input.clone())).unwrap()
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
			compiled.transform_output("test", black_box(input.clone())).unwrap()
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
			compiled.prepare_call_args("test", black_box(args.clone())).unwrap()
		});
	}

	#[divan::bench]
	fn inject_defaults_many(bencher: Bencher) {
		let mut tool = VirtualToolDef::new("test", "backend", "original");
		for i in 0..20 {
			tool = tool.with_default(format!("key{}", i), serde_json::json!(format!("value{}", i)));
		}

		let registry = Registry::with_tools(vec![tool]);
		let compiled = CompiledRegistry::compile(registry).unwrap();

		let args = serde_json::json!({
			"user_key": "user_value"
		});

		bencher.bench_local(|| {
			compiled.prepare_call_args("test", black_box(args.clone())).unwrap()
		});
	}
}
