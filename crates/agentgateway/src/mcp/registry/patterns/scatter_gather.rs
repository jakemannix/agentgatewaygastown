// Scatter-Gather pattern types

use serde::{Deserialize, Deserializer, Serialize};

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
		self
			.targets
			.iter()
			.flat_map(|t| t.referenced_tools())
			.collect()
	}
}

/// A target in a scatter-gather operation
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScatterTarget {
	/// Tool reference (optionally with server)
	Tool(ToolRef),

	/// Inline pattern
	Pattern(Box<PatternSpec>),
}

/// Reference to a tool, optionally on a specific server/backend
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRef {
	/// Tool name
	pub tool: String,

	/// Server/backend name (None = resolve from registry virtual tools)
	#[serde(default)]
	pub server: Option<String>,
}

impl ToolRef {
	/// Create a new tool reference without a server
	pub fn new(tool: impl Into<String>) -> Self {
		Self {
			tool: tool.into(),
			server: None,
		}
	}

	/// Create a new tool reference with a server
	pub fn with_server(tool: impl Into<String>, server: impl Into<String>) -> Self {
		Self {
			tool: tool.into(),
			server: Some(server.into()),
		}
	}
}

// Custom deserialization to handle both formats:
// - { "tool": "name" }
// - { "tool": "name", "server": "backend" }
// - { "pattern": { ... } }
impl<'de> Deserialize<'de> for ScatterTarget {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		#[serde(rename_all = "camelCase")]
		struct ScatterTargetHelper {
			#[serde(default)]
			tool: Option<String>,
			#[serde(default)]
			server: Option<String>,
			#[serde(default)]
			pattern: Option<PatternSpec>,
		}

		let helper = ScatterTargetHelper::deserialize(deserializer)?;

		if let Some(pattern) = helper.pattern {
			Ok(ScatterTarget::Pattern(Box::new(pattern)))
		} else if let Some(tool) = helper.tool {
			Ok(ScatterTarget::Tool(ToolRef {
				tool,
				server: helper.server,
			}))
		} else {
			Err(serde::de::Error::custom(
				"ScatterTarget must have either 'tool' or 'pattern' field",
			))
		}
	}
}

impl ScatterTarget {
	/// Get the names of tools referenced by this target
	pub fn referenced_tools(&self) -> Vec<&str> {
		match self {
			ScatterTarget::Tool(tool_ref) => vec![tool_ref.tool.as_str()],
			ScatterTarget::Pattern(p) => p.referenced_tools(),
		}
	}

	/// Get the server name if this is a backend tool reference
	pub fn server(&self) -> Option<&str> {
		match self {
			ScatterTarget::Tool(tool_ref) => tool_ref.server.as_deref(),
			ScatterTarget::Pattern(_) => None,
		}
	}

	/// Get the tool name if this is a tool reference
	pub fn tool_name(&self) -> Option<&str> {
		match self {
			ScatterTarget::Tool(tool_ref) => Some(&tool_ref.tool),
			ScatterTarget::Pattern(_) => None,
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
		Self {
			ops: vec![AggregationOp::Flatten(true)],
		}
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

	/// Wrap the result array in an object with a specified field name
	/// e.g., Wrap("results") turns [a, b, c] into {"results": [a, b, c]}
	Wrap(WrapOp),

	/// Extract a field from each element using JSONPath
	/// e.g., Extract("$.results") on [{results: [a, b]}, {results: [c]}]
	/// produces [[a, b], [c]] which can then be flattened
	Extract(ExtractOp),
}

/// Wrap operation - wraps an array in an object with specified field name
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WrapOp {
	/// The field name to wrap the array under
	pub field: String,
}

/// Extract operation - extracts a field from each array element using JSONPath
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractOp {
	/// JSONPath to extract from each element
	pub path: String,
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
		if let ScatterTarget::Tool(tool_ref) = target {
			assert_eq!(tool_ref.tool, "my_tool");
			assert!(tool_ref.server.is_none());
		}
	}

	#[test]
	fn test_parse_scatter_target_tool_with_server() {
		let json = r#"{ "tool": "get_entity", "server": "entity-service" }"#;
		let target: ScatterTarget = serde_json::from_str(json).unwrap();
		assert!(matches!(target, ScatterTarget::Tool(_)));
		if let ScatterTarget::Tool(tool_ref) = target {
			assert_eq!(tool_ref.tool, "get_entity");
			assert_eq!(tool_ref.server, Some("entity-service".to_string()));
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
