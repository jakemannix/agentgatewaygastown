//! Compatibility layer for converting between proto-generated and hand-written types
//!
//! This module provides `From` implementations to convert from proto types (used for
//! canonical JSON parsing) to hand-written types (used for runtime with builder methods).
//!
//! The proto types are generated from registry.proto and use proto3 JSON format.
//! The hand-written types have additional methods and builder patterns.
//!
//! Migration strategy:
//! 1. Parse JSON with proto types (canonical format)
//! 2. Convert to hand-written types for runtime
//! 3. Eventually merge the two type systems

use std::collections::HashMap;

use crate::types::proto::registry as proto;

use super::patterns::{
	AggregationOp, AggregationStrategy, BackoffStrategy, CacheSpec, CircuitBreakerSpec,
	ClaimCheckSpec, CoalesceSource, ConcatSource, ConstructBinding, DataBinding, DeadLetterSpec,
	DedupeOp, ExponentialBackoff, FieldPredicate, FieldSource, FilterSpec, FixedBackoff,
	IdempotentSpec, InputBinding, LimitOp, LinearBackoff, LiteralValue, MapEachInner, MapEachSpec,
	OnDuplicate, PatternSpec, PipelineSpec, PipelineStep, PredicateValue, RetrySpec, SagaSpec,
	SagaStep, ScatterGatherSpec, ScatterTarget, SchemaMapSpec, SortOp, StepBinding, StepOperation,
	TemplateSource, TimeoutSpec, ToolCall,
};
use super::types::{
	AgentDefinition, AgentSkillDefinition, OutputTransform, Registry, Schema, Server, SourceTool,
	ToolDefinition, ToolImplementation, ToolProvision, UnknownCallerPolicy,
};

// =============================================================================
// Proto Value to serde_json::Value conversion
// =============================================================================

/// Convert a prost_wkt_types::Value to serde_json::Value
fn proto_value_to_json(v: &prost_wkt_types::Value) -> serde_json::Value {
	use prost_wkt_types::value::Kind;
	match &v.kind {
		Some(Kind::NullValue(_)) => serde_json::Value::Null,
		Some(Kind::BoolValue(b)) => serde_json::Value::Bool(*b),
		Some(Kind::NumberValue(n)) => {
			serde_json::Number::from_f64(*n)
				.map(serde_json::Value::Number)
				.unwrap_or(serde_json::Value::Null)
		},
		Some(Kind::StringValue(s)) => serde_json::Value::String(s.clone()),
		Some(Kind::ListValue(list)) => {
			serde_json::Value::Array(list.values.iter().map(proto_value_to_json).collect())
		},
		Some(Kind::StructValue(s)) => proto_struct_to_json(s),
		None => serde_json::Value::Null,
	}
}

/// Convert a prost_wkt_types::Struct to serde_json::Value
fn proto_struct_to_json(s: &prost_wkt_types::Struct) -> serde_json::Value {
	let map: serde_json::Map<String, serde_json::Value> = s
		.fields
		.iter()
		.map(|(k, v)| (k.clone(), proto_value_to_json(v)))
		.collect();
	serde_json::Value::Object(map)
}

/// Convert metadata map from proto to JSON
fn convert_metadata(
	metadata: &HashMap<String, prost_wkt_types::Value>,
) -> HashMap<String, serde_json::Value> {
	metadata
		.iter()
		.map(|(k, v)| (k.clone(), proto_value_to_json(v)))
		.collect()
}

/// Convert defaults map from proto to JSON
fn convert_defaults(
	defaults: &HashMap<String, prost_wkt_types::Value>,
) -> HashMap<String, serde_json::Value> {
	convert_metadata(defaults)
}

// =============================================================================
// Registry Conversion
// =============================================================================

impl From<proto::Registry> for Registry {
	fn from(p: proto::Registry) -> Self {
		Registry {
			schema_version: p.schema_version,
			tools: p.tools.into_iter().map(Into::into).collect(),
			schemas: p.schemas.into_iter().map(Into::into).collect(),
			servers: p.servers.into_iter().map(Into::into).collect(),
			agents: p.agents.into_iter().map(Into::into).collect(),
			// Proto doesn't have this field yet, default to AllowAll
			unknown_caller_policy: UnknownCallerPolicy::default(),
			metadata: HashMap::new(),
		}
	}
}

// =============================================================================
// Schema Conversion
// =============================================================================

impl From<proto::SchemaDefinition> for Schema {
	fn from(p: proto::SchemaDefinition) -> Self {
		Schema {
			name: p.name,
			version: p.version,
			description: p.description,
			schema: p.schema.map(|s| proto_struct_to_json(&s)).unwrap_or(serde_json::Value::Null),
			metadata: convert_metadata(&p.metadata),
		}
	}
}

// =============================================================================
// Server Conversion
// =============================================================================

impl From<proto::ServerDefinition> for Server {
	fn from(p: proto::ServerDefinition) -> Self {
		Server {
			name: p.name,
			version: if p.version.is_empty() { None } else { Some(p.version) },
			description: p.description,
			provides: p.provided_tools.into_iter().map(Into::into).collect(),
			deprecated: false, // Proto doesn't have this yet
			deprecation_message: None,
			metadata: convert_metadata(&p.metadata),
		}
	}
}

impl From<proto::ServerTool> for ToolProvision {
	fn from(p: proto::ServerTool) -> Self {
		ToolProvision {
			tool: p.name,
			version: None, // Proto has schema refs, not version
		}
	}
}

// =============================================================================
// Agent Conversion
// =============================================================================

impl From<proto::AgentDefinition> for AgentDefinition {
	fn from(p: proto::AgentDefinition) -> Self {
		// Extract URL from endpoint if present
		let url = p.endpoint.and_then(|ep| {
			ep.transport.and_then(|t| match t {
				proto::agent_endpoint::Transport::A2a(a2a) => Some(a2a.url),
				proto::agent_endpoint::Transport::Mcp(_) => None,
			})
		});

		AgentDefinition {
			name: p.name,
			version: if p.version.is_empty() { None } else { Some(p.version) },
			description: p.description,
			url,
			protocol_version: None, // Not in proto
			default_input_modes: Vec::new(),
			default_output_modes: Vec::new(),
			skills: p.skills.into_iter().map(Into::into).collect(),
			capabilities: None, // TODO: Convert from proto capabilities
			provider: None,     // Not in proto
			metadata: convert_metadata(&p.metadata),
		}
	}
}

impl From<proto::AgentSkill> for AgentSkillDefinition {
	fn from(p: proto::AgentSkill) -> Self {
		AgentSkillDefinition {
			id: p.name.clone(), // Use name as id
			name: Some(p.name),
			description: p.description,
			tags: Vec::new(),
			examples: p
				.examples
				.into_iter()
				.filter_map(|e| e.description)
				.collect(),
			input_modes: Vec::new(),
			output_modes: Vec::new(),
			input_schema: None,  // TODO: Convert SchemaRef
			output_schema: None, // TODO: Convert SchemaRef
		}
	}
}

// =============================================================================
// Tool Definition Conversion
// =============================================================================

impl From<proto::ToolDefinition> for ToolDefinition {
	fn from(p: proto::ToolDefinition) -> Self {
		let implementation = match p.implementation {
			Some(proto::tool_definition::Implementation::Source(source)) => {
				ToolImplementation::Source(source.into())
			},
			Some(proto::tool_definition::Implementation::Spec(spec)) => {
				ToolImplementation::Spec(spec.into())
			},
			None => {
				// Default to empty source - this shouldn't happen in practice
				ToolImplementation::Source(SourceTool {
					target: String::new(),
					tool: String::new(),
					defaults: HashMap::new(),
					hide_fields: Vec::new(),
					server_version: None,
				})
			},
		};

		ToolDefinition {
			name: p.name,
			description: p.description,
			implementation,
			input_schema: p.input_schema.map(|s| proto_struct_to_json(&s)),
			output_transform: p.output_transform.map(Into::into),
			output_schema: None, // Not in proto ToolDefinition
			version: p.version,
			metadata: convert_metadata(&p.metadata),
			tags: Vec::new(),
			deprecated: None,
			depends: Vec::new(),
		}
	}
}

impl From<proto::SourceTool> for SourceTool {
	fn from(p: proto::SourceTool) -> Self {
		SourceTool {
			// Proto uses "server", hand-written uses "target" - they're the same concept
			target: p.server,
			tool: p.tool,
			defaults: convert_defaults(&p.defaults),
			hide_fields: p.hide_fields,
			server_version: p.server_version,
		}
	}
}

impl From<proto::OutputTransform> for OutputTransform {
	fn from(p: proto::OutputTransform) -> Self {
		OutputTransform {
			mappings: p
				.mappings
				.into_iter()
				.map(|(k, v)| (k, v.into()))
				.collect(),
		}
	}
}

// =============================================================================
// Pattern Conversion
// =============================================================================

impl From<proto::PatternSpec> for PatternSpec {
	fn from(p: proto::PatternSpec) -> Self {
		match p.pattern {
			Some(proto::pattern_spec::Pattern::Pipeline(spec)) => PatternSpec::Pipeline(spec.into()),
			Some(proto::pattern_spec::Pattern::ScatterGather(spec)) => {
				PatternSpec::ScatterGather(spec.into())
			},
			Some(proto::pattern_spec::Pattern::Filter(spec)) => PatternSpec::Filter(spec.into()),
			Some(proto::pattern_spec::Pattern::SchemaMap(spec)) => PatternSpec::SchemaMap(spec.into()),
			Some(proto::pattern_spec::Pattern::MapEach(spec)) => PatternSpec::MapEach((*spec).into()),
			Some(proto::pattern_spec::Pattern::Retry(spec)) => PatternSpec::Retry((*spec).into()),
			Some(proto::pattern_spec::Pattern::Timeout(spec)) => PatternSpec::Timeout((*spec).into()),
			Some(proto::pattern_spec::Pattern::Cache(spec)) => PatternSpec::Cache((*spec).into()),
			Some(proto::pattern_spec::Pattern::Idempotent(spec)) => {
				PatternSpec::Idempotent((*spec).into())
			},
			Some(proto::pattern_spec::Pattern::CircuitBreaker(spec)) => {
				PatternSpec::CircuitBreaker((*spec).into())
			},
			Some(proto::pattern_spec::Pattern::DeadLetter(spec)) => {
				PatternSpec::DeadLetter((*spec).into())
			},
			Some(proto::pattern_spec::Pattern::Saga(spec)) => PatternSpec::Saga(spec.into()),
			Some(proto::pattern_spec::Pattern::ClaimCheck(spec)) => {
				PatternSpec::ClaimCheck((*spec).into())
			},
			None => {
				// Default to empty pipeline - shouldn't happen in practice
				PatternSpec::Pipeline(PipelineSpec { steps: Vec::new() })
			},
		}
	}
}

// =============================================================================
// Pipeline Pattern Conversion
// =============================================================================

impl From<proto::PipelineSpec> for PipelineSpec {
	fn from(p: proto::PipelineSpec) -> Self {
		PipelineSpec {
			steps: p.steps.into_iter().map(Into::into).collect(),
		}
	}
}

impl From<proto::PipelineStep> for PipelineStep {
	fn from(p: proto::PipelineStep) -> Self {
		PipelineStep {
			id: p.id,
			operation: p
				.operation
				.map(Into::into)
				.unwrap_or_else(|| StepOperation::Tool(ToolCall::new(""))),
			input: p.input.map(Into::into),
		}
	}
}

impl From<proto::StepOperation> for StepOperation {
	fn from(p: proto::StepOperation) -> Self {
		match p.op {
			Some(proto::step_operation::Op::Tool(tool)) => StepOperation::Tool(tool.into()),
			Some(proto::step_operation::Op::Pattern(pattern)) => {
				StepOperation::Pattern(Box::new((*pattern).into()))
			},
			Some(proto::step_operation::Op::Agent(agent)) => {
				StepOperation::Agent(super::patterns::AgentCall {
					name: agent.name,
					skill: agent.skill,
					version: agent.version,
				})
			},
			None => StepOperation::Tool(ToolCall::new("")),
		}
	}
}

impl From<proto::ToolCall> for ToolCall {
	fn from(p: proto::ToolCall) -> Self {
		ToolCall {
			name: p.name,
			server: p.server,
			server_version: p.server_version,
		}
	}
}

impl From<proto::DataBinding> for DataBinding {
	fn from(p: proto::DataBinding) -> Self {
		match p.source {
			Some(proto::data_binding::Source::Input(input)) => DataBinding::Input(input.into()),
			Some(proto::data_binding::Source::Step(step)) => DataBinding::Step(step.into()),
			Some(proto::data_binding::Source::Constant(value)) => {
				DataBinding::Constant(proto_value_to_json(&value))
			},
			Some(proto::data_binding::Source::Construct(construct)) => {
				DataBinding::Construct(construct.into())
			},
			None => DataBinding::Input(InputBinding {
				path: "$".to_string(),
			}),
		}
	}
}

impl From<proto::InputBinding> for InputBinding {
	fn from(p: proto::InputBinding) -> Self {
		InputBinding { path: p.path }
	}
}

impl From<proto::StepBinding> for StepBinding {
	fn from(p: proto::StepBinding) -> Self {
		StepBinding {
			step_id: p.step_id,
			path: p.path,
		}
	}
}

impl From<proto::ConstructBinding> for ConstructBinding {
	fn from(p: proto::ConstructBinding) -> Self {
		ConstructBinding {
			fields: p
				.fields
				.into_iter()
				.map(|(k, v)| (k, v.into()))
				.collect(),
		}
	}
}

// =============================================================================
// Scatter-Gather Pattern Conversion
// =============================================================================

impl From<proto::ScatterGatherSpec> for ScatterGatherSpec {
	fn from(p: proto::ScatterGatherSpec) -> Self {
		ScatterGatherSpec {
			targets: p.targets.into_iter().map(Into::into).collect(),
			aggregation: p.aggregation.map(Into::into).unwrap_or_else(|| AggregationStrategy {
				ops: Vec::new(),
			}),
			timeout_ms: p.timeout_ms,
			fail_fast: p.fail_fast,
		}
	}
}

impl From<proto::ScatterTarget> for ScatterTarget {
	fn from(p: proto::ScatterTarget) -> Self {
		match p.target {
			Some(proto::scatter_target::Target::Tool(name)) => ScatterTarget::Tool(name),
			Some(proto::scatter_target::Target::Pattern(pattern)) => {
				// Pattern is not boxed in proto ScatterTarget
				ScatterTarget::Pattern(Box::new(pattern.into()))
			},
			None => ScatterTarget::Tool(String::new()),
		}
	}
}

impl From<proto::AggregationStrategy> for AggregationStrategy {
	fn from(p: proto::AggregationStrategy) -> Self {
		AggregationStrategy {
			ops: p.ops.into_iter().map(Into::into).collect(),
		}
	}
}

impl From<proto::AggregationOp> for AggregationOp {
	fn from(p: proto::AggregationOp) -> Self {
		match p.op {
			Some(proto::aggregation_op::Op::Flatten(b)) => AggregationOp::Flatten(b),
			Some(proto::aggregation_op::Op::Sort(sort)) => AggregationOp::Sort(sort.into()),
			Some(proto::aggregation_op::Op::Dedupe(dedupe)) => AggregationOp::Dedupe(dedupe.into()),
			Some(proto::aggregation_op::Op::Limit(limit)) => AggregationOp::Limit(limit.into()),
			Some(proto::aggregation_op::Op::Concat(b)) => AggregationOp::Concat(b),
			Some(proto::aggregation_op::Op::Merge(b)) => AggregationOp::Merge(b),
			None => AggregationOp::Flatten(false),
		}
	}
}

impl From<proto::SortOp> for SortOp {
	fn from(p: proto::SortOp) -> Self {
		SortOp {
			field: p.field,
			order: p.order,
		}
	}
}

impl From<proto::DedupeOp> for DedupeOp {
	fn from(p: proto::DedupeOp) -> Self {
		DedupeOp { field: p.field }
	}
}

impl From<proto::LimitOp> for LimitOp {
	fn from(p: proto::LimitOp) -> Self {
		LimitOp { count: p.count }
	}
}

// =============================================================================
// Filter Pattern Conversion
// =============================================================================

impl From<proto::FilterSpec> for FilterSpec {
	fn from(p: proto::FilterSpec) -> Self {
		FilterSpec {
			predicate: p.predicate.map(Into::into).unwrap_or_else(|| FieldPredicate {
				field: String::new(),
				op: "eq".to_string(),
				value: PredicateValue::null(),
			}),
		}
	}
}

impl From<proto::FieldPredicate> for FieldPredicate {
	fn from(p: proto::FieldPredicate) -> Self {
		FieldPredicate {
			field: p.field,
			op: p.op,
			value: p.value.map(Into::into).unwrap_or_else(PredicateValue::null),
		}
	}
}

impl From<proto::PredicateValue> for PredicateValue {
	fn from(p: proto::PredicateValue) -> Self {
		match p.value {
			Some(proto::predicate_value::Value::StringValue(s)) => PredicateValue::StringValue(s),
			Some(proto::predicate_value::Value::NumberValue(n)) => PredicateValue::NumberValue(n),
			Some(proto::predicate_value::Value::BoolValue(b)) => PredicateValue::BoolValue(b),
			Some(proto::predicate_value::Value::NullValue(_)) => PredicateValue::NullValue(true),
			Some(proto::predicate_value::Value::ListValue(list)) => {
				PredicateValue::ListValue(list.values.into_iter().map(Into::into).collect())
			},
			None => PredicateValue::NullValue(true),
		}
	}
}

// =============================================================================
// Schema Map Pattern Conversion
// =============================================================================

impl From<proto::SchemaMapSpec> for SchemaMapSpec {
	fn from(p: proto::SchemaMapSpec) -> Self {
		SchemaMapSpec {
			mappings: p
				.mappings
				.into_iter()
				.map(|(k, v)| (k, v.into()))
				.collect(),
		}
	}
}

impl From<proto::FieldSource> for FieldSource {
	fn from(p: proto::FieldSource) -> Self {
		match p.source {
			Some(proto::field_source::Source::Path(path)) => FieldSource::Path(path),
			Some(proto::field_source::Source::Literal(lit)) => FieldSource::Literal(lit.into()),
			Some(proto::field_source::Source::Coalesce(c)) => FieldSource::Coalesce(c.into()),
			Some(proto::field_source::Source::Template(t)) => FieldSource::Template(t.into()),
			Some(proto::field_source::Source::Concat(c)) => FieldSource::Concat(c.into()),
			Some(proto::field_source::Source::Nested(n)) => {
				// Nested is not boxed in proto
				FieldSource::Nested(Box::new(n.into()))
			},
			None => FieldSource::Path("$".to_string()),
		}
	}
}

impl From<proto::LiteralValue> for LiteralValue {
	fn from(p: proto::LiteralValue) -> Self {
		match p.value {
			Some(proto::literal_value::Value::StringValue(s)) => LiteralValue::StringValue(s),
			Some(proto::literal_value::Value::NumberValue(n)) => LiteralValue::NumberValue(n),
			Some(proto::literal_value::Value::BoolValue(b)) => LiteralValue::BoolValue(b),
			Some(proto::literal_value::Value::NullValue(_)) => LiteralValue::NullValue(true),
			None => LiteralValue::NullValue(true),
		}
	}
}

impl From<proto::CoalesceSource> for CoalesceSource {
	fn from(p: proto::CoalesceSource) -> Self {
		CoalesceSource { paths: p.paths }
	}
}

impl From<proto::TemplateSource> for TemplateSource {
	fn from(p: proto::TemplateSource) -> Self {
		TemplateSource {
			template: p.template,
			vars: p.vars,
		}
	}
}

impl From<proto::ConcatSource> for ConcatSource {
	fn from(p: proto::ConcatSource) -> Self {
		ConcatSource {
			paths: p.paths,
			separator: p.separator,
		}
	}
}

// =============================================================================
// Map Each Pattern Conversion
// =============================================================================

impl From<proto::MapEachSpec> for MapEachSpec {
	fn from(p: proto::MapEachSpec) -> Self {
		MapEachSpec {
			inner: p
				.inner
				.map(|boxed| (*boxed).into())
				.unwrap_or(MapEachInner::Tool(String::new())),
		}
	}
}

impl From<proto::MapEachInner> for MapEachInner {
	fn from(p: proto::MapEachInner) -> Self {
		match p.inner {
			Some(proto::map_each_inner::Inner::Tool(name)) => MapEachInner::Tool(name),
			Some(proto::map_each_inner::Inner::Pattern(pattern)) => {
				MapEachInner::Pattern(Box::new((*pattern).into()))
			},
			None => MapEachInner::Tool(String::new()),
		}
	}
}

// =============================================================================
// Stateful Pattern Conversions
// =============================================================================

impl From<proto::RetrySpec> for RetrySpec {
	fn from(p: proto::RetrySpec) -> Self {
		RetrySpec {
			inner: Box::new(
				p.inner
					.map(|boxed| (*boxed).into())
					.unwrap_or_else(|| StepOperation::Tool(ToolCall::new(""))),
			),
			max_attempts: p.max_attempts,
			backoff: p.backoff.map(Into::into).unwrap_or(BackoffStrategy::Fixed(FixedBackoff {
				delay_ms: 1000,
			})),
			retry_if: p.retry_if.map(Into::into),
			jitter: p.jitter,
			attempt_timeout_ms: p.attempt_timeout_ms,
		}
	}
}

impl From<proto::BackoffStrategy> for BackoffStrategy {
	fn from(p: proto::BackoffStrategy) -> Self {
		match p.strategy {
			Some(proto::backoff_strategy::Strategy::Fixed(f)) => BackoffStrategy::Fixed(f.into()),
			Some(proto::backoff_strategy::Strategy::Exponential(e)) => {
				BackoffStrategy::Exponential(e.into())
			},
			Some(proto::backoff_strategy::Strategy::Linear(l)) => BackoffStrategy::Linear(l.into()),
			None => BackoffStrategy::Fixed(FixedBackoff { delay_ms: 1000 }),
		}
	}
}

impl From<proto::FixedBackoff> for FixedBackoff {
	fn from(p: proto::FixedBackoff) -> Self {
		FixedBackoff {
			delay_ms: p.delay_ms,
		}
	}
}

impl From<proto::ExponentialBackoff> for ExponentialBackoff {
	fn from(p: proto::ExponentialBackoff) -> Self {
		ExponentialBackoff {
			initial_delay_ms: p.initial_delay_ms,
			max_delay_ms: p.max_delay_ms,
			multiplier: p.multiplier.unwrap_or(2.0),
		}
	}
}

impl From<proto::LinearBackoff> for LinearBackoff {
	fn from(p: proto::LinearBackoff) -> Self {
		LinearBackoff {
			initial_delay_ms: p.initial_delay_ms,
			increment_ms: p.increment_ms,
			max_delay_ms: p.max_delay_ms,
		}
	}
}

impl From<proto::TimeoutSpec> for TimeoutSpec {
	fn from(p: proto::TimeoutSpec) -> Self {
		TimeoutSpec {
			inner: Box::new(
				p.inner
					.map(|boxed| (*boxed).into())
					.unwrap_or_else(|| StepOperation::Tool(ToolCall::new(""))),
			),
			duration_ms: p.duration_ms,
			fallback: p.fallback.map(|boxed| Box::new((*boxed).into())),
			message: p.message,
		}
	}
}

impl From<proto::CacheSpec> for CacheSpec {
	fn from(p: proto::CacheSpec) -> Self {
		CacheSpec {
			key_paths: p.key_paths,
			inner: Box::new(
				p.inner
					.map(|boxed| (*boxed).into())
					.unwrap_or_else(|| StepOperation::Tool(ToolCall::new(""))),
			),
			store: p.store,
			ttl_seconds: p.ttl_seconds,
			stale_while_revalidate_seconds: p.stale_while_revalidate_seconds,
			cache_if: p.cache_if.map(Into::into),
		}
	}
}

impl From<proto::IdempotentSpec> for IdempotentSpec {
	fn from(p: proto::IdempotentSpec) -> Self {
		IdempotentSpec {
			key_paths: p.key_paths,
			inner: Box::new(
				p.inner
					.map(|boxed| (*boxed).into())
					.unwrap_or_else(|| StepOperation::Tool(ToolCall::new(""))),
			),
			store: p.store,
			ttl_seconds: p.ttl_seconds,
			on_duplicate: match p.on_duplicate {
				i if i == proto::OnDuplicate::Cached as i32 => OnDuplicate::Cached,
				i if i == proto::OnDuplicate::Skip as i32 => OnDuplicate::Skip,
				i if i == proto::OnDuplicate::Error as i32 => OnDuplicate::Error,
				_ => OnDuplicate::Cached,
			},
		}
	}
}

impl From<proto::CircuitBreakerSpec> for CircuitBreakerSpec {
	fn from(p: proto::CircuitBreakerSpec) -> Self {
		CircuitBreakerSpec {
			name: p.name,
			inner: Box::new(
				p.inner
					.map(|boxed| (*boxed).into())
					.unwrap_or_else(|| StepOperation::Tool(ToolCall::new(""))),
			),
			store: p.store,
			failure_threshold: p.failure_threshold,
			failure_window_seconds: p.failure_window_seconds,
			reset_timeout_seconds: p.reset_timeout_seconds,
			success_threshold: p.success_threshold.unwrap_or(1),
			fallback: p.fallback.map(|boxed| Box::new((*boxed).into())),
			failure_if: p.failure_if.map(Into::into),
		}
	}
}

impl From<proto::DeadLetterSpec> for DeadLetterSpec {
	fn from(p: proto::DeadLetterSpec) -> Self {
		DeadLetterSpec {
			inner: Box::new(
				p.inner
					.map(|boxed| (*boxed).into())
					.unwrap_or_else(|| StepOperation::Tool(ToolCall::new(""))),
			),
			dead_letter_tool: p.dead_letter_tool,
			max_attempts: p.max_attempts.unwrap_or(1),
			backoff: p.backoff.map(Into::into),
			rethrow: p.rethrow,
		}
	}
}

impl From<proto::SagaSpec> for SagaSpec {
	fn from(p: proto::SagaSpec) -> Self {
		SagaSpec {
			steps: p.steps.into_iter().map(Into::into).collect(),
			store: p.store,
			saga_id_path: p.saga_id_path,
			timeout_ms: p.timeout_ms,
			output: p.output.map(Into::into),
		}
	}
}

impl From<proto::SagaStep> for SagaStep {
	fn from(p: proto::SagaStep) -> Self {
		SagaStep {
			id: p.id,
			name: p.name,
			action: p
				.action
				.map(Into::into)
				.unwrap_or_else(|| StepOperation::Tool(ToolCall::new(""))),
			compensate: p.compensate.map(Into::into),
			input: p.input.map(Into::into).unwrap_or_else(|| {
				DataBinding::Input(InputBinding {
					path: "$".to_string(),
				})
			}),
		}
	}
}

impl From<proto::ClaimCheckSpec> for ClaimCheckSpec {
	fn from(p: proto::ClaimCheckSpec) -> Self {
		ClaimCheckSpec {
			store_tool: p.store_tool,
			retrieve_tool: p.retrieve_tool,
			inner: Box::new(
				p.inner
					.map(|boxed| (*boxed).into())
					.unwrap_or_else(|| StepOperation::Tool(ToolCall::new(""))),
			),
			retrieve_at_end: p.retrieve_at_end,
		}
	}
}

// =============================================================================
// Public conversion function
// =============================================================================

/// Parse a JSON string using proto types and convert to hand-written types.
///
/// This is the recommended way to parse registry JSON files during the migration:
/// - First attempts to parse using proto types (canonical proto3 format with "server" field)
/// - Falls back to hand-written types if proto parsing fails (for backward compat with "target" field)
/// - When proto succeeds, converts to hand-written types for runtime (with all methods)
///
/// # Example
/// ```ignore
/// let json = std::fs::read_to_string("registry.json")?;
/// let registry = parse_registry_from_proto(&json)?;
/// ```
pub fn parse_registry_from_proto(json: &str) -> Result<Registry, serde_json::Error> {
	// Try proto types first (canonical format with "server" field)
	match serde_json::from_str::<proto::Registry>(json) {
		Ok(proto_registry) => Ok(proto_registry.into()),
		Err(_proto_err) => {
			// Fall back to hand-written types (supports "target" alias for backward compat)
			serde_json::from_str::<Registry>(json)
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_minimal_registry() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": []
		}"#;
		let registry = parse_registry_from_proto(json).unwrap();
		assert_eq!(registry.schema_version, "2.0");
		assert!(registry.tools.is_empty());
	}

	#[test]
	fn test_parse_source_tool() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": [{
				"name": "get_weather",
				"description": "Get weather info",
				"source": {
					"server": "weather-backend",
					"tool": "fetch_weather"
				}
			}]
		}"#;
		let registry = parse_registry_from_proto(json).unwrap();
		assert_eq!(registry.tools.len(), 1);
		assert_eq!(registry.tools[0].name, "get_weather");

		let source = registry.tools[0].source_tool().unwrap();
		assert_eq!(source.target, "weather-backend"); // "server" → "target"
		assert_eq!(source.tool, "fetch_weather");
	}

	#[test]
	fn test_parse_pipeline() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": [{
				"name": "research",
				"spec": {
					"pipeline": {
						"steps": [{
							"id": "search",
							"operation": {"tool": {"name": "web_search"}},
							"input": {"input": {"path": "$"}}
						}]
					}
				}
			}]
		}"#;
		let registry = parse_registry_from_proto(json).unwrap();
		assert_eq!(registry.tools.len(), 1);
		assert!(registry.tools[0].is_composition());
	}

	#[test]
	fn test_parse_scatter_gather() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": [{
				"name": "multi_search",
				"spec": {
					"scatterGather": {
						"targets": [
							{"tool": "search_web"},
							{"tool": "search_arxiv"}
						],
						"aggregation": {
							"ops": [{"flatten": true}]
						}
					}
				}
			}]
		}"#;
		let registry = parse_registry_from_proto(json).unwrap();
		assert_eq!(registry.tools.len(), 1);

		if let ToolImplementation::Spec(PatternSpec::ScatterGather(sg)) =
			&registry.tools[0].implementation
		{
			assert_eq!(sg.targets.len(), 2);
		} else {
			panic!("Expected ScatterGather pattern");
		}
	}

	#[test]
	fn test_parse_with_defaults() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": [{
				"name": "get_weather",
				"source": {
					"server": "weather",
					"tool": "fetch",
					"defaults": {
						"units": "metric",
						"count": 10
					}
				}
			}]
		}"#;
		let registry = parse_registry_from_proto(json).unwrap();
		let source = registry.tools[0].source_tool().unwrap();

		assert_eq!(
			source.defaults.get("units"),
			Some(&serde_json::json!("metric"))
		);
		assert_eq!(source.defaults.get("count"), Some(&serde_json::json!(10.0))); // Numbers come as f64
	}

	#[test]
	fn test_parse_construct_binding() {
		let json = r#"{
			"schemaVersion": "2.0",
			"tools": [{
				"name": "combine",
				"spec": {
					"pipeline": {
						"steps": [{
							"id": "step1",
							"operation": {"tool": {"name": "merge"}},
							"input": {
								"construct": {
									"fields": {
										"a": {"input": {"path": "$.x"}},
										"b": {"input": {"path": "$.y"}}
									}
								}
							}
						}]
					}
				}
			}]
		}"#;
		let registry = parse_registry_from_proto(json).unwrap();

		if let ToolImplementation::Spec(PatternSpec::Pipeline(pipeline)) =
			&registry.tools[0].implementation
		{
			if let Some(DataBinding::Construct(construct)) = &pipeline.steps[0].input {
				assert_eq!(construct.fields.len(), 2);
				assert!(construct.fields.contains_key("a"));
				assert!(construct.fields.contains_key("b"));
			} else {
				panic!("Expected Construct binding");
			}
		} else {
			panic!("Expected Pipeline pattern");
		}
	}
}
