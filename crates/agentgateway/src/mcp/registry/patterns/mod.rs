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

pub use filter::{FieldPredicate, FilterSpec, PredicateValue};
pub use map_each::{MapEachInner, MapEachSpec};
pub use pipeline::{ConstructBinding, DataBinding, InputBinding, PipelineSpec, PipelineStep, StepBinding, StepOperation, ToolCall};
pub use scatter_gather::{
	AggregationOp, AggregationStrategy, DedupeOp, LimitOp, ScatterGatherSpec, ScatterTarget, SortOp,
};
pub use schema_map::{CoalesceSource, ConcatSource, FieldSource, LiteralValue, SchemaMapSpec, TemplateSource};
pub use stateful::{
	BackoffStrategy, CacheSpec, CircuitBreakerSpec, ClaimCheckSpec, DeadLetterSpec, ExponentialBackoff,
	FixedBackoff, IdempotentSpec, LinearBackoff, OnDuplicate, RetrySpec, SagaSpec, SagaStep, TimeoutSpec,
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
		}
	}

	/// Returns true if this is a stateful pattern that is not yet implemented
	pub fn is_stateful_unimplemented(&self) -> bool {
		matches!(
			self,
			PatternSpec::Retry(_)
				| PatternSpec::Timeout(_)
				| PatternSpec::Cache(_)
				| PatternSpec::Idempotent(_)
				| PatternSpec::CircuitBreaker(_)
				| PatternSpec::DeadLetter(_)
				| PatternSpec::Saga(_)
				| PatternSpec::ClaimCheck(_)
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
}

