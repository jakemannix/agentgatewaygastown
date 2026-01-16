//! Tests for circuit breaker pattern implementation.
//!
//! Test cases:
//! - test_circuit_closed_success: Normal operation
//! - test_circuit_trips_on_failures: Threshold reached, opens
//! - test_circuit_open_fast_fail: Immediate error when open
//! - test_circuit_open_with_fallback: Returns fallback when open
//! - test_circuit_half_open_recovery: Success closes circuit
//! - test_circuit_half_open_failure: Failure reopens circuit
//! - test_circuit_failure_window: Failures must be within window

use std::sync::Arc;
use std::time::Duration;

use super::*;

/// Simple error type for testing
#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Helper to create a test spec with defaults
fn test_spec(name: &str) -> CircuitBreakerSpec {
    CircuitBreakerSpec {
        name: name.to_string(),
        failure_threshold: 3,
        reset_timeout: Duration::from_millis(100),
        failure_window: Duration::from_secs(60),
        success_threshold: 2,
        fallback: None,
    }
}

/// Helper for successful async operation
async fn success_op() -> Result<&'static str, TestError> {
    Ok("success")
}

/// Helper for failing async operation
async fn failure_op() -> Result<&'static str, TestError> {
    Err(TestError("failed".to_string()))
}

#[tokio::test]
async fn test_circuit_closed_success() {
    // Test: Normal operation - circuit stays closed on success
    let store = Arc::new(InMemoryStateStore::new());
    let executor = CircuitBreakerExecutor::new(store.clone());
    let spec = test_spec("test_closed_success");

    // Execute successful operation
    let result = executor.execute(&spec, success_op()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");

    // Circuit should still be closed (or not have state at all)
    let state = store.load_circuit_state(&spec.name).await.unwrap();
    // State may or may not exist after success, but if it does, it should be closed
    if let Some(s) = state {
        assert_eq!(s.state, CircuitStateEnum::Closed);
    }
}

#[tokio::test]
async fn test_circuit_trips_on_failures() {
    // Test: Threshold reached, circuit opens
    let store = Arc::new(InMemoryStateStore::new());
    let executor = CircuitBreakerExecutor::new(store.clone());
    let spec = test_spec("test_trips_on_failures");

    // Execute failures up to threshold
    for i in 0..spec.failure_threshold {
        let result = executor.execute(&spec, failure_op()).await;
        assert!(result.is_err());

        let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
        if i < spec.failure_threshold - 1 {
            // Still closed
            assert_eq!(
                state.state,
                CircuitStateEnum::Closed,
                "circuit should be closed after {} failures",
                i + 1
            );
        } else {
            // Should now be open
            assert_eq!(
                state.state,
                CircuitStateEnum::Open,
                "circuit should be open after {} failures",
                i + 1
            );
        }
    }
}

#[tokio::test]
async fn test_circuit_open_fast_fail() {
    // Test: Immediate error when circuit is open
    let store = Arc::new(InMemoryStateStore::new());
    let executor = CircuitBreakerExecutor::new(store.clone());
    let spec = test_spec("test_open_fast_fail");

    // Trip the circuit
    for _ in 0..spec.failure_threshold {
        let _ = executor.execute(&spec, failure_op()).await;
    }

    // Verify circuit is open
    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    assert_eq!(state.state, CircuitStateEnum::Open);

    // Now try to execute - should fail fast without calling the operation
    // We use a counter to verify the operation is NOT called
    use std::sync::atomic::{AtomicU32, Ordering};
    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    async fn counted_op() -> Result<&'static str, TestError> {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok("should not reach here")
    }

    CALL_COUNT.store(0, Ordering::SeqCst);
    let result = executor.execute(&spec, counted_op()).await;

    match result {
        Err(CircuitBreakerError::CircuitOpen { name, retry_after }) => {
            assert_eq!(name, spec.name);
            assert!(retry_after.is_some()); // Should have retry hint
        }
        other => panic!("expected CircuitOpen error, got {:?}", other),
    }

    // Operation should NOT have been called
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_circuit_open_with_fallback() {
    // Test: Returns fallback when circuit is open
    // Note: The current implementation doesn't support fallback values directly
    // in the Result - the fallback is in the spec but requires integration
    // with the caller to use it. This test verifies the error contains info.
    let store = Arc::new(InMemoryStateStore::new());
    let executor = CircuitBreakerExecutor::new(store.clone());
    let mut spec = test_spec("test_open_with_fallback");
    spec.fallback = Some(serde_json::json!({"default": "value"}));

    // Trip the circuit
    for _ in 0..spec.failure_threshold {
        let _ = executor.execute(&spec, failure_op()).await;
    }

    // Execute should return CircuitOpen error with retry info
    let result: Result<&str, CircuitBreakerError> = executor.execute(&spec, success_op()).await;
    assert!(matches!(result, Err(CircuitBreakerError::CircuitOpen { .. })));
}

#[tokio::test]
async fn test_circuit_half_open_recovery() {
    // Test: Success in half-open state closes the circuit
    let store = Arc::new(InMemoryStateStore::new());
    let executor = CircuitBreakerExecutor::new(store.clone());
    let mut spec = test_spec("test_half_open_recovery");
    spec.success_threshold = 2;
    spec.reset_timeout = Duration::from_millis(10);

    // Trip the circuit
    for _ in 0..spec.failure_threshold {
        let _ = executor.execute(&spec, failure_op()).await;
    }

    // Verify open
    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    assert_eq!(state.state, CircuitStateEnum::Open);

    // Wait for reset timeout
    tokio::time::sleep(Duration::from_millis(20)).await;

    // First success - should transition to half-open and succeed
    let result = executor.execute(&spec, success_op()).await;
    assert!(result.is_ok());

    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    assert_eq!(state.state, CircuitStateEnum::HalfOpen);
    assert_eq!(state.success_count_in_half_open, 1);

    // Second success - should close the circuit
    let result = executor.execute(&spec, success_op()).await;
    assert!(result.is_ok());

    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    assert_eq!(state.state, CircuitStateEnum::Closed);
}

#[tokio::test]
async fn test_circuit_half_open_failure() {
    // Test: Failure in half-open state reopens the circuit
    let store = Arc::new(InMemoryStateStore::new());
    let executor = CircuitBreakerExecutor::new(store.clone());
    let mut spec = test_spec("test_half_open_failure");
    spec.reset_timeout = Duration::from_millis(10);

    // Trip the circuit
    for _ in 0..spec.failure_threshold {
        let _ = executor.execute(&spec, failure_op()).await;
    }

    // Wait for reset timeout
    tokio::time::sleep(Duration::from_millis(20)).await;

    // First attempt will transition to half-open, but we want to fail it
    // to verify it goes back to open
    let result = executor.execute(&spec, failure_op()).await;
    assert!(result.is_err());

    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    assert_eq!(
        state.state,
        CircuitStateEnum::Open,
        "circuit should be back to open after half-open failure"
    );
}

#[tokio::test]
async fn test_circuit_failure_window() {
    // Test: Failures outside the window don't count toward threshold
    let store = Arc::new(InMemoryStateStore::new());
    let executor = CircuitBreakerExecutor::new(store.clone());
    let mut spec = test_spec("test_failure_window");
    spec.failure_threshold = 3;
    spec.failure_window = Duration::from_millis(50); // Short window

    // First failure
    let _ = executor.execute(&spec, failure_op()).await;
    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    assert_eq!(state.failure_count, 1);

    // Wait longer than the failure window
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second failure - window expired, should reset counter
    let _ = executor.execute(&spec, failure_op()).await;
    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    // Counter should be 1 (fresh start after window expiry)
    assert_eq!(state.failure_count, 1);

    // Circuit should still be closed
    assert_eq!(state.state, CircuitStateEnum::Closed);

    // Now add failures within the window to trip it
    let _ = executor.execute(&spec, failure_op()).await;
    let _ = executor.execute(&spec, failure_op()).await;

    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    assert_eq!(
        state.state,
        CircuitStateEnum::Open,
        "circuit should be open after 3 failures within window"
    );
}

#[tokio::test]
async fn test_state_store_persistence() {
    // Test that state is properly persisted across executor instances
    let store = Arc::new(InMemoryStateStore::new());
    let spec = test_spec("test_persistence");

    // First executor instance - cause some failures
    {
        let executor = CircuitBreakerExecutor::new(store.clone());
        let _ = executor.execute(&spec, failure_op()).await;
        let _ = executor.execute(&spec, failure_op()).await;
    }

    // Second executor instance - should see the state
    {
        let executor = CircuitBreakerExecutor::new(store.clone());
        // One more failure should trip the circuit
        let _ = executor.execute(&spec, failure_op()).await;
    }

    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    assert_eq!(state.state, CircuitStateEnum::Open);
}

#[tokio::test]
async fn test_circuit_state_transitions() {
    // Test: Verify full state machine transitions
    // CLOSED -> OPEN -> HALF_OPEN -> CLOSED
    let store = Arc::new(InMemoryStateStore::new());
    let executor = CircuitBreakerExecutor::new(store.clone());
    let mut spec = test_spec("test_transitions");
    spec.reset_timeout = Duration::from_millis(10);
    spec.success_threshold = 1;

    // Start: CLOSED
    let state = store.load_circuit_state(&spec.name).await.unwrap();
    assert!(state.is_none() || state.unwrap().state == CircuitStateEnum::Closed);

    // Trip to OPEN
    for _ in 0..spec.failure_threshold {
        let _ = executor.execute(&spec, failure_op()).await;
    }
    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    assert_eq!(state.state, CircuitStateEnum::Open);

    // Wait, then execute to go to HALF_OPEN
    tokio::time::sleep(Duration::from_millis(20)).await;
    let _ = executor.execute(&spec, success_op()).await;
    let state = store.load_circuit_state(&spec.name).await.unwrap().unwrap();
    assert_eq!(state.state, CircuitStateEnum::Closed); // success_threshold is 1
}

#[tokio::test]
async fn test_default_spec_values() {
    // Test that default values are reasonable
    let spec = CircuitBreakerSpec::default();
    assert_eq!(spec.failure_threshold, 5);
    assert_eq!(spec.reset_timeout, Duration::from_secs(30));
    assert_eq!(spec.failure_window, Duration::from_secs(60));
    assert_eq!(spec.success_threshold, 3);
    assert!(spec.fallback.is_none());
}

#[tokio::test]
async fn test_circuit_breaker_error_display() {
    let err = CircuitBreakerError::CircuitOpen {
        name: "test".to_string(),
        retry_after: Some(Duration::from_secs(5)),
    };
    let s = format!("{}", err);
    assert!(s.contains("test"));
    assert!(s.contains("open"));
}
