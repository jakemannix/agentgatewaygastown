// Tracing utilities for composition execution observability

use std::time::Duration;

use agent_core::trcng;
use opentelemetry::global::BoxedSpan;
use opentelemetry::trace::{Span, SpanKind, Status, Tracer};
use opentelemetry::KeyValue;
use serde_json::Value;
use tracing::info;

use super::context::TracingContext;
use crate::telemetry::log::CompositionVerbosity;

/// Maximum size for serialized input/output in bytes (4KB)
const MAX_PAYLOAD_BYTES: usize = 4096;

/// Truncate a JSON value to a string with a maximum byte size.
/// Returns "[truncated]" suffix if the value was truncated.
pub fn truncate_json(value: &Value, max_bytes: usize) -> String {
	let serialized = match serde_json::to_string(value) {
		Ok(s) => s,
		Err(_) => return "[serialization error]".to_string(),
	};

	if serialized.len() <= max_bytes {
		serialized
	} else {
		// Truncate and add indicator
		let truncated: String = serialized.chars().take(max_bytes.saturating_sub(15)).collect();
		format!("{}...[truncated]", truncated)
	}
}

/// Log step start to stdout (always, regardless of OTEL sampling).
/// Call this before create_step_span for console visibility during development.
pub fn log_step_start(
	ctx: &TracingContext,
	step_id: &str,
	operation_type: &str,
	input: Option<&Value>,
) {
	// Direct stderr output for debugging (bypasses tracing subscriber filters)
	eprintln!(
		"[COMPOSITION DEBUG] step={} op={} verbosity={:?}",
		step_id, operation_type, ctx.verbosity
	);
	if ctx.verbosity == CompositionVerbosity::Full {
		if let Some(input_val) = input {
			let input_str = truncate_json(input_val, MAX_PAYLOAD_BYTES);
			info!(
				target: "composition",
				step_id = %step_id,
				operation_type = %operation_type,
				input = %input_str,
				"step started"
			);
		} else {
			info!(
				target: "composition",
				step_id = %step_id,
				operation_type = %operation_type,
				"step started"
			);
		}
	} else {
		info!(
			target: "composition",
			step_id = %step_id,
			operation_type = %operation_type,
			"step started"
		);
	}
}

/// Log step completion to stdout (always, regardless of OTEL sampling).
pub fn log_step_complete(
	ctx: &TracingContext,
	step_id: &str,
	duration: Duration,
	result: Result<&Value, &str>,
) {
	match &result {
		Ok(output) => {
			if ctx.verbosity == CompositionVerbosity::Full {
				let output_str = truncate_json(output, MAX_PAYLOAD_BYTES);
				info!(
					target: "composition",
					step_id = %step_id,
					duration_ms = duration.as_millis() as u64,
					output = %output_str,
					"step completed"
				);
			} else if ctx.verbosity >= CompositionVerbosity::Timing {
				info!(
					target: "composition",
					step_id = %step_id,
					duration_ms = duration.as_millis() as u64,
					"step completed"
				);
			} else {
				info!(
					target: "composition",
					step_id = %step_id,
					"step completed"
				);
			}
		},
		Err(error_msg) => {
			info!(
				target: "composition",
				step_id = %step_id,
				duration_ms = duration.as_millis() as u64,
				error = %error_msg,
				"step failed"
			);
		},
	}
}

/// Create a span for a composition step (OTEL only, no console logging).
/// Returns None if not sampled.
pub fn create_step_span(
	ctx: &TracingContext,
	step_id: &str,
	operation_type: &str,
	input: Option<&Value>,
) -> Option<BoxedSpan> {
	if !ctx.sampled {
		return None;
	}

	let tracer = trcng::get_tracer();

	// Build span with parent context
	let mut span = tracer
		.span_builder(format!("step/{}", step_id))
		.with_kind(SpanKind::Internal)
		.start_with_context(tracer, &ctx.parent_context);

	// Add basic attributes
	span.set_attribute(KeyValue::new("step.id", step_id.to_string()));
	span.set_attribute(KeyValue::new("step.operation_type", operation_type.to_string()));

	// Add input if verbosity is Full
	if ctx.verbosity == CompositionVerbosity::Full {
		if let Some(input_val) = input {
			let input_str = truncate_json(input_val, MAX_PAYLOAD_BYTES);
			span.set_attribute(KeyValue::new("step.input", input_str));
		}
	}

	Some(span)
}

/// Log composition start to stdout (always, regardless of OTEL sampling).
pub fn log_composition_start(ctx: &TracingContext, composition_name: &str) {
	// Direct stderr output for debugging (bypasses tracing subscriber filters)
	eprintln!(
		"[COMPOSITION DEBUG] composition={} verbosity={:?} sampled={}",
		composition_name, ctx.verbosity, ctx.sampled
	);
	info!(
		target: "composition",
		composition = %composition_name,
		verbosity = ?ctx.verbosity,
		sampled = ctx.sampled,
		"composition execution started"
	);
}

/// Create a span for a composition execution (OTEL only).
/// Returns None if not sampled.
pub fn create_composition_span(ctx: &TracingContext, composition_name: &str) -> Option<BoxedSpan> {
	if !ctx.sampled {
		return None;
	}

	let tracer = trcng::get_tracer();

	let mut span = tracer
		.span_builder(format!("composition/{}", composition_name))
		.with_kind(SpanKind::Internal)
		.start_with_context(tracer, &ctx.parent_context);

	span.set_attribute(KeyValue::new("composition.name", composition_name.to_string()));

	Some(span)
}

/// Create a span for a scatter-gather target.
/// Returns None if tracing is not enabled or not sampled.
pub fn create_target_span(
	ctx: &TracingContext,
	target_name: &str,
	target_index: usize,
) -> Option<BoxedSpan> {
	if !ctx.sampled {
		return None;
	}

	let tracer = trcng::get_tracer();

	let mut span = tracer
		.span_builder(format!("target/{}", target_name))
		.with_kind(SpanKind::Internal)
		.start_with_context(tracer, &ctx.parent_context);

	span.set_attribute(KeyValue::new("target.name", target_name.to_string()));
	span.set_attribute(KeyValue::new("target.index", target_index as i64));

	Some(span)
}

/// Record completion of a step span with timing and optional output (OTEL only).
/// For console logging, use log_step_complete separately.
pub fn record_step_completion<S: Span>(
	span: &mut S,
	ctx: &TracingContext,
	duration: Duration,
	result: Result<&Value, &str>,
) {
	// Add timing if verbosity >= Timing
	if ctx.verbosity >= CompositionVerbosity::Timing {
		span.set_attribute(KeyValue::new("step.duration_ms", duration.as_millis() as i64));
	}

	match result {
		Ok(output) => {
			// Add output if verbosity is Full
			if ctx.verbosity == CompositionVerbosity::Full {
				let output_str = truncate_json(output, MAX_PAYLOAD_BYTES);
				span.set_attribute(KeyValue::new("step.output", output_str));
			}
			span.set_status(Status::Ok);
		},
		Err(error_msg) => {
			span.set_attribute(KeyValue::new("step.error", error_msg.to_string()));
			span.set_status(Status::error(error_msg.to_string()));
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_truncate_json_small() {
		let value = serde_json::json!({"key": "value"});
		let result = truncate_json(&value, 100);
		assert_eq!(result, r#"{"key":"value"}"#);
	}

	#[test]
	fn test_truncate_json_large() {
		let large_string = "x".repeat(5000);
		let value = serde_json::json!({"data": large_string});
		let result = truncate_json(&value, 100);
		assert!(result.len() <= 100);
		assert!(result.ends_with("...[truncated]"));
	}

	#[test]
	fn test_truncate_json_exact_boundary() {
		let value = serde_json::json!({"a": "b"});
		let serialized = serde_json::to_string(&value).unwrap();
		let result = truncate_json(&value, serialized.len());
		assert_eq!(result, serialized);
	}
}
