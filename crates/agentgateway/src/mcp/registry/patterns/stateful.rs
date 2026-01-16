// Stateful pattern type definitions for tool compositions
//
// These patterns require external state stores and are not yet implemented
// in the runtime. The IR types are defined so compositions can be parsed
// and validated, with helpful errors when execution is attempted.

use super::{DataBinding, FieldPredicate, StepOperation};
use serde::{Deserialize, Serialize};

// =============================================================================
// Retry Pattern
// =============================================================================

/// RetrySpec - retry with configurable backoff on failure
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrySpec {
    /// The operation to retry
    pub inner: Box<StepOperation>,

    /// Maximum attempts (including initial)
    pub max_attempts: u32,

    /// Backoff strategy
    pub backoff: BackoffStrategy,

    /// Condition to retry (if absent, retry all errors)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_if: Option<FieldPredicate>,

    /// Jitter factor (0.0 - 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jitter: Option<f32>,

    /// Per-attempt timeout in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BackoffStrategy {
    Fixed(FixedBackoff),
    Exponential(ExponentialBackoff),
    Linear(LinearBackoff),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixedBackoff {
    pub delay_ms: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExponentialBackoff {
    pub initial_delay_ms: u32,
    pub max_delay_ms: u32,
    #[serde(default = "default_multiplier")]
    pub multiplier: f32,
}

fn default_multiplier() -> f32 {
    2.0
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearBackoff {
    pub initial_delay_ms: u32,
    pub increment_ms: u32,
    pub max_delay_ms: u32,
}

// =============================================================================
// Timeout Pattern
// =============================================================================

/// TimeoutSpec - enforce maximum execution duration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeoutSpec {
    /// The operation to wrap
    pub inner: Box<StepOperation>,

    /// Timeout duration in milliseconds
    pub duration_ms: u32,

    /// Fallback on timeout (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<Box<StepOperation>>,

    /// Custom error message
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// =============================================================================
// Cache Pattern
// =============================================================================

/// CacheSpec - read-through caching with TTL
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheSpec {
    /// JSONPath expressions to derive cache key
    pub key_paths: Vec<String>,

    /// The operation to cache
    pub inner: Box<StepOperation>,

    /// Store reference name (configured in gateway)
    pub store: String,

    /// TTL in seconds
    pub ttl_seconds: u32,

    /// Stale-while-revalidate window in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_while_revalidate_seconds: Option<u32>,

    /// Condition to cache result (if absent, always cache)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_if: Option<FieldPredicate>,
}

// =============================================================================
// Idempotent Pattern
// =============================================================================

/// IdempotentSpec - prevent duplicate processing
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdempotentSpec {
    /// JSONPath expressions to derive idempotency key
    pub key_paths: Vec<String>,

    /// The operation to wrap
    pub inner: Box<StepOperation>,

    /// Store reference name (configured in gateway)
    pub store: String,

    /// TTL in seconds (None = no expiry)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u32>,

    /// Behavior on duplicate
    #[serde(default)]
    pub on_duplicate: OnDuplicate,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnDuplicate {
    #[default]
    Cached,
    Skip,
    Error,
}

// =============================================================================
// Circuit Breaker Pattern
// =============================================================================

/// CircuitBreakerSpec - fail fast with automatic recovery
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerSpec {
    /// Unique name for this circuit (for state isolation)
    pub name: String,

    /// The protected operation
    pub inner: Box<StepOperation>,

    /// Store for circuit state
    pub store: String,

    /// Number of failures to trip the circuit
    pub failure_threshold: u32,

    /// Window for counting failures (seconds)
    pub failure_window_seconds: u32,

    /// Time to wait before half-open (seconds)
    pub reset_timeout_seconds: u32,

    /// Successes needed in half-open to close (default: 1)
    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,

    /// Fallback when circuit is open (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<Box<StepOperation>>,

    /// Custom failure condition (if absent, any error)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_if: Option<FieldPredicate>,
}

fn default_success_threshold() -> u32 {
    1
}

// =============================================================================
// Dead Letter Pattern
// =============================================================================

/// DeadLetterSpec - capture failures for later processing
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadLetterSpec {
    /// The operation to wrap
    pub inner: Box<StepOperation>,

    /// Tool to invoke on failure
    pub dead_letter_tool: String,

    /// Max attempts before dead-lettering (default: 1)
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Backoff between attempts
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backoff: Option<BackoffStrategy>,

    /// Whether to rethrow after dead-lettering
    #[serde(default)]
    pub rethrow: bool,
}

fn default_max_attempts() -> u32 {
    1
}

// =============================================================================
// Saga Pattern
// =============================================================================

/// SagaSpec - distributed transaction with compensation
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SagaSpec {
    /// Ordered list of saga steps
    pub steps: Vec<SagaStep>,

    /// Store for saga state (for recovery)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<String>,

    /// JSONPath to derive saga instance ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saga_id_path: Option<String>,

    /// Timeout for entire saga in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,

    /// Output binding
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<DataBinding>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SagaStep {
    /// Step identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// The action to perform
    pub action: StepOperation,

    /// Compensating action (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compensate: Option<StepOperation>,

    /// Input binding for this step
    pub input: DataBinding,
}

// =============================================================================
// Claim Check Pattern
// =============================================================================

/// ClaimCheckSpec - externalize large payloads
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimCheckSpec {
    /// Tool to store payload and return reference
    pub store_tool: String,

    /// Tool to retrieve payload from reference
    pub retrieve_tool: String,

    /// Inner operation operating on reference
    pub inner: Box<StepOperation>,

    /// Whether to retrieve original at end
    #[serde(default)]
    pub retrieve_at_end: bool,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_retry_spec() {
        let json = r#"{
            "inner": { "tool": { "name": "flaky_api" } },
            "maxAttempts": 3,
            "backoff": {
                "exponential": {
                    "initialDelayMs": 100,
                    "maxDelayMs": 5000,
                    "multiplier": 2.0
                }
            },
            "jitter": 0.1
        }"#;

        let spec: RetrySpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.max_attempts, 3);
        assert!(matches!(spec.backoff, BackoffStrategy::Exponential(_)));
        assert_eq!(spec.jitter, Some(0.1));
    }

    #[test]
    fn test_parse_timeout_spec() {
        let json = r#"{
            "inner": { "tool": { "name": "slow_tool" } },
            "durationMs": 30000,
            "message": "Operation timed out"
        }"#;

        let spec: TimeoutSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.duration_ms, 30000);
        assert_eq!(spec.message, Some("Operation timed out".to_string()));
    }

    #[test]
    fn test_parse_cache_spec() {
        let json = r#"{
            "keyPaths": ["$.query", "$.filters"],
            "inner": { "tool": { "name": "search" } },
            "store": "result_cache",
            "ttlSeconds": 900
        }"#;

        let spec: CacheSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.key_paths.len(), 2);
        assert_eq!(spec.store, "result_cache");
        assert_eq!(spec.ttl_seconds, 900);
    }

    #[test]
    fn test_parse_circuit_breaker_spec() {
        let json = r#"{
            "name": "payment_api",
            "inner": { "tool": { "name": "payment_service" } },
            "store": "circuit_state",
            "failureThreshold": 5,
            "failureWindowSeconds": 60,
            "resetTimeoutSeconds": 30
        }"#;

        let spec: CircuitBreakerSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.name, "payment_api");
        assert_eq!(spec.failure_threshold, 5);
        assert_eq!(spec.success_threshold, 1); // default
    }

    #[test]
    fn test_parse_saga_spec() {
        let json = r#"{
            "steps": [
                {
                    "id": "step_0",
                    "name": "reserve_inventory",
                    "action": { "tool": { "name": "reserve" } },
                    "compensate": { "tool": { "name": "release" } },
                    "input": { "input": { "path": "$" } }
                }
            ],
            "sagaIdPath": "$.orderId",
            "timeoutMs": 300000
        }"#;

        let spec: SagaSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.steps.len(), 1);
        assert_eq!(spec.saga_id_path, Some("$.orderId".to_string()));
    }
}
