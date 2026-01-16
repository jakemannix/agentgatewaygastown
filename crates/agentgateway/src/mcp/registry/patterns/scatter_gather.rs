// Scatter-Gather pattern types

use serde::{Deserialize, Serialize};

use super::PatternSpec;

/// ScatterGatherSpec fans out to multiple targets in parallel and aggregates results
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScatterGatherSpec {
	/// Targets to invoke in parallel
	pub targets: Vec<ScatterTarget>,

	/// How to aggregate results
	pub aggregation: AggregationStrategy,

	/// Timeout in milliseconds (optional)
	#[serde(default)]
	pub timeout_ms: Option<u32>,

	/// If true, fail immediately on first error
	#[serde(default)]
	pub fail_fast: bool,
}

impl ScatterGatherSpec {
	/// Get the names of tools referenced by this scatter-gather
	pub fn referenced_tools(&self) -> Vec<&str> {
		self.targets.iter().flat_map(|t| t.referenced_tools()).collect()
	}
}

/// A target in a scatter-gather operation
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScatterTarget {
	/// Tool name (resolved from registry or backend)
	Tool(String),

	/// Inline pattern
	Pattern(Box<PatternSpec>),
}

impl ScatterTarget {
	/// Get the names of tools referenced by this target
	pub fn referenced_tools(&self) -> Vec<&str> {
		match self {
			ScatterTarget::Tool(name) => vec![name.as_str()],
			ScatterTarget::Pattern(p) => p.referenced_tools(),
		}
	}
}

/// AggregationStrategy defines how to combine scatter-gather results
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationStrategy {
	/// Sequence of operations applied in order
	pub ops: Vec<AggregationOp>,
}

impl Default for AggregationStrategy {
	fn default() -> Self {
		Self { ops: vec![AggregationOp::Flatten(true)] }
	}
}

/// A single aggregation operation
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AggregationOp {
	/// Flatten array of arrays into single array
	Flatten(bool),

	/// Sort by field
	Sort(SortOp),

	/// Deduplicate by field
	Dedupe(DedupeOp),

	/// Take first N results
	Limit(LimitOp),

	/// Keep arrays nested (no flattening)
	Concat(bool),

	/// Merge objects (for object results)
	Merge(bool),
}

/// Sort operation
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SortOp {
	/// JSONPath to the field to sort by
	pub field: String,

	/// Sort order: "asc" or "desc"
	pub order: String,
}

/// Dedupe operation
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DedupeOp {
	/// JSONPath to the field to dedupe by
	pub field: String,
}

/// Limit operation
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitOp {
	/// Maximum number of results
	pub count: u32,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_scatter_gather() {
		let json = r#"{
			"targets": [
				{ "tool": "search_web" },
				{ "tool": "search_arxiv" }
			],
			"aggregation": {
				"ops": [
					{ "flatten": true },
					{ "sort": { "field": "$.score", "order": "desc" } },
					{ "limit": { "count": 10 } }
				]
			},
			"timeoutMs": 5000,
			"failFast": true
		}"#;

		let sg: ScatterGatherSpec = serde_json::from_str(json).unwrap();
		assert_eq!(sg.targets.len(), 2);
		assert_eq!(sg.aggregation.ops.len(), 3);
		assert_eq!(sg.timeout_ms, Some(5000));
		assert!(sg.fail_fast);
	}

	#[test]
	fn test_parse_scatter_target_tool() {
		let json = r#"{ "tool": "my_tool" }"#;
		let target: ScatterTarget = serde_json::from_str(json).unwrap();
		assert!(matches!(target, ScatterTarget::Tool(_)));
		if let ScatterTarget::Tool(name) = target {
			assert_eq!(name, "my_tool");
		}
	}

	#[test]
	fn test_parse_aggregation_ops() {
		let json = r#"{
			"ops": [
				{ "flatten": true },
				{ "sort": { "field": "$.relevance", "order": "desc" } },
				{ "dedupe": { "field": "$.id" } },
				{ "limit": { "count": 5 } }
			]
		}"#;

		let strategy: AggregationStrategy = serde_json::from_str(json).unwrap();
		assert_eq!(strategy.ops.len(), 4);
		assert!(matches!(strategy.ops[0], AggregationOp::Flatten(true)));
		assert!(matches!(strategy.ops[1], AggregationOp::Sort(_)));
		assert!(matches!(strategy.ops[2], AggregationOp::Dedupe(_)));
		assert!(matches!(strategy.ops[3], AggregationOp::Limit(_)));
	}

	#[test]
	fn test_referenced_tools() {
		let json = r#"{
			"targets": [
				{ "tool": "tool_a" },
				{ "tool": "tool_b" }
			],
			"aggregation": { "ops": [] }
		}"#;

		let sg: ScatterGatherSpec = serde_json::from_str(json).unwrap();
		let refs = sg.referenced_tools();
		assert_eq!(refs, vec!["tool_a", "tool_b"]);
	}
}

