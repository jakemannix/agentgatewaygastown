//! Saga pattern executor implementation.
//!
//! Provides the runtime execution of sagas, including forward execution,
//! compensation on failure, and timeout handling.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::saga::types::{InputBinding, OutputBinding, Saga, SagaStep, StepAction};

/// Errors that can occur during saga execution.
#[derive(Debug, Error)]
pub enum SagaError {
    #[error("Step '{step_id}' failed: {message}")]
    StepFailed { step_id: String, message: String },

    #[error("Compensation failed for step '{step_id}': {message}")]
    CompensationFailed { step_id: String, message: String },

    #[error("Saga timed out after {duration:?}")]
    Timeout { duration: Duration },

    #[error("Invalid binding: {0}")]
    InvalidBinding(String),

    #[error("Step '{step_id}' not found")]
    StepNotFound { step_id: String },

    #[error("JSONPath error: {0}")]
    JsonPath(String),
}

/// Result of a successfully completed step.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_id: String,
    pub output: serde_json::Value,
    pub duration: Duration,
}

/// Result of a successful saga execution.
#[derive(Debug)]
pub struct SagaResult {
    /// The final output as determined by the saga's output binding
    pub output: serde_json::Value,
    /// Individual step results keyed by step id
    pub step_results: HashMap<String, StepResult>,
    /// Total execution time
    pub duration: Duration,
}

/// Current status of a saga execution.
#[derive(Debug, Clone)]
pub enum SagaStatus {
    /// Currently executing forward steps
    Executing { current_step: usize },
    /// A step failed, compensation is in progress
    Compensating {
        failed_step: usize,
        error: String,
        compensating_step: usize,
    },
    /// All steps completed successfully
    Completed,
    /// Compensation completed after a failure
    CompensatedFailure { original_error: String },
    /// The saga timed out
    TimedOut,
}

/// Trait for routing and executing step actions.
///
/// Implementors handle the actual execution of tools, HTTP calls, or backend
/// invocations as specified by `StepAction`.
#[async_trait]
pub trait ActionRouter: Send + Sync {
    /// Execute an action with the given input.
    ///
    /// # Arguments
    /// * `action` - The action to execute
    /// * `input` - Input data for the action
    /// * `timeout` - Optional timeout for this action
    ///
    /// # Returns
    /// The action's output as JSON, or an error.
    async fn execute_action(
        &self,
        action: &StepAction,
        input: serde_json::Value,
        timeout: Option<Duration>,
    ) -> Result<serde_json::Value, SagaError>;
}

/// Executor for running sagas.
pub struct SagaExecutor<R: ActionRouter> {
    router: Arc<R>,
}

impl<R: ActionRouter> SagaExecutor<R> {
    /// Create a new executor with the given action router.
    pub fn new(router: Arc<R>) -> Self {
        Self { router }
    }

    /// Execute a saga to completion.
    ///
    /// Runs each step in order. If a step fails, runs compensation for
    /// all previously completed steps in reverse order.
    pub async fn execute(
        &self,
        saga: Saga,
        input: serde_json::Value,
    ) -> Result<SagaResult, SagaError> {
        let start = Instant::now();
        let saga_timeout = saga.timeout;
        let mut step_results: HashMap<String, StepResult> = HashMap::new();

        info!(
            saga_name = ?saga.name,
            saga_id = ?saga.id,
            step_count = saga.steps.len(),
            "Starting saga execution"
        );

        // Execute steps in order
        for (idx, step) in saga.steps.iter().enumerate() {
            // Check saga-level timeout
            if let Some(timeout) = saga_timeout
                && start.elapsed() > timeout
            {
                warn!(
                    saga_name = ?saga.name,
                    elapsed = ?start.elapsed(),
                    "Saga timed out"
                );
                // Compensate completed steps
                self.compensate(&saga.steps[..idx], &step_results).await;
                return Err(SagaError::Timeout { duration: timeout });
            }

            debug!(
                step_id = %step.id,
                step_name = ?step.name,
                step_index = idx,
                "Executing step"
            );

            // Resolve step input
            let step_input = self.resolve_input_binding(&step.input, &input, &step_results)?;

            // Determine timeout for this step
            let step_timeout = step.timeout.or(saga_timeout);

            // Execute the step
            let step_start = Instant::now();
            let result = self
                .execute_step_with_timeout(&step.action, step_input, step_timeout)
                .await;

            match result {
                Ok(output) => {
                    let duration = step_start.elapsed();
                    info!(
                        step_id = %step.id,
                        duration = ?duration,
                        "Step completed successfully"
                    );
                    step_results.insert(
                        step.id.clone(),
                        StepResult {
                            step_id: step.id.clone(),
                            output,
                            duration,
                        },
                    );
                }
                Err(e) => {
                    error!(
                        step_id = %step.id,
                        error = %e,
                        "Step failed, starting compensation"
                    );
                    // Compensate all completed steps
                    self.compensate(&saga.steps[..idx], &step_results).await;

                    // Preserve Timeout errors, wrap others in StepFailed
                    return match e {
                        SagaError::Timeout { .. } => Err(e),
                        _ => Err(SagaError::StepFailed {
                            step_id: step.id.clone(),
                            message: e.to_string(),
                        }),
                    };
                }
            }
        }

        // Construct output
        let output = self.resolve_output_binding(&saga.output, &step_results)?;

        let duration = start.elapsed();
        info!(
            saga_name = ?saga.name,
            duration = ?duration,
            "Saga completed successfully"
        );

        Ok(SagaResult {
            output,
            step_results,
            duration,
        })
    }

    /// Execute a step action with optional timeout.
    async fn execute_step_with_timeout(
        &self,
        action: &StepAction,
        input: serde_json::Value,
        timeout: Option<Duration>,
    ) -> Result<serde_json::Value, SagaError> {
        match timeout {
            Some(dur) => {
                tokio::time::timeout(dur, self.router.execute_action(action, input, Some(dur)))
                    .await
                    .map_err(|_| SagaError::Timeout { duration: dur })?
            }
            None => self.router.execute_action(action, input, None).await,
        }
    }

    /// Compensate completed steps in reverse order.
    async fn compensate(&self, completed_steps: &[SagaStep], results: &HashMap<String, StepResult>) {
        info!(
            step_count = completed_steps.len(),
            "Starting compensation for completed steps"
        );

        for step in completed_steps.iter().rev() {
            if let Some(compensate_action) = &step.compensate {
                debug!(
                    step_id = %step.id,
                    "Compensating step"
                );

                // Use the step's result as input to compensation
                let comp_input = results
                    .get(&step.id)
                    .map(|r| r.output.clone())
                    .unwrap_or(serde_json::Value::Null);

                // Execute compensation - log errors but don't fail
                match self
                    .router
                    .execute_action(compensate_action, comp_input, step.timeout)
                    .await
                {
                    Ok(_) => {
                        info!(step_id = %step.id, "Compensation succeeded");
                    }
                    Err(e) => {
                        error!(
                            step_id = %step.id,
                            error = %e,
                            "Compensation failed (continuing with remaining compensations)"
                        );
                    }
                }
            } else {
                debug!(
                    step_id = %step.id,
                    "Step has no compensation action, skipping"
                );
            }
        }
    }

    /// Resolve an input binding to a concrete JSON value.
    fn resolve_input_binding(
        &self,
        binding: &Option<InputBinding>,
        saga_input: &serde_json::Value,
        step_results: &HashMap<String, StepResult>,
    ) -> Result<serde_json::Value, SagaError> {
        match binding {
            None => Ok(saga_input.clone()),
            Some(InputBinding::Input { path }) => {
                self.jsonpath_extract(saga_input, path)
            }
            Some(InputBinding::Step { id, path }) => {
                let step_result = step_results
                    .get(id)
                    .ok_or_else(|| SagaError::StepNotFound { step_id: id.clone() })?;

                match path {
                    Some(p) => self.jsonpath_extract(&step_result.output, p),
                    None => Ok(step_result.output.clone()),
                }
            }
            Some(InputBinding::Merge(bindings)) => {
                let mut merged = serde_json::Map::new();
                for (i, binding) in bindings.iter().enumerate() {
                    let value = self.resolve_input_binding(&Some(binding.clone()), saga_input, step_results)?;
                    if let serde_json::Value::Object(obj) = value {
                        for (k, v) in obj {
                            merged.insert(k, v);
                        }
                    } else {
                        // For non-objects, use index as key
                        merged.insert(format!("_{}", i), value);
                    }
                }
                Ok(serde_json::Value::Object(merged))
            }
            Some(InputBinding::Static(value)) => Ok(value.clone()),
        }
    }

    /// Resolve an output binding to construct the saga result.
    fn resolve_output_binding(
        &self,
        binding: &Option<OutputBinding>,
        step_results: &HashMap<String, StepResult>,
    ) -> Result<serde_json::Value, SagaError> {
        match binding {
            None | Some(OutputBinding::All) => {
                // Return all step results as an object
                let obj: serde_json::Map<String, serde_json::Value> = step_results
                    .iter()
                    .map(|(k, v)| (k.clone(), v.output.clone()))
                    .collect();
                Ok(serde_json::Value::Object(obj))
            }
            Some(OutputBinding::Step { id, path }) => {
                let step_result = step_results
                    .get(id)
                    .ok_or_else(|| SagaError::StepNotFound { step_id: id.clone() })?;

                match path {
                    Some(p) => self.jsonpath_extract(&step_result.output, p),
                    None => Ok(step_result.output.clone()),
                }
            }
            Some(OutputBinding::Object(fields)) => {
                let mut obj = serde_json::Map::new();
                for (key, binding) in fields {
                    let value = self.resolve_output_binding(&Some(binding.clone()), step_results)?;
                    obj.insert(key.clone(), value);
                }
                Ok(serde_json::Value::Object(obj))
            }
        }
    }

    /// Extract a value from JSON using a simple JSONPath expression.
    ///
    /// Supports basic paths like "$.field" or "$.field.nested".
    fn jsonpath_extract(
        &self,
        value: &serde_json::Value,
        path: &str,
    ) -> Result<serde_json::Value, SagaError> {
        // Simple JSONPath implementation - just handle $.field.nested patterns
        let path = path.strip_prefix("$.").unwrap_or(path);

        if path.is_empty() {
            return Ok(value.clone());
        }

        let mut current = value;
        for segment in path.split('.') {
            current = current
                .get(segment)
                .ok_or_else(|| SagaError::JsonPath(format!("Path '{}' not found", path)))?;
        }
        Ok(current.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Mock router that records calls and returns predefined responses.
    struct MockRouter {
        responses: Mutex<Vec<Result<serde_json::Value, String>>>,
        calls: Mutex<Vec<(StepAction, serde_json::Value)>>,
    }

    impl MockRouter {
        fn new(responses: Vec<Result<serde_json::Value, String>>) -> Self {
            Self {
                responses: Mutex::new(responses),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }

        fn get_calls(&self) -> Vec<(StepAction, serde_json::Value)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ActionRouter for MockRouter {
        async fn execute_action(
            &self,
            action: &StepAction,
            input: serde_json::Value,
            _timeout: Option<Duration>,
        ) -> Result<serde_json::Value, SagaError> {
            self.calls.lock().unwrap().push((action.clone(), input));

            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Ok(serde_json::json!({"default": "response"}));
            }

            responses.remove(0).map_err(|msg| SagaError::StepFailed {
                step_id: "unknown".to_string(),
                message: msg,
            })
        }
    }

    #[tokio::test]
    async fn test_saga_happy_path() {
        let router = Arc::new(MockRouter::new(vec![
            Ok(serde_json::json!({"confirmation": "FL123"})),
            Ok(serde_json::json!({"reservation": "HT456"})),
        ]));

        let executor = SagaExecutor::new(router.clone());

        let saga = Saga {
            id: Some("test".to_string()),
            name: Some("Happy Path Test".to_string()),
            steps: vec![
                SagaStep {
                    id: "flight".to_string(),
                    name: Some("Book flight".to_string()),
                    action: StepAction::Tool { name: "airline.book".to_string() },
                    compensate: Some(StepAction::Tool { name: "airline.cancel".to_string() }),
                    input: Some(InputBinding::Input { path: "$.flight".to_string() }),
                    timeout: None,
                },
                SagaStep {
                    id: "hotel".to_string(),
                    name: Some("Book hotel".to_string()),
                    action: StepAction::Tool { name: "hotel.reserve".to_string() },
                    compensate: Some(StepAction::Tool { name: "hotel.cancel".to_string() }),
                    input: Some(InputBinding::Input { path: "$.hotel".to_string() }),
                    timeout: None,
                },
            ],
            output: Some(OutputBinding::All),
            timeout: None,
        };

        let input = serde_json::json!({
            "flight": { "from": "SFO", "to": "JFK" },
            "hotel": { "city": "NYC", "nights": 3 }
        });

        let result = executor.execute(saga, input).await.unwrap();

        // Verify both steps executed
        assert_eq!(router.call_count(), 2);
        assert!(result.step_results.contains_key("flight"));
        assert!(result.step_results.contains_key("hotel"));

        // Verify output contains both results
        let output = result.output.as_object().unwrap();
        assert!(output.contains_key("flight"));
        assert!(output.contains_key("hotel"));
    }

    #[tokio::test]
    async fn test_saga_compensation_on_failure() {
        let router = Arc::new(MockRouter::new(vec![
            Ok(serde_json::json!({"confirmation": "FL123"})),
            Ok(serde_json::json!({"reservation": "HT456"})),
            Err("Payment declined".to_string()),
            Ok(serde_json::json!({"cancelled": true})), // hotel compensation
            Ok(serde_json::json!({"cancelled": true})), // flight compensation
        ]));

        let executor = SagaExecutor::new(router.clone());

        let saga = Saga {
            id: None,
            name: Some("Compensation Test".to_string()),
            steps: vec![
                SagaStep {
                    id: "flight".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "airline.book".to_string() },
                    compensate: Some(StepAction::Tool { name: "airline.cancel".to_string() }),
                    input: None,
                    timeout: None,
                },
                SagaStep {
                    id: "hotel".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "hotel.reserve".to_string() },
                    compensate: Some(StepAction::Tool { name: "hotel.cancel".to_string() }),
                    input: None,
                    timeout: None,
                },
                SagaStep {
                    id: "payment".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "payment.charge".to_string() },
                    compensate: Some(StepAction::Tool { name: "payment.refund".to_string() }),
                    input: None,
                    timeout: None,
                },
            ],
            output: None,
            timeout: None,
        };

        let result = executor.execute(saga, serde_json::json!({})).await;

        // Should fail
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SagaError::StepFailed { step_id, .. } if step_id == "payment"));

        // Should have called: flight, hotel, payment (fail), hotel-cancel, flight-cancel
        // = 5 calls total
        assert_eq!(router.call_count(), 5);

        // Verify compensation was called in reverse order
        let calls = router.get_calls();
        // Last two calls should be compensations
        if let StepAction::Tool { name } = &calls[3].0 {
            assert_eq!(name, "hotel.cancel");
        }
        if let StepAction::Tool { name } = &calls[4].0 {
            assert_eq!(name, "airline.cancel");
        }
    }

    #[tokio::test]
    async fn test_saga_partial_compensation() {
        // Test when some steps have no compensation
        let router = Arc::new(MockRouter::new(vec![
            Ok(serde_json::json!({"step1": "done"})),
            Ok(serde_json::json!({"step2": "done"})),
            Err("Step 3 failed".to_string()),
            // Only step1 has compensation, so only one compensation call
            Ok(serde_json::json!({"compensated": true})),
        ]));

        let executor = SagaExecutor::new(router.clone());

        let saga = Saga {
            id: None,
            name: None,
            steps: vec![
                SagaStep {
                    id: "step1".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "action1".to_string() },
                    compensate: Some(StepAction::Tool { name: "undo1".to_string() }),
                    input: None,
                    timeout: None,
                },
                SagaStep {
                    id: "step2".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "action2".to_string() },
                    compensate: None, // No compensation
                    input: None,
                    timeout: None,
                },
                SagaStep {
                    id: "step3".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "action3".to_string() },
                    compensate: None,
                    input: None,
                    timeout: None,
                },
            ],
            output: None,
            timeout: None,
        };

        let result = executor.execute(saga, serde_json::json!({})).await;
        assert!(result.is_err());

        // 3 forward calls + 1 compensation (only step1 has it)
        assert_eq!(router.call_count(), 4);
    }

    #[tokio::test]
    async fn test_saga_with_step_bindings() {
        let router = Arc::new(MockRouter::new(vec![
            Ok(serde_json::json!({"user_id": "U123", "name": "Alice"})),
            Ok(serde_json::json!({"order_id": "O456"})),
        ]));

        let executor = SagaExecutor::new(router.clone());

        let saga = Saga {
            id: None,
            name: None,
            steps: vec![
                SagaStep {
                    id: "create_user".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "user.create".to_string() },
                    compensate: None,
                    input: Some(InputBinding::Input { path: "$.user".to_string() }),
                    timeout: None,
                },
                SagaStep {
                    id: "create_order".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "order.create".to_string() },
                    compensate: None,
                    // Reference the user_id from previous step
                    input: Some(InputBinding::Step {
                        id: "create_user".to_string(),
                        path: Some("$.user_id".to_string()),
                    }),
                    timeout: None,
                },
            ],
            output: None,
            timeout: None,
        };

        let input = serde_json::json!({
            "user": { "email": "alice@example.com" }
        });

        let result = executor.execute(saga, input).await.unwrap();

        // Verify the second call received the user_id from the first step
        let calls = router.get_calls();
        assert_eq!(calls[1].1, serde_json::json!("U123"));

        assert!(result.step_results.contains_key("create_user"));
        assert!(result.step_results.contains_key("create_order"));
    }

    #[tokio::test]
    async fn test_saga_timeout() {
        // Create a router that delays
        struct SlowRouter;

        #[async_trait]
        impl ActionRouter for SlowRouter {
            async fn execute_action(
                &self,
                _action: &StepAction,
                _input: serde_json::Value,
                _timeout: Option<Duration>,
            ) -> Result<serde_json::Value, SagaError> {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(serde_json::json!({"done": true}))
            }
        }

        let executor = SagaExecutor::new(Arc::new(SlowRouter));

        let saga = Saga {
            id: None,
            name: None,
            steps: vec![
                SagaStep {
                    id: "slow".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "slow.action".to_string() },
                    compensate: None,
                    input: None,
                    timeout: Some(Duration::from_millis(10)), // Very short timeout
                },
            ],
            output: None,
            timeout: None,
        };

        let result = executor.execute(saga, serde_json::json!({})).await;
        assert!(matches!(result, Err(SagaError::Timeout { .. })));
    }

    #[tokio::test]
    async fn test_saga_output_binding() {
        let router = Arc::new(MockRouter::new(vec![
            Ok(serde_json::json!({"flight_conf": "FL123", "price": 450})),
            Ok(serde_json::json!({"hotel_conf": "HT456", "nights": 3})),
        ]));

        let executor = SagaExecutor::new(router.clone());

        let mut output_fields = HashMap::new();
        output_fields.insert(
            "booking".to_string(),
            OutputBinding::Object({
                let mut inner = HashMap::new();
                inner.insert(
                    "flight".to_string(),
                    OutputBinding::Step {
                        id: "flight".to_string(),
                        path: Some("$.flight_conf".to_string()),
                    },
                );
                inner.insert(
                    "hotel".to_string(),
                    OutputBinding::Step {
                        id: "hotel".to_string(),
                        path: Some("$.hotel_conf".to_string()),
                    },
                );
                inner
            }),
        );

        let saga = Saga {
            id: None,
            name: None,
            steps: vec![
                SagaStep {
                    id: "flight".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "book.flight".to_string() },
                    compensate: None,
                    input: None,
                    timeout: None,
                },
                SagaStep {
                    id: "hotel".to_string(),
                    name: None,
                    action: StepAction::Tool { name: "book.hotel".to_string() },
                    compensate: None,
                    input: None,
                    timeout: None,
                },
            ],
            output: Some(OutputBinding::Object(output_fields)),
            timeout: None,
        };

        let result = executor.execute(saga, serde_json::json!({})).await.unwrap();

        // Check the custom output structure
        let booking = &result.output["booking"];
        assert_eq!(booking["flight"], "FL123");
        assert_eq!(booking["hotel"], "HT456");
    }

    #[test]
    fn test_jsonpath_extract() {
        let router = Arc::new(MockRouter::new(vec![]));
        let executor = SagaExecutor::new(router);

        let value = serde_json::json!({
            "user": {
                "name": "Alice",
                "address": {
                    "city": "NYC"
                }
            }
        });

        assert_eq!(
            executor.jsonpath_extract(&value, "$.user.name").unwrap(),
            serde_json::json!("Alice")
        );
        assert_eq!(
            executor.jsonpath_extract(&value, "$.user.address.city").unwrap(),
            serde_json::json!("NYC")
        );
        assert!(executor.jsonpath_extract(&value, "$.nonexistent").is_err());
    }
}
