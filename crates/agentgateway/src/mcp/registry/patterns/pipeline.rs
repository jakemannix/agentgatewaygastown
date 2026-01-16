// Pipeline pattern types

use serde::{Deserialize, Serialize};

use super::PatternSpec;

/// PipelineSpec executes steps sequentially, passing output to next step
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineSpec {
	/// Steps to execute in order
	pub steps: Vec<PipelineStep>,
}

impl PipelineSpec {
	/// Get the names of tools referenced by this pipeline
	pub fn referenced_tools(&self) -> Vec<&str> {
		self.steps
			.iter()
			.flat_map(|step| step.operation.referenced_tools())
			.collect()
	}
}

/// A single step in a pipeline
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStep {
	/// Unique identifier for this step (for data binding references)
	pub id: String,

	/// The operation to execute
	pub operation: StepOperation,

	/// Input binding for this step
	#[serde(default)]
	pub input: Option<DataBinding>,
}

/// StepOperation defines what a step does
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StepOperation {
	/// Call a tool by name
	Tool(ToolCall),

	/// Inline pattern (no separate name)
	Pattern(Box<PatternSpec>),
}

impl StepOperation {
	/// Get the names of tools referenced by this operation
	pub fn referenced_tools(&self) -> Vec<&str> {
		match self {
			StepOperation::Tool(tc) => vec![tc.name.as_str()],
			StepOperation::Pattern(p) => p.referenced_tools(),
		}
	}
}

/// Tool call reference
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
	/// Tool name (can be virtual tool, composition, or backend tool)
	pub name: String,
}

/// DataBinding specifies where step input comes from
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DataBinding {
	/// From composition input
	Input(InputBinding),

	/// From a previous step's output
	Step(StepBinding),

	/// Constant value
	Constant(serde_json::Value),

	/// Construct an object from multiple bindings
	/// This enables input schema construction from prior step outputs
	Construct(ConstructBinding),
}

impl Default for DataBinding {
	fn default() -> Self {
		DataBinding::Input(InputBinding { path: "$".to_string() })
	}
}

/// Input binding - reference to composition input
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputBinding {
	/// JSONPath into composition input
	pub path: String,
}

/// Step binding - reference to a previous step's output
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepBinding {
	/// ID of the step to reference
	pub step_id: String,

	/// JSONPath into step output
	pub path: String,
}

/// Construct binding - build an object from multiple bindings
/// Enables symmetric input construction (like outputTransform does for outputs)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstructBinding {
	/// Field name -> binding that produces the field value
	pub fields: std::collections::HashMap<String, DataBinding>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_simple_pipeline() {
		let json = r#"{
			"steps": [
				{
					"id": "search",
					"operation": { "tool": { "name": "web_search" } },
					"input": { "input": { "path": "$" } }
				},
				{
					"id": "summarize",
					"operation": { "tool": { "name": "summarize" } },
					"input": { "step": { "stepId": "search", "path": "$" } }
				}
			]
		}"#;

		let pipeline: PipelineSpec = serde_json::from_str(json).unwrap();
		assert_eq!(pipeline.steps.len(), 2);
		assert_eq!(pipeline.steps[0].id, "search");
		assert_eq!(pipeline.steps[1].id, "summarize");
	}

	#[test]
	fn test_parse_step_operation_tool() {
		let json = r#"{ "tool": { "name": "fetch" } }"#;
		let op: StepOperation = serde_json::from_str(json).unwrap();
		assert!(matches!(op, StepOperation::Tool(_)));
		if let StepOperation::Tool(tc) = op {
			assert_eq!(tc.name, "fetch");
		}
	}

	#[test]
	fn test_parse_data_binding_input() {
		let json = r#"{ "input": { "path": "$.query" } }"#;
		let binding: DataBinding = serde_json::from_str(json).unwrap();
		assert!(matches!(binding, DataBinding::Input(_)));
	}

	#[test]
	fn test_parse_data_binding_step() {
		let json = r#"{ "step": { "stepId": "step1", "path": "$.results" } }"#;
		let binding: DataBinding = serde_json::from_str(json).unwrap();
		assert!(matches!(binding, DataBinding::Step(_)));
	}

	#[test]
	fn test_parse_data_binding_constant() {
		let json = r#"{ "constant": "fixed_value" }"#;
		let binding: DataBinding = serde_json::from_str(json).unwrap();
		assert!(matches!(binding, DataBinding::Constant(_)));
	}

	#[test]
	fn test_referenced_tools() {
		let json = r#"{
			"steps": [
				{
					"id": "s1",
					"operation": { "tool": { "name": "tool_a" } }
				},
				{
					"id": "s2",
					"operation": { "tool": { "name": "tool_b" } }
				}
			]
		}"#;

		let pipeline: PipelineSpec = serde_json::from_str(json).unwrap();
		let refs = pipeline.referenced_tools();
		assert_eq!(refs, vec!["tool_a", "tool_b"]);
	}
}

