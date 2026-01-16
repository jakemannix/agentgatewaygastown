// Vision pattern type definitions for tool compositions
//
// These patterns are "Vision Patterns" - advanced routing, enrichment, and
// aggregation patterns inspired by Enterprise Integration Patterns. They
// enable sophisticated content-based routing, parallel enrichment, observability
// taps, and intelligent result aggregation.

use super::{AggregationStrategy, DataBinding, FieldPredicate, SchemaMapSpec, StepOperation};
use serde::{Deserialize, Serialize};

// =============================================================================
// ContentRouter Pattern (Choice/Switch)
// =============================================================================

/// RouterSpec - content-based routing to different tools based on predicates
///
/// Routes input to different operations based on content predicates. Evaluates
/// conditions in order and executes the first matching route. Provides an
/// optional default route for unmatched inputs.
///
/// **DSL Example:**
/// ```typescript
/// route()
///   .when(field('$.type').eq('pdf')).then('pdf_processor')
///   .when(field('$.type').eq('csv')).then('csv_processor')
///   .otherwise('generic_processor')
///   .build();
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterSpec {
	/// Ordered list of route conditions
	pub routes: Vec<RouteCase>,

	/// Default route if no conditions match
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub otherwise: Option<Box<StepOperation>>,
}

impl RouterSpec {
	/// Get the names of tools referenced by this router
	pub fn referenced_tools(&self) -> Vec<&str> {
		let mut refs: Vec<&str> = self
			.routes
			.iter()
			.flat_map(|r| r.then.referenced_tools())
			.collect();

		if let Some(ref otherwise) = self.otherwise {
			refs.extend(otherwise.referenced_tools());
		}

		refs
	}
}

/// A single route case with predicate and target operation
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteCase {
	/// Predicate to evaluate
	pub when: FieldPredicate,

	/// Operation to execute if predicate matches
	pub then: StepOperation,
}

// =============================================================================
// Enricher Pattern
// =============================================================================

/// EnricherSpec - augment input with results from parallel enrichment calls
///
/// Runs multiple enrichment operations in parallel and merges their results
/// with the original input. Supports various merge strategies and can handle
/// enrichment failures gracefully.
///
/// **DSL Example:**
/// ```typescript
/// enrich()
///   .field('history', 'crm.get_history', { input: '$.customer_id' })
///   .field('web_presence', 'web_search', { input: '$.company_name' })
///   .field('sentiment', 'analyze_sentiment', { input: '$.last_email' })
///   .merge('spread')
///   .build();
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnricherSpec {
	/// Enrichment operations to run in parallel
	pub enrichments: Vec<EnrichmentSource>,

	/// How to merge enrichments with original input
	pub merge: MergeStrategy,

	/// Continue on enrichment failure?
	#[serde(default)]
	pub ignore_failures: bool,

	/// Timeout for enrichment calls in milliseconds
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub timeout_ms: Option<u32>,
}

impl EnricherSpec {
	/// Get the names of tools referenced by this enricher
	pub fn referenced_tools(&self) -> Vec<&str> {
		self.enrichments
			.iter()
			.flat_map(|e| e.operation.referenced_tools())
			.collect()
	}
}

/// A single enrichment source
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentSource {
	/// Field name for this enrichment in result
	pub field: String,

	/// Operation to get enrichment data
	pub operation: StepOperation,

	/// Input binding for this enrichment
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub input: Option<DataBinding>,
}

/// Strategy for merging enrichment results with original input
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MergeStrategy {
	/// Spread enrichments into root object ($.field1, $.field2, etc.)
	Spread,

	/// Put enrichments under a key ($.enrichments.field1, $.enrichments.field2, etc.)
	Nested { key: String },

	/// Custom schema map for fine-grained control
	SchemaMap(SchemaMapSpec),
}

impl Default for MergeStrategy {
	fn default() -> Self {
		MergeStrategy::Spread
	}
}

// =============================================================================
// WireTap Pattern
// =============================================================================

/// WireTapSpec - send copies of data to side channels without affecting main flow
///
/// Allows observability and auditing by tapping data flow at various points.
/// Taps are fire-and-forget and don't affect the main execution path.
///
/// **DSL Example:**
/// ```typescript
/// wireTap()
///   .inner(pipeline().step('process').step('store').build())
///   .tap('audit_logger', { point: 'after' })
///   .tap('metrics_collector', { point: 'both' })
///   .build();
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireTapSpec {
	/// Main operation
	pub inner: Box<StepOperation>,

	/// Tap targets (fire-and-forget)
	pub taps: Vec<TapTarget>,

	/// When to tap: before, after, or both
	#[serde(default)]
	pub tap_point: TapPoint,
}

impl WireTapSpec {
	/// Get the names of tools referenced by this wiretap
	pub fn referenced_tools(&self) -> Vec<&str> {
		let mut refs = self.inner.referenced_tools();
		refs.extend(self.taps.iter().map(|t| t.tool.as_str()));
		refs
	}
}

/// A tap target for side-channel data
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TapTarget {
	/// Tool to send tap data to
	pub tool: String,

	/// Transform input before sending to tap
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub transform: Option<SchemaMapSpec>,
}

/// When to tap data in the flow
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TapPoint {
	/// Tap before inner operation executes
	Before,

	/// Tap after inner operation completes (with result)
	#[default]
	After,

	/// Tap both before and after
	Both,
}

// =============================================================================
// RecipientList Pattern (Dynamic Routing)
// =============================================================================

/// RecipientListSpec - dynamically determine targets at runtime
///
/// Unlike static scatter-gather, the recipient list is determined from the
/// input data or by calling a tool at runtime. Supports parallel execution
/// and various aggregation strategies.
///
/// **DSL Example:**
/// ```typescript
/// recipientList()
///   .recipientsPath('$.notificationChannels')
///   .parallel(true)
///   .aggregation({ ops: [{ flatten: true }] })
///   .build();
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipientListSpec {
	/// JSONPath to list of tool names in input
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub recipients_path: Option<String>,

	/// Alternatively, a tool that returns recipient list
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub recipients_tool: Option<String>,

	/// Aggregation strategy for results
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub aggregation: Option<AggregationStrategy>,

	/// Execute in parallel?
	#[serde(default = "default_true")]
	pub parallel: bool,

	/// Fail if any recipient fails?
	#[serde(default)]
	pub fail_on_error: bool,
}

impl RecipientListSpec {
	/// Get the names of tools referenced by this recipient list
	/// Note: Actual recipients are determined at runtime
	pub fn referenced_tools(&self) -> Vec<&str> {
		self.recipients_tool
			.as_ref()
			.map(|t| vec![t.as_str()])
			.unwrap_or_default()
	}
}

fn default_true() -> bool {
	true
}

// =============================================================================
// CapabilityRouter Pattern (MCP-Specific)
// =============================================================================

/// CapabilityRouterSpec - route based on tool capabilities
///
/// An MCP-specific pattern that routes to tools based on their declared
/// capabilities. Requires registry introspection to discover available tools
/// and their capability annotations.
///
/// **DSL Example:**
/// ```typescript
/// capabilityRoute()
///   .required(['text-generation', 'streaming'])
///   .preferred(['low-latency'])
///   .fallback(tool('default_generator'))
///   .build();
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRouterSpec {
	/// Required capabilities (tool must have all)
	pub required: Vec<String>,

	/// Preferred capabilities (for ranking when multiple tools match)
	#[serde(default)]
	pub preferred: Vec<String>,

	/// Fallback if no matching tool found
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub fallback: Option<Box<StepOperation>>,
}

impl CapabilityRouterSpec {
	/// Get the names of tools referenced by this capability router
	/// Note: Main tools are discovered at runtime via capability matching
	pub fn referenced_tools(&self) -> Vec<&str> {
		self.fallback
			.as_ref()
			.map(|f| f.referenced_tools())
			.unwrap_or_default()
	}
}

// =============================================================================
// SemanticDedup Pattern
// =============================================================================

/// SemanticDedupSpec - deduplicate based on semantic similarity
///
/// Uses an embedding service to deduplicate results based on semantic
/// similarity rather than exact field matching. Useful for aggregating
/// search results from multiple sources.
///
/// **DSL Example:**
/// ```typescript
/// semanticDedup()
///   .embedder('text_embedder')
///   .contentPath('$.content')
///   .threshold(0.95)
///   .keep('highest_score')
///   .build();
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticDedupSpec {
	/// Embedding tool/service
	pub embedder: String,

	/// JSONPath to field to embed for similarity comparison
	pub content_path: String,

	/// Similarity threshold (0.0 - 1.0)
	pub threshold: f32,

	/// Strategy for choosing representative item
	#[serde(default)]
	pub keep: DedupKeepStrategy,
}

impl SemanticDedupSpec {
	/// Get the names of tools referenced by this semantic dedup
	pub fn referenced_tools(&self) -> Vec<&str> {
		vec![self.embedder.as_str()]
	}
}

/// Strategy for choosing which item to keep when duplicates are found
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DedupKeepStrategy {
	/// Keep the first occurrence
	#[default]
	First,

	/// Keep the last occurrence
	Last,

	/// Keep the one with highest score field
	HighestScore,

	/// Keep the one with most populated fields
	MostComplete,
}

// =============================================================================
// ConfidenceAggregator Pattern
// =============================================================================

/// ConfidenceAggregatorSpec - weighted aggregation based on source reliability
///
/// Aggregates results from multiple sources with different reliability weights.
/// Can detect conflicts between high-weight sources and flag them.
///
/// **DSL Example:**
/// ```typescript
/// confidenceAggregate()
///   .source(tool('expert_api'), { weight: 0.9 })
///   .source(tool('general_search'), { weight: 0.5 })
///   .source(tool('fallback_api'), { weight: 0.3 })
///   .strategy('weighted_vote')
///   .minWeight(0.7)
///   .conflictThreshold(0.8)
///   .build();
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceAggregatorSpec {
	/// Weighted sources
	pub sources: Vec<WeightedSource>,

	/// Aggregation strategy
	pub strategy: ConfidenceStrategy,

	/// Minimum total weight required for valid result
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub min_weight: Option<f32>,

	/// Flag if sources with this combined weight disagree
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub conflict_threshold: Option<f32>,
}

impl ConfidenceAggregatorSpec {
	/// Get the names of tools referenced by this confidence aggregator
	pub fn referenced_tools(&self) -> Vec<&str> {
		self.sources
			.iter()
			.flat_map(|s| s.operation.referenced_tools())
			.collect()
	}
}

/// A source with a confidence/reliability weight
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeightedSource {
	/// Operation to get data from this source
	pub operation: StepOperation,

	/// Weight representing source reliability (0.0 - 1.0)
	pub weight: f32,
}

/// Strategy for aggregating confidence-weighted results
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceStrategy {
	/// Use highest-weight source that returns a result
	#[default]
	HighestWeight,

	/// Weighted voting for consensus
	WeightedVote,

	/// Require quorum of weighted sources
	Quorum,

	/// Take all results, annotated with weights
	All,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_router_spec() {
		let json = r#"{
            "routes": [
                {
                    "when": {
                        "field": "$.type",
                        "op": "eq",
                        "value": { "stringValue": "pdf" }
                    },
                    "then": { "tool": { "name": "pdf_processor" } }
                },
                {
                    "when": {
                        "field": "$.type",
                        "op": "eq",
                        "value": { "stringValue": "csv" }
                    },
                    "then": { "tool": { "name": "csv_processor" } }
                }
            ],
            "otherwise": { "tool": { "name": "generic_processor" } }
        }"#;

		let spec: RouterSpec = serde_json::from_str(json).unwrap();
		assert_eq!(spec.routes.len(), 2);
		assert!(spec.otherwise.is_some());
		assert_eq!(
			spec.referenced_tools(),
			vec!["pdf_processor", "csv_processor", "generic_processor"]
		);
	}

	#[test]
	fn test_parse_enricher_spec() {
		let json = r#"{
            "enrichments": [
                {
                    "field": "history",
                    "operation": { "tool": { "name": "crm.get_history" } },
                    "input": { "input": { "path": "$.customer_id" } }
                },
                {
                    "field": "sentiment",
                    "operation": { "tool": { "name": "analyze_sentiment" } }
                }
            ],
            "merge": "spread",
            "ignoreFailures": true,
            "timeoutMs": 5000
        }"#;

		let spec: EnricherSpec = serde_json::from_str(json).unwrap();
		assert_eq!(spec.enrichments.len(), 2);
		assert!(matches!(spec.merge, MergeStrategy::Spread));
		assert!(spec.ignore_failures);
		assert_eq!(spec.timeout_ms, Some(5000));
		assert_eq!(
			spec.referenced_tools(),
			vec!["crm.get_history", "analyze_sentiment"]
		);
	}

	#[test]
	fn test_parse_enricher_nested_merge() {
		let json = r#"{
            "enrichments": [
                {
                    "field": "data",
                    "operation": { "tool": { "name": "fetcher" } }
                }
            ],
            "merge": { "nested": { "key": "enrichments" } }
        }"#;

		let spec: EnricherSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec.merge, MergeStrategy::Nested { key } if key == "enrichments"));
	}

	#[test]
	fn test_parse_wiretap_spec() {
		let json = r#"{
            "inner": { "tool": { "name": "main_processor" } },
            "taps": [
                { "tool": "audit_logger" },
                { "tool": "metrics_collector", "transform": { "mappings": { "event": { "path": "$" } } } }
            ],
            "tapPoint": "both"
        }"#;

		let spec: WireTapSpec = serde_json::from_str(json).unwrap();
		assert_eq!(spec.taps.len(), 2);
		assert_eq!(spec.tap_point, TapPoint::Both);
		assert_eq!(
			spec.referenced_tools(),
			vec!["main_processor", "audit_logger", "metrics_collector"]
		);
	}

	#[test]
	fn test_parse_recipient_list_spec() {
		let json = r#"{
            "recipientsPath": "$.notificationChannels",
            "parallel": true,
            "failOnError": false
        }"#;

		let spec: RecipientListSpec = serde_json::from_str(json).unwrap();
		assert_eq!(
			spec.recipients_path,
			Some("$.notificationChannels".to_string())
		);
		assert!(spec.parallel);
		assert!(!spec.fail_on_error);
	}

	#[test]
	fn test_parse_recipient_list_with_tool() {
		let json = r#"{
            "recipientsTool": "get_recipients",
            "parallel": false
        }"#;

		let spec: RecipientListSpec = serde_json::from_str(json).unwrap();
		assert_eq!(spec.recipients_tool, Some("get_recipients".to_string()));
		assert!(!spec.parallel);
		assert_eq!(spec.referenced_tools(), vec!["get_recipients"]);
	}

	#[test]
	fn test_parse_capability_router_spec() {
		let json = r#"{
            "required": ["text-generation", "streaming"],
            "preferred": ["low-latency"],
            "fallback": { "tool": { "name": "default_generator" } }
        }"#;

		let spec: CapabilityRouterSpec = serde_json::from_str(json).unwrap();
		assert_eq!(spec.required, vec!["text-generation", "streaming"]);
		assert_eq!(spec.preferred, vec!["low-latency"]);
		assert!(spec.fallback.is_some());
		assert_eq!(spec.referenced_tools(), vec!["default_generator"]);
	}

	#[test]
	fn test_parse_semantic_dedup_spec() {
		let json = r#"{
            "embedder": "text_embedder",
            "contentPath": "$.content",
            "threshold": 0.95,
            "keep": "highest_score"
        }"#;

		let spec: SemanticDedupSpec = serde_json::from_str(json).unwrap();
		assert_eq!(spec.embedder, "text_embedder");
		assert_eq!(spec.content_path, "$.content");
		assert!((spec.threshold - 0.95).abs() < f32::EPSILON);
		assert_eq!(spec.keep, DedupKeepStrategy::HighestScore);
		assert_eq!(spec.referenced_tools(), vec!["text_embedder"]);
	}

	#[test]
	fn test_semantic_dedup_keep_strategies() {
		for (keep_str, expected) in [
			("first", DedupKeepStrategy::First),
			("last", DedupKeepStrategy::Last),
			("highest_score", DedupKeepStrategy::HighestScore),
			("most_complete", DedupKeepStrategy::MostComplete),
		] {
			let json = format!(
				r#"{{
                    "embedder": "test",
                    "contentPath": "$.text",
                    "threshold": 0.9,
                    "keep": "{}"
                }}"#,
				keep_str
			);
			let spec: SemanticDedupSpec = serde_json::from_str(&json).unwrap();
			assert_eq!(spec.keep, expected);
		}
	}

	#[test]
	fn test_parse_confidence_aggregator_spec() {
		let json = r#"{
            "sources": [
                {
                    "operation": { "tool": { "name": "expert_api" } },
                    "weight": 0.9
                },
                {
                    "operation": { "tool": { "name": "general_search" } },
                    "weight": 0.5
                }
            ],
            "strategy": "weighted_vote",
            "minWeight": 0.7,
            "conflictThreshold": 0.8
        }"#;

		let spec: ConfidenceAggregatorSpec = serde_json::from_str(json).unwrap();
		assert_eq!(spec.sources.len(), 2);
		assert_eq!(spec.strategy, ConfidenceStrategy::WeightedVote);
		assert_eq!(spec.min_weight, Some(0.7));
		assert_eq!(spec.conflict_threshold, Some(0.8));
		assert_eq!(spec.referenced_tools(), vec!["expert_api", "general_search"]);
	}

	#[test]
	fn test_confidence_strategies() {
		for (strategy_str, expected) in [
			("highest_weight", ConfidenceStrategy::HighestWeight),
			("weighted_vote", ConfidenceStrategy::WeightedVote),
			("quorum", ConfidenceStrategy::Quorum),
			("all", ConfidenceStrategy::All),
		] {
			let json = format!(
				r#"{{
                    "sources": [
                        {{ "operation": {{ "tool": {{ "name": "test" }} }}, "weight": 1.0 }}
                    ],
                    "strategy": "{}"
                }}"#,
				strategy_str
			);
			let spec: ConfidenceAggregatorSpec = serde_json::from_str(&json).unwrap();
			assert_eq!(spec.strategy, expected);
		}
	}

	#[test]
	fn test_tap_points() {
		for (point_str, expected) in [
			("before", TapPoint::Before),
			("after", TapPoint::After),
			("both", TapPoint::Both),
		] {
			let json = format!(
				r#"{{
                    "inner": {{ "tool": {{ "name": "test" }} }},
                    "taps": [{{ "tool": "logger" }}],
                    "tapPoint": "{}"
                }}"#,
				point_str
			);
			let spec: WireTapSpec = serde_json::from_str(&json).unwrap();
			assert_eq!(spec.tap_point, expected);
		}
	}

	#[test]
	fn test_merge_strategy_schema_map() {
		let json = r#"{
            "enrichments": [
                {
                    "field": "data",
                    "operation": { "tool": { "name": "fetcher" } }
                }
            ],
            "merge": {
                "schemaMap": {
                    "mappings": {
                        "result": { "path": "$.data" }
                    }
                }
            }
        }"#;

		let spec: EnricherSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec.merge, MergeStrategy::SchemaMap(_)));
	}
}
