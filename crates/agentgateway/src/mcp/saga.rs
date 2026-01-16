//! Saga Pattern implementation for orchestrating multi-step tool workflows
//!
//! The Saga pattern enables executing a sequence of tool calls with automatic
//! compensation (rollback) on failure. Each step can optionally define a
//! compensating action that undoes its effects.
//!
//! # Example
//!
//! ```json
//! {
//!   "saga": {
//!     "steps": [
//!       {
//!         "id": "flight",
//!         "name": "Book flight",
//!         "action": { "name": "airline.book" },
//!         "compensate": { "name": "airline.cancel" }
//!       },
//!       {
//!         "id": "hotel",
//!         "name": "Book hotel",
//!         "action": { "name": "hotel.reserve" },
//!         "compensate": { "name": "hotel.cancel" }
//!       }
//!     ]
//!   }
//! }
//! ```

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A saga is a sequence of steps with compensation on failure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(crate::JsonSchema))]
pub struct Saga {
	/// Ordered list of steps to execute
	pub steps: Vec<SagaStep>,
	/// Optional overall timeout for the saga (e.g., "30s", "1m", "1h")
	#[serde(
		default,
		skip_serializing_if = "Option::is_none",
		with = "serde_duration_opt"
	)]
	#[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
	pub timeout: Option<Duration>,
	/// Optional output binding to construct the result
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub output: Option<OutputBinding>,
}

/// A single step in a saga
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(crate::JsonSchema))]
pub struct SagaStep {
	/// Unique identifier for this step (used in output binding)
	pub id: String,
	/// Human-readable name for logging
	pub name: String,
	/// The action to execute
	pub action: ToolCall,
	/// Optional compensation action to run on failure
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub compensate: Option<ToolCall>,
	/// Input binding for this step
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub input: Option<InputBinding>,
}

/// A tool call specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(crate::JsonSchema))]
pub struct ToolCall {
	/// Tool name to invoke
	pub name: String,
	/// Arguments (can include path bindings)
	#[serde(default)]
	pub arguments: Value,
}

/// Input binding for a saga step
///
/// Can be a direct path reference, an object with nested bindings, or a literal value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(crate::JsonSchema))]
pub enum InputBinding {
	/// Direct path reference (e.g., `$.input.flight`)
	Path { path: String },
	/// Object with nested bindings
	Object(HashMap<String, InputBinding>),
	/// Literal value
	Literal(Value),
}

/// Output binding for constructing the saga result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(crate::JsonSchema))]
pub struct OutputBinding {
	/// Map of output field names to path bindings
	#[serde(flatten)]
	pub fields: HashMap<String, PathBinding>,
}

/// A path binding that references a value in the execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(crate::JsonSchema))]
pub struct PathBinding {
	/// JSONPath-like path to extract value
	/// Examples:
	/// - `$.input.flight` - from saga input
	/// - `$.steps.flight.confirmationNumber` - from step output
	pub path: String,
}

/// Result of executing a saga
#[derive(Debug, Clone)]
pub struct SagaResult {
	/// The constructed output based on output binding
	pub output: Value,
	/// Results from each step, keyed by step id
	pub step_results: HashMap<String, Value>,
	/// The final status of the saga
	pub status: SagaStatus,
}

/// The status of a saga execution
#[derive(Debug, Clone)]
pub enum SagaStatus {
	/// All steps completed successfully
	Completed,
	/// A step failed, compensation may have been attempted
	Failed {
		/// The step that failed
		step: String,
		/// The error message
		error: String,
	},
	/// Compensation also failed
	CompensationFailed {
		/// The original failure error
		original_error: String,
		/// Errors from compensation attempts
		compensation_errors: Vec<String>,
	},
	/// The saga timed out
	TimedOut,
}

/// Error type for saga execution
#[derive(Debug, thiserror::Error)]
pub enum SagaError {
	#[error("step {step} failed: {message}")]
	StepFailed { step: String, message: String },

	#[error("invalid path binding: {0}")]
	InvalidPath(String),

	#[error("saga timed out after {0:?}")]
	Timeout(Duration),

	#[error("tool execution error: {0}")]
	ToolError(String),
}

/// Trait for executing tool calls
///
/// This abstraction allows the SagaExecutor to be tested with mock tools.
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
	/// Execute a tool call and return the result
	async fn execute(&self, tool: &ToolCall, input: Value) -> Result<Value, SagaError>;
}

/// Executor for sagas
pub struct SagaExecutor<T: ToolExecutor> {
	tool_executor: T,
}

impl<T: ToolExecutor> SagaExecutor<T> {
	/// Create a new SagaExecutor with the given tool executor
	pub fn new(tool_executor: T) -> Self {
		Self { tool_executor }
	}

	/// Execute a saga with the given input
	///
	/// This is the main entry point for saga execution. It will:
	/// 1. Execute each step in order
	/// 2. On failure, run compensation in reverse order
	/// 3. Construct the output based on the output binding
	pub async fn execute(&self, saga: &Saga, input: Value) -> Result<SagaResult, SagaError> {
		// If timeout is specified, wrap the execution in a timeout
		if let Some(timeout) = saga.timeout {
			match tokio::time::timeout(timeout, self.execute_inner(saga, input)).await {
				Ok(result) => result,
				Err(_) => Ok(SagaResult {
					output: Value::Null,
					step_results: HashMap::new(),
					status: SagaStatus::TimedOut,
				}),
			}
		} else {
			self.execute_inner(saga, input).await
		}
	}

	async fn execute_inner(&self, saga: &Saga, input: Value) -> Result<SagaResult, SagaError> {
		let mut step_results: HashMap<String, Value> = HashMap::new();
		let mut completed_steps: Vec<&SagaStep> = Vec::new();

		// Forward execution
		for step in &saga.steps {
			// Resolve input binding
			let step_input = self.resolve_input(&step.input, &input, &step_results)?;

			// Execute the action
			match self
				.tool_executor
				.execute(&step.action, step_input.clone())
				.await
			{
				Ok(result) => {
					step_results.insert(step.id.clone(), result);
					completed_steps.push(step);
				},
				Err(e) => {
					// Step failed, run compensation
					let compensation_errors = self
						.run_compensation(&completed_steps, &input, &step_results)
						.await;

					let status = if compensation_errors.is_empty() {
						SagaStatus::Failed {
							step: step.id.clone(),
							error: e.to_string(),
						}
					} else {
						SagaStatus::CompensationFailed {
							original_error: e.to_string(),
							compensation_errors,
						}
					};

					return Ok(SagaResult {
						output: Value::Null,
						step_results,
						status,
					});
				},
			}
		}

		// All steps completed, construct output
		let output = self.resolve_output(&saga.output, &input, &step_results)?;

		Ok(SagaResult {
			output,
			step_results,
			status: SagaStatus::Completed,
		})
	}

	/// Run compensation for completed steps in reverse order
	async fn run_compensation(
		&self,
		completed_steps: &[&SagaStep],
		input: &Value,
		step_results: &HashMap<String, Value>,
	) -> Vec<String> {
		let mut errors = Vec::new();

		// Run compensation in reverse order
		for step in completed_steps.iter().rev() {
			if let Some(compensate) = &step.compensate {
				// Resolve input for compensation (same as the original action)
				let comp_input = match self.resolve_input(&step.input, input, step_results) {
					Ok(input) => input,
					Err(e) => {
						errors.push(format!("Failed to resolve compensation input for {}: {}", step.id, e));
						continue;
					},
				};

				// Execute compensation - best effort, don't stop on error
				if let Err(e) = self.tool_executor.execute(compensate, comp_input).await {
					errors.push(format!("Compensation failed for {}: {}", step.id, e));
				}
			}
		}

		errors
	}

	/// Resolve input binding to a concrete value
	fn resolve_input(
		&self,
		binding: &Option<InputBinding>,
		input: &Value,
		step_results: &HashMap<String, Value>,
	) -> Result<Value, SagaError> {
		match binding {
			None => Ok(input.clone()),
			Some(InputBinding::Literal(v)) => Ok(v.clone()),
			Some(InputBinding::Path { path }) => self.resolve_path(path, input, step_results),
			Some(InputBinding::Object(obj)) => {
				let mut result = serde_json::Map::new();
				for (key, binding) in obj {
					let value = self.resolve_input(&Some(binding.clone()), input, step_results)?;
					result.insert(key.clone(), value);
				}
				Ok(Value::Object(result))
			},
		}
	}

	/// Resolve a JSONPath-like path to a value
	fn resolve_path(
		&self,
		path: &str,
		input: &Value,
		step_results: &HashMap<String, Value>,
	) -> Result<Value, SagaError> {
		// Parse path like "$.input.flight" or "$.steps.flight.confirmationNumber"
		let parts: Vec<&str> = path.split('.').collect();
		if parts.is_empty() || parts[0] != "$" {
			return Err(SagaError::InvalidPath(format!(
				"Path must start with $: {}",
				path
			)));
		}

		if parts.len() < 2 {
			return Err(SagaError::InvalidPath(format!("Path too short: {}", path)));
		}

		let root = match parts[1] {
			"input" => input,
			"steps" => {
				if parts.len() < 3 {
					return Err(SagaError::InvalidPath(format!(
						"$.steps requires step id: {}",
						path
					)));
				}
				let step_id = parts[2];
				step_results
					.get(step_id)
					.ok_or_else(|| SagaError::InvalidPath(format!("Step {} not found", step_id)))?
			},
			other => {
				return Err(SagaError::InvalidPath(format!(
					"Unknown root '{}' in path: {}",
					other, path
				)))
			},
		};

		// Navigate the remaining path
		let start_idx = if parts[1] == "steps" { 3 } else { 2 };
		let mut current = root;
		for part in parts.iter().skip(start_idx) {
			current = current
				.get(*part)
				.ok_or_else(|| SagaError::InvalidPath(format!("Key '{}' not found in path: {}", part, path)))?;
		}

		Ok(current.clone())
	}

	/// Resolve output binding to construct the final result
	fn resolve_output(
		&self,
		binding: &Option<OutputBinding>,
		input: &Value,
		step_results: &HashMap<String, Value>,
	) -> Result<Value, SagaError> {
		match binding {
			None => {
				// No output binding, return all step results
				Ok(serde_json::to_value(step_results).unwrap_or(Value::Null))
			},
			Some(output) => {
				let mut result = serde_json::Map::new();
				for (key, path_binding) in &output.fields {
					let value = self.resolve_path(&path_binding.path, input, step_results)?;
					result.insert(key.clone(), value);
				}
				Ok(Value::Object(result))
			},
		}
	}
}

/// Helper module for serializing Option<Duration>
mod serde_duration_opt {
	use serde::{self, Deserialize, Deserializer, Serializer};
	use std::time::Duration;

	pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match duration {
			Some(d) => serializer.serialize_str(&format!("{}s", d.as_secs())),
			None => serializer.serialize_none(),
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s: Option<String> = Option::deserialize(deserializer)?;
		match s {
			Some(s) => {
				// Parse duration strings like "30s", "1m", "1h"
				let s = s.trim();
				if let Some(secs) = s.strip_suffix('s') {
					let secs: u64 = secs.parse().map_err(serde::de::Error::custom)?;
					Ok(Some(Duration::from_secs(secs)))
				} else if let Some(mins) = s.strip_suffix('m') {
					let mins: u64 = mins.parse().map_err(serde::de::Error::custom)?;
					Ok(Some(Duration::from_secs(mins * 60)))
				} else if let Some(hours) = s.strip_suffix('h') {
					let hours: u64 = hours.parse().map_err(serde::de::Error::custom)?;
					Ok(Some(Duration::from_secs(hours * 3600)))
				} else {
					Err(serde::de::Error::custom(format!(
						"Invalid duration format: {}",
						s
					)))
				}
			},
			None => Ok(None),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::sync::Arc;
	use tokio::sync::Mutex;

	/// Mock tool executor for testing
	struct MockToolExecutor {
		/// Track tool calls for verification
		calls: Arc<Mutex<Vec<(String, Value)>>>,
		/// Predefined responses for each tool
		responses: HashMap<String, Result<Value, String>>,
		/// Count of compensation calls
		compensation_calls: Arc<AtomicUsize>,
	}

	impl MockToolExecutor {
		fn new() -> Self {
			Self {
				calls: Arc::new(Mutex::new(Vec::new())),
				responses: HashMap::new(),
				compensation_calls: Arc::new(AtomicUsize::new(0)),
			}
		}

		fn with_response(mut self, tool: &str, response: Result<Value, String>) -> Self {
			self.responses.insert(tool.to_string(), response);
			self
		}
	}

	#[async_trait::async_trait]
	impl ToolExecutor for MockToolExecutor {
		async fn execute(&self, tool: &ToolCall, input: Value) -> Result<Value, SagaError> {
			// Track the call
			self.calls.lock().await.push((tool.name.clone(), input));

			// Track compensation calls
			if tool.name.contains("cancel") || tool.name.contains("compensate") {
				self.compensation_calls.fetch_add(1, Ordering::SeqCst);
			}

			// Return predefined response or error
			match self.responses.get(&tool.name) {
				Some(Ok(v)) => Ok(v.clone()),
				Some(Err(e)) => Err(SagaError::ToolError(e.clone())),
				None => Ok(serde_json::json!({"status": "ok"})),
			}
		}
	}

	#[tokio::test]
	async fn test_saga_happy_path() {
		// All steps succeed
		let executor = MockToolExecutor::new()
			.with_response("airline.book", Ok(serde_json::json!({"confirmationNumber": "FL123"})))
			.with_response("hotel.reserve", Ok(serde_json::json!({"confirmationNumber": "HT456"})));

		let saga_executor = SagaExecutor::new(executor);

		let saga = Saga {
			steps: vec![
				SagaStep {
					id: "flight".to_string(),
					name: "Book flight".to_string(),
					action: ToolCall {
						name: "airline.book".to_string(),
						arguments: Value::Null,
					},
					compensate: Some(ToolCall {
						name: "airline.cancel".to_string(),
						arguments: Value::Null,
					}),
					input: None,
				},
				SagaStep {
					id: "hotel".to_string(),
					name: "Book hotel".to_string(),
					action: ToolCall {
						name: "hotel.reserve".to_string(),
						arguments: Value::Null,
					},
					compensate: Some(ToolCall {
						name: "hotel.cancel".to_string(),
						arguments: Value::Null,
					}),
					input: None,
				},
			],
			timeout: None,
			output: None,
		};

		let result = saga_executor.execute(&saga, Value::Null).await.unwrap();

		assert!(matches!(result.status, SagaStatus::Completed));
		assert_eq!(result.step_results.len(), 2);
		assert_eq!(
			result.step_results.get("flight").unwrap(),
			&serde_json::json!({"confirmationNumber": "FL123"})
		);
		assert_eq!(
			result.step_results.get("hotel").unwrap(),
			&serde_json::json!({"confirmationNumber": "HT456"})
		);
	}

	#[tokio::test]
	async fn test_saga_compensation_on_failure() {
		// Step 3 fails, compensates 2,1
		let executor = MockToolExecutor::new()
			.with_response("step1.action", Ok(serde_json::json!({"result": "1"})))
			.with_response("step2.action", Ok(serde_json::json!({"result": "2"})))
			.with_response("step3.action", Err("Step 3 failed!".to_string()));

		let saga_executor = SagaExecutor::new(executor);

		let saga = Saga {
			steps: vec![
				SagaStep {
					id: "step1".to_string(),
					name: "Step 1".to_string(),
					action: ToolCall {
						name: "step1.action".to_string(),
						arguments: Value::Null,
					},
					compensate: Some(ToolCall {
						name: "step1.compensate".to_string(),
						arguments: Value::Null,
					}),
					input: None,
				},
				SagaStep {
					id: "step2".to_string(),
					name: "Step 2".to_string(),
					action: ToolCall {
						name: "step2.action".to_string(),
						arguments: Value::Null,
					},
					compensate: Some(ToolCall {
						name: "step2.compensate".to_string(),
						arguments: Value::Null,
					}),
					input: None,
				},
				SagaStep {
					id: "step3".to_string(),
					name: "Step 3".to_string(),
					action: ToolCall {
						name: "step3.action".to_string(),
						arguments: Value::Null,
					},
					compensate: Some(ToolCall {
						name: "step3.compensate".to_string(),
						arguments: Value::Null,
					}),
					input: None,
				},
			],
			timeout: None,
			output: None,
		};

		let result = saga_executor.execute(&saga, Value::Null).await.unwrap();

		// Should have failed on step3
		match &result.status {
			SagaStatus::Failed { step, error } => {
				assert_eq!(step, "step3");
				assert!(error.contains("Step 3 failed"));
			},
			_ => panic!("Expected SagaStatus::Failed"),
		}

		// Compensation should have run for step1 and step2 (2 calls)
		let calls = saga_executor.tool_executor.calls.lock().await;
		let compensation_count = calls
			.iter()
			.filter(|(name, _)| name.contains("compensate"))
			.count();
		assert_eq!(compensation_count, 2);
	}

	#[tokio::test]
	async fn test_saga_partial_compensation() {
		// Some steps have no compensate
		let executor = MockToolExecutor::new()
			.with_response("step1.action", Ok(serde_json::json!({"result": "1"})))
			.with_response("step2.action", Ok(serde_json::json!({"result": "2"})))
			.with_response("step3.action", Err("Failed".to_string()));

		let saga_executor = SagaExecutor::new(executor);

		let saga = Saga {
			steps: vec![
				SagaStep {
					id: "step1".to_string(),
					name: "Step 1".to_string(),
					action: ToolCall {
						name: "step1.action".to_string(),
						arguments: Value::Null,
					},
					compensate: Some(ToolCall {
						name: "step1.compensate".to_string(),
						arguments: Value::Null,
					}),
					input: None,
				},
				SagaStep {
					id: "step2".to_string(),
					name: "Step 2".to_string(),
					action: ToolCall {
						name: "step2.action".to_string(),
						arguments: Value::Null,
					},
					compensate: None, // No compensation for step 2
					input: None,
				},
				SagaStep {
					id: "step3".to_string(),
					name: "Step 3".to_string(),
					action: ToolCall {
						name: "step3.action".to_string(),
						arguments: Value::Null,
					},
					compensate: None,
					input: None,
				},
			],
			timeout: None,
			output: None,
		};

		let result = saga_executor.execute(&saga, Value::Null).await.unwrap();

		// Should have failed
		assert!(matches!(result.status, SagaStatus::Failed { .. }));

		// Only step1 should have compensation called (step2 has no compensate)
		let calls = saga_executor.tool_executor.calls.lock().await;
		let compensation_count = calls
			.iter()
			.filter(|(name, _)| name.contains("compensate"))
			.count();
		assert_eq!(compensation_count, 1);
	}

	#[tokio::test]
	async fn test_saga_with_step_bindings() {
		// Steps reference previous step outputs
		let executor = MockToolExecutor::new()
			.with_response(
				"airline.book",
				Ok(serde_json::json!({"confirmationNumber": "FL123", "arrivalTime": "14:00"})),
			)
			.with_response(
				"hotel.reserve",
				Ok(serde_json::json!({"confirmationNumber": "HT456", "address": "123 Main St"})),
			)
			.with_response(
				"rental.book",
				Ok(serde_json::json!({"confirmationNumber": "CR789"})),
			);

		let saga_executor = SagaExecutor::new(executor);

		let saga = Saga {
			steps: vec![
				SagaStep {
					id: "flight".to_string(),
					name: "Book flight".to_string(),
					action: ToolCall {
						name: "airline.book".to_string(),
						arguments: Value::Null,
					},
					compensate: None,
					input: Some(InputBinding::Path {
						path: "$.input.flight".to_string(),
					}),
				},
				SagaStep {
					id: "hotel".to_string(),
					name: "Book hotel".to_string(),
					action: ToolCall {
						name: "hotel.reserve".to_string(),
						arguments: Value::Null,
					},
					compensate: None,
					input: Some(InputBinding::Path {
						path: "$.input.hotel".to_string(),
					}),
				},
				SagaStep {
					id: "car".to_string(),
					name: "Book car rental".to_string(),
					action: ToolCall {
						name: "rental.book".to_string(),
						arguments: Value::Null,
					},
					compensate: None,
					input: Some(InputBinding::Object(HashMap::from([
						(
							"flightArrival".to_string(),
							InputBinding::Path {
								path: "$.steps.flight.arrivalTime".to_string(),
							},
						),
						(
							"hotelAddress".to_string(),
							InputBinding::Path {
								path: "$.steps.hotel.address".to_string(),
							},
						),
					]))),
				},
			],
			timeout: None,
			output: None,
		};

		let input = serde_json::json!({
			"flight": {"destination": "NYC"},
			"hotel": {"city": "NYC"}
		});

		let result = saga_executor.execute(&saga, input).await.unwrap();

		assert!(matches!(result.status, SagaStatus::Completed));

		// Check that the car rental step received the bound inputs
		let calls = saga_executor.tool_executor.calls.lock().await;
		let car_call = calls.iter().find(|(name, _)| name == "rental.book").unwrap();
		let car_input = &car_call.1;
		assert_eq!(car_input.get("flightArrival").unwrap(), "14:00");
		assert_eq!(car_input.get("hotelAddress").unwrap(), "123 Main St");
	}

	#[tokio::test]
	async fn test_saga_timeout() {
		// Create an executor that delays
		struct SlowExecutor;

		#[async_trait::async_trait]
		impl ToolExecutor for SlowExecutor {
			async fn execute(&self, _tool: &ToolCall, _input: Value) -> Result<Value, SagaError> {
				tokio::time::sleep(Duration::from_secs(10)).await;
				Ok(Value::Null)
			}
		}

		let saga_executor = SagaExecutor::new(SlowExecutor);

		let saga = Saga {
			steps: vec![SagaStep {
				id: "slow".to_string(),
				name: "Slow step".to_string(),
				action: ToolCall {
					name: "slow.action".to_string(),
					arguments: Value::Null,
				},
				compensate: None,
				input: None,
			}],
			timeout: Some(Duration::from_millis(100)),
			output: None,
		};

		let result = saga_executor.execute(&saga, Value::Null).await.unwrap();

		assert!(matches!(result.status, SagaStatus::TimedOut));
	}

	#[tokio::test]
	async fn test_saga_output_binding() {
		// Custom output construction
		let executor = MockToolExecutor::new()
			.with_response("airline.book", Ok(serde_json::json!({"confirmationNumber": "FL123"})))
			.with_response("hotel.reserve", Ok(serde_json::json!({"confirmationNumber": "HT456"})));

		let saga_executor = SagaExecutor::new(executor);

		let saga = Saga {
			steps: vec![
				SagaStep {
					id: "flight".to_string(),
					name: "Book flight".to_string(),
					action: ToolCall {
						name: "airline.book".to_string(),
						arguments: Value::Null,
					},
					compensate: None,
					input: None,
				},
				SagaStep {
					id: "hotel".to_string(),
					name: "Book hotel".to_string(),
					action: ToolCall {
						name: "hotel.reserve".to_string(),
						arguments: Value::Null,
					},
					compensate: None,
					input: None,
				},
			],
			timeout: None,
			output: Some(OutputBinding {
				fields: HashMap::from([
					(
						"flightConfirmation".to_string(),
						PathBinding {
							path: "$.steps.flight.confirmationNumber".to_string(),
						},
					),
					(
						"hotelConfirmation".to_string(),
						PathBinding {
							path: "$.steps.hotel.confirmationNumber".to_string(),
						},
					),
				]),
			}),
		};

		let result = saga_executor.execute(&saga, Value::Null).await.unwrap();

		assert!(matches!(result.status, SagaStatus::Completed));
		assert_eq!(result.output.get("flightConfirmation").unwrap(), "FL123");
		assert_eq!(result.output.get("hotelConfirmation").unwrap(), "HT456");
	}

	#[test]
	fn test_saga_serialization() {
		let saga = Saga {
			steps: vec![SagaStep {
				id: "flight".to_string(),
				name: "Book flight".to_string(),
				action: ToolCall {
					name: "airline.book".to_string(),
					arguments: serde_json::json!({"destination": "NYC"}),
				},
				compensate: Some(ToolCall {
					name: "airline.cancel".to_string(),
					arguments: Value::Null,
				}),
				input: Some(InputBinding::Path {
					path: "$.input.flight".to_string(),
				}),
			}],
			timeout: Some(Duration::from_secs(30)),
			output: None,
		};

		let json = serde_json::to_string_pretty(&saga).unwrap();
		let deserialized: Saga = serde_json::from_str(&json).unwrap();

		assert_eq!(deserialized.steps.len(), 1);
		assert_eq!(deserialized.steps[0].id, "flight");
		assert_eq!(deserialized.timeout, Some(Duration::from_secs(30)));
	}
}
