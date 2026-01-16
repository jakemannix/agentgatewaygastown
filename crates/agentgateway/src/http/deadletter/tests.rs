use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use super::*;

/// Mock executor that can be configured to succeed or fail
struct MockExecutor {
	fail_count: AtomicU32,
	success_value: Value,
}

impl MockExecutor {
	fn new(fail_count: u32, success_value: Value) -> Self {
		Self {
			fail_count: AtomicU32::new(fail_count),
			success_value,
		}
	}
}

impl Executor for MockExecutor {
	async fn execute(&self, _input: &Value) -> Result<Value, ExecutionError> {
		let remaining = self.fail_count.fetch_sub(1, Ordering::SeqCst);
		if remaining > 0 {
			Err(ExecutionError::new("Mock execution failed"))
		} else {
			Ok(self.success_value.clone())
		}
	}
}

/// Mock dead letter handler that records what was sent
struct MockDeadLetterHandler {
	received: std::sync::Mutex<Vec<Value>>,
}

impl MockDeadLetterHandler {
	fn new() -> Self {
		Self {
			received: std::sync::Mutex::new(Vec::new()),
		}
	}

	fn get_received(&self) -> Vec<Value> {
		self.received.lock().unwrap().clone()
	}
}

impl DeadLetterHandler for MockDeadLetterHandler {
	async fn send(&self, payload: Value) -> Result<(), ExecutionError> {
		self.received.lock().unwrap().push(payload);
		Ok(())
	}
}

#[tokio::test]
async fn test_dead_letter_success() {
	// When the inner operation succeeds, no dead-lettering should occur
	let policy = Policy {
		dead_letter_tool: "dead_letter_queue".to_string(),
		max_attempts: 3,
		backoff: None,
		rethrow: false,
	};

	let executor = MockExecutor::new(0, json!({"result": "success"}));
	let dead_letter = MockDeadLetterHandler::new();
	let input = json!({"key": "value"});

	let result = execute_with_dead_letter(&policy, &executor, &dead_letter, input.clone()).await;

	assert!(result.is_ok());
	assert_eq!(result.unwrap(), json!({"result": "success"}));
	// No dead letters should have been sent
	assert!(dead_letter.get_received().is_empty());
}

#[tokio::test]
async fn test_dead_letter_on_failure() {
	// When all attempts fail, should send to dead letter
	let policy = Policy {
		dead_letter_tool: "dead_letter_queue".to_string(),
		max_attempts: 3,
		backoff: None,
		rethrow: false,
	};

	let executor = MockExecutor::new(10, json!({"result": "success"})); // Will always fail
	let dead_letter = MockDeadLetterHandler::new();
	let input = json!({"key": "value"});

	let result = execute_with_dead_letter(&policy, &executor, &dead_letter, input.clone()).await;

	// Should return Ok(Null) when rethrow is false
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), Value::Null);

	// Dead letter should have been sent
	let received = dead_letter.get_received();
	assert_eq!(received.len(), 1);
	assert_eq!(received[0]["original_input"], input);
	assert_eq!(received[0]["attempts"], 3);
}

#[tokio::test]
async fn test_dead_letter_with_retry() {
	// When some attempts fail but eventual success, no dead-lettering
	let policy = Policy {
		dead_letter_tool: "dead_letter_queue".to_string(),
		max_attempts: 5,
		backoff: Some(Duration::from_millis(10)),
		rethrow: false,
	};

	// Fail twice, then succeed
	let executor = MockExecutor::new(2, json!({"result": "success after retries"}));
	let dead_letter = MockDeadLetterHandler::new();
	let input = json!({"key": "value"});

	let result = execute_with_dead_letter(&policy, &executor, &dead_letter, input.clone()).await;

	assert!(result.is_ok());
	assert_eq!(result.unwrap(), json!({"result": "success after retries"}));
	// No dead letters - succeeded within max_attempts
	assert!(dead_letter.get_received().is_empty());
}

#[tokio::test]
async fn test_dead_letter_rethrow() {
	// When rethrow is true, should return error after dead-lettering
	let policy = Policy {
		dead_letter_tool: "dead_letter_queue".to_string(),
		max_attempts: 2,
		backoff: None,
		rethrow: true,
	};

	let executor = MockExecutor::new(10, json!({"result": "success"})); // Will always fail
	let dead_letter = MockDeadLetterHandler::new();
	let input = json!({"key": "value"});

	let result = execute_with_dead_letter(&policy, &executor, &dead_letter, input.clone()).await;

	// Should return error when rethrow is true
	assert!(result.is_err());
	let err = result.unwrap_err();
	assert!(err.to_string().contains("Mock execution failed"));

	// Dead letter should still have been sent
	assert_eq!(dead_letter.get_received().len(), 1);
}

#[tokio::test]
async fn test_dead_letter_no_rethrow() {
	// When rethrow is false, should return Null after dead-lettering
	let policy = Policy {
		dead_letter_tool: "dead_letter_queue".to_string(),
		max_attempts: 1,
		backoff: None,
		rethrow: false,
	};

	let executor = MockExecutor::new(10, json!({"result": "success"})); // Will always fail
	let dead_letter = MockDeadLetterHandler::new();
	let input = json!({"key": "value"});

	let result = execute_with_dead_letter(&policy, &executor, &dead_letter, input.clone()).await;

	// Should return Ok(Null) when rethrow is false
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), Value::Null);
}

#[tokio::test]
async fn test_dead_letter_payload() {
	// Verify the dead letter payload structure
	let policy = Policy {
		dead_letter_tool: "dead_letter_queue".to_string(),
		max_attempts: 2,
		backoff: None,
		rethrow: false,
	};

	let executor = MockExecutor::new(10, json!({"result": "success"})); // Will always fail
	let dead_letter = MockDeadLetterHandler::new();
	let input = json!({"request": "data", "nested": {"value": 123}});

	let _ = execute_with_dead_letter(&policy, &executor, &dead_letter, input.clone()).await;

	let received = dead_letter.get_received();
	assert_eq!(received.len(), 1);

	let payload = &received[0];
	// Check required fields
	assert_eq!(payload["original_input"], input);
	assert!(payload.get("error").is_some());
	assert_eq!(payload["attempts"], 2);
	assert!(payload.get("timestamp").is_some());

	// Verify timestamp is RFC3339 format
	let timestamp = payload["timestamp"].as_str().unwrap();
	assert!(chrono::DateTime::parse_from_rfc3339(timestamp).is_ok());
}
