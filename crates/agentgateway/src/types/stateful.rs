//! Stateful composition patterns for resilient operation orchestration.
//!
//! This module provides IR types for stateful patterns like Timeout, Retry,
//! Circuit Breaker, Saga, etc. These patterns wrap operations and provide
//! resilience capabilities at the composition level.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Timeout pattern specification.
///
/// Wraps an inner operation with a timeout. If the inner operation doesn't
/// complete within the specified duration, either returns an error or
/// executes a fallback operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeoutSpec {
	/// The timeout duration in milliseconds.
	pub duration_ms: u64,
	/// The inner operation to execute with a timeout.
	pub inner: Box<Operation>,
	/// Optional fallback operation to execute if timeout is exceeded.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fallback: Option<Box<Operation>>,
	/// Optional custom error message for timeout errors.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error_message: Option<String>,
}

impl TimeoutSpec {
	/// Create a new timeout specification.
	pub fn new(duration_ms: u64, inner: Operation) -> Self {
		Self {
			duration_ms,
			inner: Box::new(inner),
			fallback: None,
			error_message: None,
		}
	}

	/// Add a fallback operation.
	pub fn with_fallback(mut self, fallback: Operation) -> Self {
		self.fallback = Some(Box::new(fallback));
		self
	}

	/// Add a custom error message.
	pub fn with_error_message(mut self, message: impl Into<String>) -> Self {
		self.error_message = Some(message.into());
		self
	}

	/// Get the timeout duration.
	pub fn duration(&self) -> Duration {
		Duration::from_millis(self.duration_ms)
	}
}

/// Represents an operation that can be composed with patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Operation {
	/// A direct value/constant operation.
	Constant { value: Value },
	/// An MCP tool call operation.
	ToolCall {
		tool_name: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		arguments: Option<Value>,
	},
	/// A timeout-wrapped operation.
	Timeout(TimeoutSpec),
	// Future patterns will be added here:
	// Retry(RetrySpec),
	// CircuitBreaker(CircuitBreakerSpec),
	// Saga(SagaSpec),
	// Idempotent(IdempotentSpec),
	// ClaimCheck(ClaimCheckSpec),
}

impl Operation {
	/// Create a constant operation.
	pub fn constant(value: Value) -> Self {
		Self::Constant { value }
	}

	/// Create a tool call operation.
	pub fn tool_call(tool_name: impl Into<String>) -> Self {
		Self::ToolCall {
			tool_name: tool_name.into(),
			arguments: None,
		}
	}

	/// Create a tool call operation with arguments.
	pub fn tool_call_with_args(tool_name: impl Into<String>, arguments: Value) -> Self {
		Self::ToolCall {
			tool_name: tool_name.into(),
			arguments: Some(arguments),
		}
	}

	/// Wrap this operation with a timeout.
	pub fn with_timeout(self, duration_ms: u64) -> Self {
		Self::Timeout(TimeoutSpec::new(duration_ms, self))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_timeout_spec_creation() {
		let inner = Operation::tool_call("test_tool");
		let spec = TimeoutSpec::new(5000, inner);

		assert_eq!(spec.duration_ms, 5000);
		assert!(spec.fallback.is_none());
		assert!(spec.error_message.is_none());
	}

	#[test]
	fn test_timeout_spec_with_fallback() {
		let inner = Operation::tool_call("main_tool");
		let fallback = Operation::constant(json!({"status": "fallback"}));
		let spec = TimeoutSpec::new(5000, inner).with_fallback(fallback);

		assert!(spec.fallback.is_some());
	}

	#[test]
	fn test_timeout_spec_with_error_message() {
		let inner = Operation::tool_call("test_tool");
		let spec = TimeoutSpec::new(5000, inner).with_error_message("Custom timeout message");

		assert_eq!(
			spec.error_message,
			Some("Custom timeout message".to_string())
		);
	}

	#[test]
	fn test_timeout_duration() {
		let inner = Operation::tool_call("test_tool");
		let spec = TimeoutSpec::new(5000, inner);

		assert_eq!(spec.duration(), Duration::from_millis(5000));
	}

	#[test]
	fn test_operation_serialization() {
		let op = Operation::Timeout(TimeoutSpec::new(5000, Operation::tool_call("test_tool")));

		let json = serde_json::to_string(&op).unwrap();
		let parsed: Operation = serde_json::from_str(&json).unwrap();
		assert_eq!(op, parsed);
	}

	#[test]
	fn test_operation_with_timeout_builder() {
		let op = Operation::tool_call("my_tool").with_timeout(3000);

		match op {
			Operation::Timeout(spec) => {
				assert_eq!(spec.duration_ms, 3000);
			},
			_ => panic!("Expected Timeout operation"),
		}
	}
}
