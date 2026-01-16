//! Stateful patterns for MCP tool composition.
//!
//! This module contains patterns that manage state across tool executions,
//! enabling complex workflows like the ClaimCheck pattern.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[cfg(feature = "schema")]
use schemars::JsonSchema;

/// Errors that can occur during ClaimCheck execution.
#[derive(Error, Debug)]
pub enum ExecutionError {
	#[error("store tool execution failed: {0}")]
	StoreFailed(String),

	#[error("retrieve tool execution failed: {0}")]
	RetrieveFailed(String),

	#[error("inner operation failed: {0}")]
	InnerOperationFailed(String),

	#[error("reference transform failed: {0}")]
	TransformFailed(String),

	#[error("tool not found: {0}")]
	ToolNotFound(String),

	#[error("executor not implemented")]
	NotImplemented,
}

/// ClaimCheck pattern specification.
///
/// The ClaimCheck pattern externalizes large payloads by:
/// 1. Storing the payload via a "store" tool and receiving a reference/ticket
/// 2. Processing the inner operation using only the reference
/// 3. Optionally retrieving the original payload at the end
///
/// This is useful when:
/// - Payloads are too large to pass through the entire processing pipeline
/// - Multiple operations need access to the same large payload
/// - You want to decouple payload storage from processing logic
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ClaimCheckSpec {
	/// The name of the MCP tool used to store the payload.
	/// This tool should accept the payload and return a reference/claim ticket.
	pub store_tool: String,

	/// The name of the MCP tool used to retrieve the payload.
	/// This tool should accept the reference and return the original payload.
	pub retrieve_tool: String,

	/// The inner operation to execute with the claim reference.
	/// This receives the reference from the store operation instead of the original payload.
	pub inner: Box<ClaimCheckInner>,

	/// Whether to retrieve the original payload at the end of processing.
	/// If true, the final result will be the original payload.
	/// If false, the final result will be the inner operation's result.
	#[serde(default)]
	pub retrieve_at_end: bool,

	/// Optional transformation to apply to the stored reference before passing to inner operation.
	/// This is a CEL expression that transforms the store result.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub reference_transform: Option<String>,
}

/// Inner operation for the ClaimCheck pattern.
///
/// Defines what operation to perform with the claim reference.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum ClaimCheckInner {
	/// Execute another MCP tool with the reference.
	Tool {
		/// The name of the tool to execute.
		name: String,
		/// Optional additional arguments to pass to the tool (merged with the reference).
		#[serde(default, skip_serializing_if = "Option::is_none")]
		args: Option<Value>,
	},

	/// Pass through the reference unchanged (useful for testing or simple workflows).
	#[default]
	Passthrough,
}

/// Trait for executing MCP tools.
///
/// This trait abstracts tool execution so the ClaimCheckExecutor can be tested
/// with mock implementations.
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
	/// Execute a tool with the given name and input.
	async fn execute_tool(&self, tool_name: &str, input: Value) -> Result<Value, ExecutionError>;
}

/// Executor for the ClaimCheck pattern.
///
/// Orchestrates the store -> process -> retrieve workflow.
pub struct ClaimCheckExecutor<T: ToolExecutor> {
	executor: Arc<T>,
}

impl<T: ToolExecutor> ClaimCheckExecutor<T> {
	/// Create a new ClaimCheckExecutor with the given tool executor.
	pub fn new(executor: Arc<T>) -> Self {
		Self { executor }
	}

	/// Execute the ClaimCheck pattern with the given spec and input.
	///
	/// The execution flow is:
	/// 1. Store the input payload using `spec.store_tool`
	/// 2. Optionally transform the reference using `spec.reference_transform`
	/// 3. Execute the inner operation with the reference
	/// 4. If `spec.retrieve_at_end` is true, retrieve the original payload
	/// 5. Return either the inner result or the retrieved payload
	pub async fn execute(
		&self,
		spec: &ClaimCheckSpec,
		input: Value,
	) -> Result<Value, ExecutionError> {
		// Step 1: Store the payload and get a reference
		let store_result = self
			.executor
			.execute_tool(&spec.store_tool, input)
			.await
			.map_err(|e| ExecutionError::StoreFailed(e.to_string()))?;

		// Step 2: Optionally transform the reference
		let reference = if let Some(ref transform) = spec.reference_transform {
			self.apply_transform(transform, &store_result)?
		} else {
			store_result.clone()
		};

		// Step 3: Execute the inner operation with the reference
		let inner_result = match spec.inner.as_ref() {
			ClaimCheckInner::Tool { name, args } => {
				// Merge reference with any additional args
				let tool_input = if let Some(extra_args) = args {
					Self::merge_json(&reference, extra_args)
				} else {
					reference.clone()
				};
				self
					.executor
					.execute_tool(name, tool_input)
					.await
					.map_err(|e| ExecutionError::InnerOperationFailed(e.to_string()))?
			},
			ClaimCheckInner::Passthrough => {
				// Just return the reference unchanged
				reference.clone()
			},
		};

		// Step 4: Optionally retrieve the original payload
		if spec.retrieve_at_end {
			self
				.executor
				.execute_tool(&spec.retrieve_tool, store_result)
				.await
				.map_err(|e| ExecutionError::RetrieveFailed(e.to_string()))
		} else {
			Ok(inner_result)
		}
	}

	/// Apply a simple JSONPath-like transform to extract a value.
	/// For now, supports simple dot notation like "input.reference"
	fn apply_transform(&self, transform: &str, input: &Value) -> Result<Value, ExecutionError> {
		// Simple implementation: support "input.field" pattern
		let parts: Vec<&str> = transform.split('.').collect();
		if parts.is_empty() {
			return Err(ExecutionError::TransformFailed("empty transform".into()));
		}

		let mut current = input;
		for (i, part) in parts.iter().enumerate() {
			// Skip "input" as the root
			if i == 0 && *part == "input" {
				continue;
			}
			current = current
				.get(*part)
				.ok_or_else(|| ExecutionError::TransformFailed(format!("field '{}' not found", part)))?;
		}
		Ok(current.clone())
	}

	/// Merge two JSON values, with the second taking precedence for conflicts.
	fn merge_json(base: &Value, overlay: &Value) -> Value {
		match (base, overlay) {
			(Value::Object(base_map), Value::Object(overlay_map)) => {
				let mut result = base_map.clone();
				for (k, v) in overlay_map {
					result.insert(k.clone(), v.clone());
				}
				Value::Object(result)
			},
			_ => overlay.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;
	use std::sync::Mutex;

	/// Mock tool executor for testing.
	struct MockToolExecutor {
		/// Stored payloads by reference ID.
		store: Mutex<HashMap<String, Value>>,
		/// Counter for generating unique reference IDs.
		counter: Mutex<u64>,
		/// Results to return for specific tool calls.
		tool_results: HashMap<String, Value>,
	}

	impl MockToolExecutor {
		fn new() -> Self {
			Self {
				store: Mutex::new(HashMap::new()),
				counter: Mutex::new(0),
				tool_results: HashMap::new(),
			}
		}

		fn with_tool_result(mut self, tool_name: &str, result: Value) -> Self {
			self.tool_results.insert(tool_name.to_string(), result);
			self
		}
	}

	#[async_trait::async_trait]
	impl ToolExecutor for MockToolExecutor {
		async fn execute_tool(&self, tool_name: &str, input: Value) -> Result<Value, ExecutionError> {
			match tool_name {
				"blob_store" => {
					// Store the payload and return a reference
					let mut counter = self.counter.lock().unwrap();
					*counter += 1;
					let ref_id = format!("ref-{}", *counter);
					self.store.lock().unwrap().insert(ref_id.clone(), input);
					Ok(serde_json::json!({ "reference": ref_id }))
				},
				"blob_retrieve" => {
					// Retrieve the payload by reference
					let ref_id = input
						.get("reference")
						.and_then(|v| v.as_str())
						.ok_or_else(|| ExecutionError::RetrieveFailed("missing reference".into()))?;
					let store = self.store.lock().unwrap();
					store
						.get(ref_id)
						.cloned()
						.ok_or_else(|| ExecutionError::RetrieveFailed("reference not found".into()))
				},
				name => {
					// Check for pre-configured results
					self
						.tool_results
						.get(name)
						.cloned()
						.ok_or_else(|| ExecutionError::ToolNotFound(name.to_string()))
				},
			}
		}
	}

	#[test]
	fn test_claim_check_spec_serialization() {
		let spec = ClaimCheckSpec {
			store_tool: "blob_store".to_string(),
			retrieve_tool: "blob_retrieve".to_string(),
			inner: Box::new(ClaimCheckInner::Tool {
				name: "process_ref".to_string(),
				args: None,
			}),
			retrieve_at_end: true,
			reference_transform: None,
		};

		let json = serde_json::to_string_pretty(&spec).unwrap();
		assert!(json.contains("storeTool"));
		assert!(json.contains("retrieveTool"));
		assert!(json.contains("retrieveAtEnd"));

		// Round-trip test
		let deserialized: ClaimCheckSpec = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized.store_tool, "blob_store");
		assert_eq!(deserialized.retrieve_at_end, true);
	}

	#[test]
	fn test_claim_check_spec_default_retrieve_at_end() {
		let json = r#"{
            "storeTool": "store",
            "retrieveTool": "retrieve",
            "inner": "passthrough"
        }"#;

		let spec: ClaimCheckSpec = serde_json::from_str(json).unwrap();
		assert_eq!(spec.retrieve_at_end, false);
	}

	#[test]
	fn test_claim_check_inner_variants() {
		// Tool variant
		let tool = ClaimCheckInner::Tool {
			name: "my_tool".to_string(),
			args: Some(serde_json::json!({"extra": "arg"})),
		};
		let json = serde_json::to_string(&tool).unwrap();
		assert!(json.contains("my_tool"));

		// Passthrough variant
		let passthrough = ClaimCheckInner::Passthrough;
		let json = serde_json::to_string(&passthrough).unwrap();
		assert_eq!(json, "\"passthrough\"");
	}

	// ============================================================
	// TDD Tests - These will fail until ClaimCheckExecutor is implemented
	// ============================================================

	/// Test that ClaimCheck stores payload and processes with reference.
	#[tokio::test]
	async fn test_claim_check_store_and_process() {
		let mock = MockToolExecutor::new()
			.with_tool_result("process_ref", serde_json::json!({"processed": true}));
		let executor = ClaimCheckExecutor::new(Arc::new(mock));

		let spec = ClaimCheckSpec {
			store_tool: "blob_store".to_string(),
			retrieve_tool: "blob_retrieve".to_string(),
			inner: Box::new(ClaimCheckInner::Tool {
				name: "process_ref".to_string(),
				args: None,
			}),
			retrieve_at_end: false,
			reference_transform: None,
		};

		let input = serde_json::json!({"large_payload": "data".repeat(1000)});
		let result = executor.execute(&spec, input).await;

		// Should succeed and return the inner operation's result
		assert!(result.is_ok(), "Expected success, got: {:?}", result);
		let value = result.unwrap();
		assert_eq!(value.get("processed"), Some(&serde_json::json!(true)));
	}

	/// Test that ClaimCheck retrieves original payload at end when configured.
	#[tokio::test]
	async fn test_claim_check_retrieve_at_end() {
		let mock = MockToolExecutor::new()
			.with_tool_result("process_ref", serde_json::json!({"processed": true}));
		let executor = ClaimCheckExecutor::new(Arc::new(mock));

		let spec = ClaimCheckSpec {
			store_tool: "blob_store".to_string(),
			retrieve_tool: "blob_retrieve".to_string(),
			inner: Box::new(ClaimCheckInner::Tool {
				name: "process_ref".to_string(),
				args: None,
			}),
			retrieve_at_end: true, // Should retrieve original at end
			reference_transform: None,
		};

		let original_payload = serde_json::json!({"original": "data", "important": true});
		let result = executor.execute(&spec, original_payload.clone()).await;

		// Should succeed and return the ORIGINAL payload, not the inner result
		assert!(result.is_ok(), "Expected success, got: {:?}", result);
		let value = result.unwrap();
		assert_eq!(value, original_payload);
	}

	/// Test that ClaimCheck returns inner result when retrieve_at_end is false.
	#[tokio::test]
	async fn test_claim_check_no_retrieve() {
		let mock = MockToolExecutor::new().with_tool_result(
			"transform_ref",
			serde_json::json!({"transformed": "result"}),
		);
		let executor = ClaimCheckExecutor::new(Arc::new(mock));

		let spec = ClaimCheckSpec {
			store_tool: "blob_store".to_string(),
			retrieve_tool: "blob_retrieve".to_string(),
			inner: Box::new(ClaimCheckInner::Tool {
				name: "transform_ref".to_string(),
				args: None,
			}),
			retrieve_at_end: false, // Should NOT retrieve
			reference_transform: None,
		};

		let input = serde_json::json!({"data": "test"});
		let result = executor.execute(&spec, input).await;

		// Should succeed and return the inner operation's result
		assert!(result.is_ok(), "Expected success, got: {:?}", result);
		let value = result.unwrap();
		assert_eq!(value.get("transformed"), Some(&serde_json::json!("result")));
	}

	/// Test that ClaimCheck can transform the reference before passing to inner operation.
	#[tokio::test]
	async fn test_claim_check_with_transform() {
		let mock = MockToolExecutor::new().with_tool_result(
			"process_transformed",
			serde_json::json!({"result": "transformed"}),
		);
		let executor = ClaimCheckExecutor::new(Arc::new(mock));

		let spec = ClaimCheckSpec {
			store_tool: "blob_store".to_string(),
			retrieve_tool: "blob_retrieve".to_string(),
			inner: Box::new(ClaimCheckInner::Tool {
				name: "process_transformed".to_string(),
				args: None,
			}),
			retrieve_at_end: false,
			// CEL expression to extract just the reference string
			reference_transform: Some("input.reference".to_string()),
		};

		let input = serde_json::json!({"data": "large_data"});
		let result = executor.execute(&spec, input).await;

		// Should succeed - the transform extracts the reference for the inner tool
		assert!(result.is_ok(), "Expected success, got: {:?}", result);
	}

	/// Test passthrough inner operation just returns the reference.
	#[tokio::test]
	async fn test_claim_check_passthrough() {
		let mock = MockToolExecutor::new();
		let executor = ClaimCheckExecutor::new(Arc::new(mock));

		let spec = ClaimCheckSpec {
			store_tool: "blob_store".to_string(),
			retrieve_tool: "blob_retrieve".to_string(),
			inner: Box::new(ClaimCheckInner::Passthrough),
			retrieve_at_end: false,
			reference_transform: None,
		};

		let input = serde_json::json!({"data": "test"});
		let result = executor.execute(&spec, input).await;

		// Should succeed and return the reference from store
		assert!(result.is_ok(), "Expected success, got: {:?}", result);
		let value = result.unwrap();
		assert!(
			value.get("reference").is_some(),
			"Expected reference in result"
		);
	}
}
