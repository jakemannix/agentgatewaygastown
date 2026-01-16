// Pattern type definitions for tool compositions
//
// These types correspond to the registry.proto schema and are used
// for deserializing composition definitions from JSON.

mod filter;
mod map_each;
mod pipeline;
mod scatter_gather;
mod schema_map;
mod stateful;
mod vision;

pub use filter::{FieldPredicate, FilterSpec, PredicateValue};
pub use map_each::{MapEachInner, MapEachSpec};
pub use pipeline::{
	ConstructBinding, DataBinding, InputBinding, PipelineSpec, PipelineStep, StepBinding,
	StepOperation, ToolCall,
};
pub use scatter_gather::{
	AggregationOp, AggregationStrategy, DedupeOp, LimitOp, ScatterGatherSpec, ScatterTarget, SortOp,
};
pub use schema_map::{
	CoalesceSource, ConcatSource, FieldSource, LiteralValue, SchemaMapSpec, TemplateSource,
};
pub use stateful::{
	BackoffStrategy, CacheSpec, CircuitBreakerSpec, ClaimCheckSpec, DeadLetterSpec,
	ExponentialBackoff, FixedBackoff, IdempotentSpec, LinearBackoff, OnDuplicate, OnExceeded,
	RetrySpec, SagaSpec, SagaStep, ThrottleSpec, ThrottleStrategy, TimeoutSpec,
};
pub use vision::{
	CapabilityRouterSpec, ConfidenceAggregatorSpec, ConfidenceStrategy, DedupKeepStrategy,
	EnrichmentSource, EnricherSpec, MergeStrategy, RecipientListSpec, RouteCase, RouterSpec,
	SemanticDedupSpec, TapPoint, TapTarget, WeightedSource, WireTapSpec,
};

use serde::{Deserialize, Serialize};

/// PatternSpec defines a composition pattern
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PatternSpec {
	// Stateless patterns (implemented)
	/// Sequential execution of steps
	Pipeline(PipelineSpec),

	/// Parallel fan-out with aggregation
	ScatterGather(ScatterGatherSpec),

	/// Filter array elements by predicate
	Filter(FilterSpec),

	/// Transform fields using mappings
	SchemaMap(SchemaMapSpec),

	/// Apply operation to each array element
	MapEach(MapEachSpec),

	// Stateful patterns (IR defined, runtime not yet implemented)
	/// Retry with configurable backoff
	Retry(RetrySpec),

	/// Enforce maximum execution duration
	Timeout(TimeoutSpec),

	/// Read-through caching with TTL
	Cache(CacheSpec),

	/// Prevent duplicate processing
	Idempotent(IdempotentSpec),

	/// Fail fast with automatic recovery
	CircuitBreaker(CircuitBreakerSpec),

	/// Capture failures for later processing
	DeadLetter(DeadLetterSpec),

	/// Distributed transaction with compensation
	Saga(SagaSpec),

	/// Externalize large payloads
	ClaimCheck(ClaimCheckSpec),

	/// Rate limiting for tool invocations
	Throttle(ThrottleSpec),

	// Vision patterns (advanced routing, enrichment, aggregation)
	/// Content-based routing to different tools
	Router(RouterSpec),

	/// Augment input with parallel enrichments
	Enricher(EnricherSpec),

	/// Fire-and-forget side channel taps
	WireTap(WireTapSpec),

	/// Dynamic recipient list from input data
	RecipientList(RecipientListSpec),

	/// Route based on tool capabilities (MCP-specific)
	CapabilityRouter(CapabilityRouterSpec),

	/// Semantic similarity-based deduplication
	SemanticDedup(SemanticDedupSpec),

	/// Confidence-weighted aggregation
	ConfidenceAggregator(ConfidenceAggregatorSpec),
}

impl PatternSpec {
	/// Get the names of tools referenced by this pattern
	pub fn referenced_tools(&self) -> Vec<&str> {
		match self {
			// Stateless patterns
			PatternSpec::Pipeline(p) => p.referenced_tools(),
			PatternSpec::ScatterGather(sg) => sg.referenced_tools(),
			PatternSpec::Filter(_) => vec![],
			PatternSpec::SchemaMap(_) => vec![],
			PatternSpec::MapEach(me) => me.referenced_tools(),
			// Stateful patterns - return empty for now as they're not executed
			PatternSpec::Retry(_) => vec![],
			PatternSpec::Timeout(_) => vec![],
			PatternSpec::Cache(_) => vec![],
			PatternSpec::Idempotent(_) => vec![],
			PatternSpec::CircuitBreaker(_) => vec![],
			PatternSpec::DeadLetter(_) => vec![],
			PatternSpec::Saga(_) => vec![],
			PatternSpec::ClaimCheck(_) => vec![],
			PatternSpec::Throttle(_) => vec![],
			// Vision patterns - include referenced tools for validation
			PatternSpec::Router(r) => r.referenced_tools(),
			PatternSpec::Enricher(e) => e.referenced_tools(),
			PatternSpec::WireTap(w) => w.referenced_tools(),
			PatternSpec::RecipientList(rl) => rl.referenced_tools(),
			PatternSpec::CapabilityRouter(cr) => cr.referenced_tools(),
			PatternSpec::SemanticDedup(sd) => sd.referenced_tools(),
			PatternSpec::ConfidenceAggregator(ca) => ca.referenced_tools(),
		}
	}

	/// Returns true if this is a stateful or vision pattern that is not yet implemented
	pub fn is_stateful_unimplemented(&self) -> bool {
		matches!(
			self,
			// Stateful patterns
			PatternSpec::Retry(_)
				| PatternSpec::Timeout(_)
				| PatternSpec::Cache(_)
				| PatternSpec::Idempotent(_)
				| PatternSpec::CircuitBreaker(_)
				| PatternSpec::DeadLetter(_)
				| PatternSpec::Saga(_)
				| PatternSpec::ClaimCheck(_)
				| PatternSpec::Throttle(_)
				// Vision patterns
				| PatternSpec::Router(_)
				| PatternSpec::Enricher(_)
				| PatternSpec::WireTap(_)
				| PatternSpec::RecipientList(_)
				| PatternSpec::CapabilityRouter(_)
				| PatternSpec::SemanticDedup(_)
				| PatternSpec::ConfidenceAggregator(_)
		)
	}

	/// Get the pattern name for error messages
	pub fn pattern_name(&self) -> &'static str {
		match self {
			PatternSpec::Pipeline(_) => "pipeline",
			PatternSpec::ScatterGather(_) => "scatter_gather",
			PatternSpec::Filter(_) => "filter",
			PatternSpec::SchemaMap(_) => "schema_map",
			PatternSpec::MapEach(_) => "map_each",
			PatternSpec::Retry(_) => "retry",
			PatternSpec::Timeout(_) => "timeout",
			PatternSpec::Cache(_) => "cache",
			PatternSpec::Idempotent(_) => "idempotent",
			PatternSpec::CircuitBreaker(_) => "circuit_breaker",
			PatternSpec::DeadLetter(_) => "dead_letter",
			PatternSpec::Saga(_) => "saga",
			PatternSpec::ClaimCheck(_) => "claim_check",
			PatternSpec::Throttle(_) => "throttle",
			PatternSpec::Router(_) => "router",
			PatternSpec::Enricher(_) => "enricher",
			PatternSpec::WireTap(_) => "wire_tap",
			PatternSpec::RecipientList(_) => "recipient_list",
			PatternSpec::CapabilityRouter(_) => "capability_router",
			PatternSpec::SemanticDedup(_) => "semantic_dedup",
			PatternSpec::ConfidenceAggregator(_) => "confidence_aggregator",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_pipeline_pattern() {
		let json = r#"{
			"pipeline": {
				"steps": [
					{
						"id": "step1",
						"operation": { "tool": { "name": "search" } },
						"input": { "input": { "path": "$" } }
					}
				]
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::Pipeline(_)));
	}

	#[test]
	fn test_parse_scatter_gather_pattern() {
		let json = r#"{
			"scatterGather": {
				"targets": [
					{ "tool": "search1" },
					{ "tool": "search2" }
				],
				"aggregation": {
					"ops": [
						{ "flatten": true }
					]
				}
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::ScatterGather(_)));
	}

	#[test]
	fn test_parse_filter_pattern() {
		let json = r#"{
			"filter": {
				"predicate": {
					"field": "$.score",
					"op": "gt",
					"value": { "numberValue": 0.5 }
				}
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::Filter(_)));
	}

	#[test]
	fn test_parse_schema_map_pattern() {
		let json = r#"{
			"schemaMap": {
				"mappings": {
					"title": { "path": "$.name" },
					"source": { "literal": { "stringValue": "web" } }
				}
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::SchemaMap(_)));
	}

	#[test]
	fn test_parse_map_each_pattern() {
		let json = r#"{
			"mapEach": {
				"inner": { "tool": "fetch" }
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::MapEach(_)));
	}

	#[test]
	fn test_parse_throttle_pattern() {
		let json = r#"{
			"throttle": {
				"inner": { "tool": { "name": "expensive_api" } },
				"rate": 100,
				"windowMs": 60000,
				"strategy": "sliding_window",
				"onExceeded": "wait"
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::Throttle(_)));
		assert_eq!(spec.pattern_name(), "throttle");
		assert!(spec.is_stateful_unimplemented());
	}

	// Vision pattern tests

	#[test]
	fn test_parse_router_pattern() {
		let json = r#"{
			"router": {
				"routes": [
					{
						"when": {
							"field": "$.type",
							"op": "eq",
							"value": { "stringValue": "pdf" }
						},
						"then": { "tool": { "name": "pdf_processor" } }
					}
				],
				"otherwise": { "tool": { "name": "default_processor" } }
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::Router(_)));
		assert_eq!(spec.pattern_name(), "router");
		assert!(spec.is_stateful_unimplemented());
		assert_eq!(spec.referenced_tools(), vec!["pdf_processor", "default_processor"]);
	}

	#[test]
	fn test_parse_enricher_pattern() {
		let json = r#"{
			"enricher": {
				"enrichments": [
					{
						"field": "metadata",
						"operation": { "tool": { "name": "fetch_metadata" } }
					}
				],
				"merge": "spread",
				"ignoreFailures": true
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::Enricher(_)));
		assert_eq!(spec.pattern_name(), "enricher");
		assert!(spec.is_stateful_unimplemented());
		assert_eq!(spec.referenced_tools(), vec!["fetch_metadata"]);
	}

	#[test]
	fn test_parse_wiretap_pattern() {
		let json = r#"{
			"wireTap": {
				"inner": { "tool": { "name": "main_process" } },
				"taps": [
					{ "tool": "audit_logger" }
				],
				"tapPoint": "after"
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::WireTap(_)));
		assert_eq!(spec.pattern_name(), "wire_tap");
		assert!(spec.is_stateful_unimplemented());
		assert_eq!(spec.referenced_tools(), vec!["main_process", "audit_logger"]);
	}

	#[test]
	fn test_parse_recipient_list_pattern() {
		let json = r#"{
			"recipientList": {
				"recipientsPath": "$.targets",
				"parallel": true,
				"failOnError": false
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::RecipientList(_)));
		assert_eq!(spec.pattern_name(), "recipient_list");
		assert!(spec.is_stateful_unimplemented());
	}

	#[test]
	fn test_parse_capability_router_pattern() {
		let json = r#"{
			"capabilityRouter": {
				"required": ["text-generation"],
				"preferred": ["low-latency"],
				"fallback": { "tool": { "name": "default_tool" } }
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::CapabilityRouter(_)));
		assert_eq!(spec.pattern_name(), "capability_router");
		assert!(spec.is_stateful_unimplemented());
		assert_eq!(spec.referenced_tools(), vec!["default_tool"]);
	}

	#[test]
	fn test_parse_semantic_dedup_pattern() {
		let json = r#"{
			"semanticDedup": {
				"embedder": "text_embedder",
				"contentPath": "$.content",
				"threshold": 0.9,
				"keep": "first"
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::SemanticDedup(_)));
		assert_eq!(spec.pattern_name(), "semantic_dedup");
		assert!(spec.is_stateful_unimplemented());
		assert_eq!(spec.referenced_tools(), vec!["text_embedder"]);
	}

	#[test]
	fn test_parse_confidence_aggregator_pattern() {
		let json = r#"{
			"confidenceAggregator": {
				"sources": [
					{
						"operation": { "tool": { "name": "expert_api" } },
						"weight": 0.9
					},
					{
						"operation": { "tool": { "name": "fallback_api" } },
						"weight": 0.5
					}
				],
				"strategy": "weighted_vote",
				"minWeight": 0.7
			}
		}"#;

		let spec: PatternSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(spec, PatternSpec::ConfidenceAggregator(_)));
		assert_eq!(spec.pattern_name(), "confidence_aggregator");
		assert!(spec.is_stateful_unimplemented());
		assert_eq!(spec.referenced_tools(), vec!["expert_api", "fallback_api"]);
	}
}
