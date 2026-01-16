// Execution Graph representation for compiled compositions
//
// The execution graph is a DAG (Directed Acyclic Graph) representing
// the flow of data through a composition's operations.


use super::patterns::{
	AggregationStrategy, DataBinding, FilterSpec, MapEachInner, PatternSpec, SchemaMapSpec,
};

/// An execution graph representing a compiled composition
#[derive(Debug, Clone)]
pub struct ExecutionGraph {
	/// All nodes in the graph
	pub nodes: Vec<ExecutionNode>,
	/// Entry node index
	pub entry: usize,
	/// Exit node index (where final output comes from)
	pub exit: usize,
}

/// A node in the execution graph
#[derive(Debug, Clone)]
pub struct ExecutionNode {
	/// Unique ID within the graph
	pub id: String,
	/// The operation this node performs
	pub operation: NodeOperation,
	/// Input sources for this node
	pub inputs: Vec<NodeInput>,
}

/// The operation a node performs
#[derive(Debug, Clone)]
pub enum NodeOperation {
	/// Call a tool
	ToolCall { name: String },

	/// Execute a nested pattern
	Pattern(Box<PatternSpec>),

	/// Pipeline: execute steps in sequence
	Pipeline { steps: Vec<PipelineStepNode> },

	/// Scatter-gather: fan out to multiple targets, aggregate results
	ScatterGather {
		targets: Vec<ScatterTargetNode>,
		aggregation: AggregationStrategy,
		timeout_ms: Option<u32>,
		fail_fast: bool,
	},

	/// Filter: keep elements matching predicate
	Filter(FilterSpec),

	/// SchemaMap: transform fields
	SchemaMap(SchemaMapSpec),

	/// MapEach: apply operation to each array element
	MapEach { inner: MapEachInner },

	/// Input: the composition's input
	Input,

	/// Output: the composition's output (identity)
	Output,
}

/// A step in a pipeline node
#[derive(Debug, Clone)]
pub struct PipelineStepNode {
	/// Step ID
	pub id: String,
	/// Step operation
	pub operation: StepOperationNode,
	/// Input binding
	pub input: Option<DataBinding>,
}

/// Step operation in pipeline
#[derive(Debug, Clone)]
pub enum StepOperationNode {
	/// Tool call
	Tool { name: String },
	/// Inline pattern
	Pattern(Box<PatternSpec>),
}

/// A target in scatter-gather
#[derive(Debug, Clone)]
pub enum ScatterTargetNode {
	/// Tool reference
	Tool(String),
	/// Inline pattern
	Pattern(Box<PatternSpec>),
}

/// Input source for a node
#[derive(Debug, Clone)]
pub enum NodeInput {
	/// From the composition input
	CompositionInput,
	/// From another node's output
	Node { node_index: usize, path: Option<String> },
	/// Constant value
	Constant(serde_json::Value),
}

impl ExecutionGraph {
	/// Build an execution graph from a pattern spec
	pub fn from_pattern(spec: &PatternSpec) -> Self {
		let mut nodes = Vec::new();

		// Input node
		nodes.push(ExecutionNode {
			id: "_input".to_string(),
			operation: NodeOperation::Input,
			inputs: vec![],
		});

		// Main pattern node
		let main_node = ExecutionNode {
			id: "_main".to_string(),
			operation: Self::pattern_to_operation(spec),
			inputs: vec![NodeInput::Node { node_index: 0, path: None }],
		};
		nodes.push(main_node);

		// Output node
		nodes.push(ExecutionNode {
			id: "_output".to_string(),
			operation: NodeOperation::Output,
			inputs: vec![NodeInput::Node { node_index: 1, path: None }],
		});

		Self { nodes, entry: 0, exit: 2 }
	}

	/// Convert a pattern spec to a node operation
	fn pattern_to_operation(spec: &PatternSpec) -> NodeOperation {
		match spec {
			// Stateless patterns - convert to node operations
			PatternSpec::Pipeline(p) => {
				let steps = p
					.steps
					.iter()
					.map(|s| PipelineStepNode {
						id: s.id.clone(),
						operation: match &s.operation {
							super::patterns::StepOperation::Tool(tc) => {
								StepOperationNode::Tool { name: tc.name.clone() }
							},
							super::patterns::StepOperation::Pattern(p) => {
								StepOperationNode::Pattern(p.clone())
							},
						},
						input: s.input.clone(),
					})
					.collect();
				NodeOperation::Pipeline { steps }
			},
			PatternSpec::ScatterGather(sg) => {
				let targets = sg
					.targets
					.iter()
					.map(|t| match t {
						super::patterns::ScatterTarget::Tool(name) => ScatterTargetNode::Tool(name.clone()),
						super::patterns::ScatterTarget::Pattern(p) => ScatterTargetNode::Pattern(p.clone()),
					})
					.collect();
				NodeOperation::ScatterGather {
					targets,
					aggregation: sg.aggregation.clone(),
					timeout_ms: sg.timeout_ms,
					fail_fast: sg.fail_fast,
				}
			},
			PatternSpec::Filter(f) => NodeOperation::Filter(f.clone()),
			PatternSpec::SchemaMap(sm) => NodeOperation::SchemaMap(sm.clone()),
			PatternSpec::MapEach(me) => NodeOperation::MapEach { inner: me.inner.clone() },

			// Stateful patterns - wrap as Pattern for now (execution will error at runtime)
			PatternSpec::Retry(_)
			| PatternSpec::Timeout(_)
			| PatternSpec::Cache(_)
			| PatternSpec::Idempotent(_)
			| PatternSpec::CircuitBreaker(_)
			| PatternSpec::DeadLetter(_)
			| PatternSpec::Saga(_)
			| PatternSpec::ClaimCheck(_) => NodeOperation::Pattern(Box::new(spec.clone())),
		}
	}

	/// Get a node by index
	pub fn get_node(&self, index: usize) -> Option<&ExecutionNode> {
		self.nodes.get(index)
	}

	/// Get the number of nodes
	pub fn node_count(&self) -> usize {
		self.nodes.len()
	}

	/// Get all tool references in this graph
	pub fn tool_references(&self) -> Vec<String> {
		let mut refs = Vec::new();
		for node in &self.nodes {
			Self::collect_tool_refs(&node.operation, &mut refs);
		}
		refs
	}

	fn collect_tool_refs(op: &NodeOperation, refs: &mut Vec<String>) {
		match op {
			NodeOperation::ToolCall { name } => refs.push(name.clone()),
			NodeOperation::Pipeline { steps } => {
				for step in steps {
					match &step.operation {
						StepOperationNode::Tool { name } => refs.push(name.clone()),
						StepOperationNode::Pattern(p) => {
							let inner_op = Self::pattern_to_operation(p);
							Self::collect_tool_refs(&inner_op, refs);
						},
					}
				}
			},
			NodeOperation::ScatterGather { targets, .. } => {
				for target in targets {
					match target {
						ScatterTargetNode::Tool(name) => refs.push(name.clone()),
						ScatterTargetNode::Pattern(p) => {
							let inner_op = Self::pattern_to_operation(p);
							Self::collect_tool_refs(&inner_op, refs);
						},
					}
				}
			},
			NodeOperation::MapEach { inner } => match inner {
				MapEachInner::Tool(name) => refs.push(name.clone()),
				MapEachInner::Pattern(p) => {
					let inner_op = Self::pattern_to_operation(p);
					Self::collect_tool_refs(&inner_op, refs);
				},
			},
			NodeOperation::Pattern(p) => {
				let inner_op = Self::pattern_to_operation(p);
				Self::collect_tool_refs(&inner_op, refs);
			},
			NodeOperation::Filter(_) | NodeOperation::SchemaMap(_) | NodeOperation::Input | NodeOperation::Output => {},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::super::patterns::{AggregationOp, AggregationStrategy, PipelineSpec, PipelineStep, ScatterGatherSpec, StepOperation, ToolCall};
	use super::*;

	#[test]
	fn test_build_pipeline_graph() {
		let spec = PatternSpec::Pipeline(PipelineSpec {
			steps: vec![
				PipelineStep {
					id: "step1".to_string(),
					operation: StepOperation::Tool(ToolCall { name: "search".to_string() }),
					input: None,
				},
				PipelineStep {
					id: "step2".to_string(),
					operation: StepOperation::Tool(ToolCall { name: "summarize".to_string() }),
					input: None,
				},
			],
		});

		let graph = ExecutionGraph::from_pattern(&spec);

		assert_eq!(graph.node_count(), 3); // input, main, output
		assert_eq!(graph.entry, 0);
		assert_eq!(graph.exit, 2);

		let refs = graph.tool_references();
		assert!(refs.contains(&"search".to_string()));
		assert!(refs.contains(&"summarize".to_string()));
	}

	#[test]
	fn test_build_scatter_gather_graph() {
		let spec = PatternSpec::ScatterGather(ScatterGatherSpec {
			targets: vec![
				super::super::patterns::ScatterTarget::Tool("tool_a".to_string()),
				super::super::patterns::ScatterTarget::Tool("tool_b".to_string()),
			],
			aggregation: AggregationStrategy { ops: vec![AggregationOp::Flatten(true)] },
			timeout_ms: Some(5000),
			fail_fast: false,
		});

		let graph = ExecutionGraph::from_pattern(&spec);

		assert_eq!(graph.node_count(), 3);

		let refs = graph.tool_references();
		assert_eq!(refs.len(), 2);
		assert!(refs.contains(&"tool_a".to_string()));
		assert!(refs.contains(&"tool_b".to_string()));
	}

	#[test]
	fn test_build_filter_graph() {
		use super::super::patterns::{FieldPredicate, PredicateValue};

		let spec = PatternSpec::Filter(FilterSpec {
			predicate: FieldPredicate {
				field: "$.score".to_string(),
				op: "gt".to_string(),
				value: PredicateValue::NumberValue(0.5),
			},
		});

		let graph = ExecutionGraph::from_pattern(&spec);

		assert_eq!(graph.node_count(), 3);
		assert!(graph.tool_references().is_empty());
	}

	#[test]
	fn test_build_map_each_graph() {
		let spec = PatternSpec::MapEach(super::super::patterns::MapEachSpec {
			inner: MapEachInner::Tool("fetch".to_string()),
		});

		let graph = ExecutionGraph::from_pattern(&spec);

		assert_eq!(graph.node_count(), 3);
		let refs = graph.tool_references();
		assert_eq!(refs, vec!["fetch"]);
	}
}

