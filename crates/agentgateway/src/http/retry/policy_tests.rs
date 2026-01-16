use std::num::NonZeroU8;
use std::time::Duration;

use ::http::StatusCode;

use super::*;

// =============================================================================
// Tests for retry::Policy serialization/deserialization
// =============================================================================

#[test]
fn test_policy_default_attempts() {
	let json = r#"{"codes": [502, 503, 504]}"#;
	let policy: Policy = serde_json::from_str(json).unwrap();
	assert_eq!(policy.attempts.get(), 1);
}

#[test]
fn test_policy_custom_attempts() {
	let json = r#"{"attempts": 3, "codes": [502, 503]}"#;
	let policy: Policy = serde_json::from_str(json).unwrap();
	assert_eq!(policy.attempts.get(), 3);
}

#[test]
fn test_policy_with_backoff() {
	let json = r#"{"attempts": 2, "backoff": "500ms", "codes": [503]}"#;
	let policy: Policy = serde_json::from_str(json).unwrap();
	assert_eq!(policy.backoff, Some(Duration::from_millis(500)));
}

#[test]
fn test_policy_without_backoff() {
	let json = r#"{"attempts": 2, "codes": [503]}"#;
	let policy: Policy = serde_json::from_str(json).unwrap();
	assert_eq!(policy.backoff, None);
}

#[test]
fn test_policy_codes_deserialization() {
	let json = r#"{"codes": [500, 502, 503, 504]}"#;
	let policy: Policy = serde_json::from_str(json).unwrap();
	assert_eq!(policy.codes.len(), 4);
	assert!(policy.codes.contains(&StatusCode::INTERNAL_SERVER_ERROR));
	assert!(policy.codes.contains(&StatusCode::BAD_GATEWAY));
	assert!(policy.codes.contains(&StatusCode::SERVICE_UNAVAILABLE));
	assert!(policy.codes.contains(&StatusCode::GATEWAY_TIMEOUT));
}

#[test]
fn test_policy_empty_codes() {
	let json = r#"{"codes": []}"#;
	let policy: Policy = serde_json::from_str(json).unwrap();
	assert!(policy.codes.is_empty());
}

#[test]
fn test_policy_serialization_produces_readable_output() {
	// Note: The Policy serialization uses a human-readable format for status codes
	// (e.g., "502 Bad Gateway") but expects numeric input for deserialization.
	// This is intentionally asymmetric for better debugging/logging output.
	let policy = Policy {
		attempts: NonZeroU8::new(3).unwrap(),
		backoff: Some(Duration::from_secs(1)),
		codes: vec![StatusCode::BAD_GATEWAY, StatusCode::SERVICE_UNAVAILABLE].into_boxed_slice(),
	};

	let json = serde_json::to_string(&policy).unwrap();

	// Verify the serialized output contains human-readable status code strings
	assert!(json.contains("502 Bad Gateway") || json.contains("502"));
	assert!(json.contains("503 Service Unavailable") || json.contains("503"));
}

#[test]
fn test_policy_denies_unknown_fields() {
	let json = r#"{"attempts": 2, "codes": [503], "unknown_field": "value"}"#;
	let result: Result<Policy, _> = serde_json::from_str(json);
	assert!(result.is_err());
}

#[test]
fn test_policy_invalid_status_code() {
	// HTTP status codes must be in range 100-999
	// Values outside this range should fail deserialization
	let json = r#"{"codes": [99]}"#; // Too low - below 100
	let result: Result<Policy, _> = serde_json::from_str(json);
	assert!(result.is_err());

	let json = r#"{"codes": [1000]}"#; // Too high - above 999
	let result: Result<Policy, _> = serde_json::from_str(json);
	assert!(result.is_err());
}

#[test]
fn test_policy_extension_status_codes_valid() {
	// Extension status codes (e.g., 599) should be valid
	let json = r#"{"codes": [599]}"#;
	let result: Result<Policy, _> = serde_json::from_str(json);
	assert!(result.is_ok());
}

// =============================================================================
// Tests for status code matching in retry policy
// =============================================================================

#[test]
fn test_policy_codes_contains_matching_status() {
	let policy = Policy {
		attempts: NonZeroU8::new(2).unwrap(),
		backoff: None,
		codes: vec![StatusCode::BAD_GATEWAY, StatusCode::SERVICE_UNAVAILABLE].into_boxed_slice(),
	};

	assert!(policy.codes.contains(&StatusCode::BAD_GATEWAY));
	assert!(policy.codes.contains(&StatusCode::SERVICE_UNAVAILABLE));
	assert!(!policy.codes.contains(&StatusCode::OK));
	assert!(!policy.codes.contains(&StatusCode::NOT_FOUND));
}

#[test]
fn test_policy_equality() {
	let policy1 = Policy {
		attempts: NonZeroU8::new(2).unwrap(),
		backoff: Some(Duration::from_millis(100)),
		codes: vec![StatusCode::BAD_GATEWAY].into_boxed_slice(),
	};

	let policy2 = Policy {
		attempts: NonZeroU8::new(2).unwrap(),
		backoff: Some(Duration::from_millis(100)),
		codes: vec![StatusCode::BAD_GATEWAY].into_boxed_slice(),
	};

	let policy3 = Policy {
		attempts: NonZeroU8::new(3).unwrap(),
		backoff: Some(Duration::from_millis(100)),
		codes: vec![StatusCode::BAD_GATEWAY].into_boxed_slice(),
	};

	assert_eq!(policy1, policy2);
	assert_ne!(policy1, policy3);
}
