use std::time::Duration;

use serde_json::Value;

use crate::*;

#[cfg(test)]
mod tests;

/// Error type for dead letter execution
#[derive(Debug)]
pub struct ExecutionError {
	message: String,
}

impl ExecutionError {
	pub fn new(message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
		}
	}
}

impl std::fmt::Display for ExecutionError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.message)
	}
}

impl std::error::Error for ExecutionError {}

/// Dead letter policy configuration.
///
/// When an operation fails after all retry attempts, the original request
/// is sent to a dead letter tool for later processing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Eq, PartialEq)]
pub struct Policy {
	/// The tool/destination to send failed requests to
	pub dead_letter_tool: String,

	/// Maximum number of attempts before dead-lettering (default: 1)
	#[serde(default = "default_max_attempts")]
	pub max_attempts: u8,

	/// Optional backoff duration between retry attempts
	#[serde(
		default,
		skip_serializing_if = "Option::is_none",
		with = "serde_dur_option"
	)]
	#[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
	pub backoff: Option<Duration>,

	/// Whether to rethrow the error after dead-lettering (default: false)
	/// If true, returns the error. If false, returns null.
	#[serde(default)]
	pub rethrow: bool,
}

fn default_max_attempts() -> u8 {
	1
}

/// Execute an operation with dead letter handling.
///
/// Attempts the operation up to `max_attempts` times. If all attempts fail,
/// sends the original input to the dead letter tool with error details.
///
/// # Arguments
/// * `policy` - Dead letter policy configuration
/// * `executor` - The inner executor that performs the actual operation
/// * `dead_letter_handler` - Handler for sending to the dead letter destination
/// * `input` - The input to the operation
///
/// # Returns
/// * `Ok(Value)` - The result on success, or `Value::Null` on failure if `rethrow` is false
/// * `Err(ExecutionError)` - The error on failure if `rethrow` is true
pub async fn execute_with_dead_letter<E, D>(
	policy: &Policy,
	executor: &E,
	dead_letter_handler: &D,
	input: Value,
) -> Result<Value, ExecutionError>
where
	E: Executor,
	D: DeadLetterHandler,
{
	let mut last_error: Option<ExecutionError> = None;

	for attempt in 0..policy.max_attempts {
		match executor.execute(&input).await {
			Ok(result) => return Ok(result),
			Err(e) => {
				last_error = Some(e);
				// Apply backoff if not the last attempt
				if attempt + 1 < policy.max_attempts
					&& let Some(backoff) = policy.backoff
				{
					tokio::time::sleep(backoff).await;
				}
			},
		}
	}

	// All attempts failed - send to dead letter
	let payload = serde_json::json!({
		"original_input": input,
		"error": last_error.as_ref().map(|e| e.to_string()),
		"attempts": policy.max_attempts,
		"timestamp": chrono::Utc::now().to_rfc3339(),
	});

	// Send to dead letter handler (ignore errors for now)
	let _ = dead_letter_handler.send(payload).await;

	if policy.rethrow {
		Err(last_error.unwrap_or_else(|| ExecutionError::new("Unknown error")))
	} else {
		Ok(Value::Null)
	}
}

/// Trait for the inner executor
pub trait Executor {
	fn execute(
		&self,
		input: &Value,
	) -> impl std::future::Future<Output = Result<Value, ExecutionError>> + Send;
}

/// Trait for the dead letter handler
pub trait DeadLetterHandler {
	fn send(
		&self,
		payload: Value,
	) -> impl std::future::Future<Output = Result<(), ExecutionError>> + Send;
}
