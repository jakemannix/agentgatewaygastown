//! TDD tests for WireTap pattern implementation.
//!
//! Test cases from the spec:
//! - test_wiretap_after - Taps after main execution
//! - test_wiretap_before - Taps before main execution
//! - test_wiretap_both - Taps before and after
//! - test_wiretap_multiple_targets - Multiple tap targets
//! - test_wiretap_with_transform - Transform before tapping (future)
//! - test_wiretap_fire_and_forget - Tap failures don't affect main

use super::*;
use crate::types::agent::SimpleBackendReference;

/// Test that WireTap with default TapPoint::After taps after execution
#[test]
fn test_wiretap_after() {
	let target = TapTarget {
		backend: SimpleBackendReference::Backend("audit-service".into()),
		percentage: 1.0,
	};
	let wiretap = WireTap::after(vec![target]);

	assert_eq!(wiretap.tap_point, TapPoint::After);
	assert!(!wiretap.should_tap_before());
	assert!(wiretap.should_tap_after());
}

/// Test that WireTap with TapPoint::Before taps before execution
#[test]
fn test_wiretap_before() {
	let target = TapTarget {
		backend: SimpleBackendReference::Backend("pre-processor".into()),
		percentage: 1.0,
	};
	let wiretap = WireTap::before(vec![target]);

	assert_eq!(wiretap.tap_point, TapPoint::Before);
	assert!(wiretap.should_tap_before());
	assert!(!wiretap.should_tap_after());
}

/// Test that WireTap with TapPoint::Both taps before and after
#[test]
fn test_wiretap_both() {
	let target = TapTarget {
		backend: SimpleBackendReference::Backend("full-audit".into()),
		percentage: 1.0,
	};
	let wiretap = WireTap::both(vec![target]);

	assert_eq!(wiretap.tap_point, TapPoint::Both);
	assert!(wiretap.should_tap_before());
	assert!(wiretap.should_tap_after());
}

/// Test that WireTap supports multiple tap targets
#[test]
fn test_wiretap_multiple_targets() {
	let targets = vec![
		TapTarget {
			backend: SimpleBackendReference::Backend("audit-service".into()),
			percentage: 1.0,
		},
		TapTarget {
			backend: SimpleBackendReference::Backend("metrics-service".into()),
			percentage: 0.5, // 50% sampling
		},
		TapTarget {
			backend: SimpleBackendReference::Backend("debug-service".into()),
			percentage: 0.1, // 10% sampling for debug
		},
	];
	let wiretap = WireTap::new(targets);

	assert_eq!(wiretap.targets.len(), 3);

	// Check percentages
	assert_eq!(wiretap.targets[0].percentage, 1.0);
	assert_eq!(wiretap.targets[1].percentage, 0.5);
	assert_eq!(wiretap.targets[2].percentage, 0.1);

	// Verify backends are Backend variants
	assert!(matches!(
		&wiretap.targets[0].backend,
		SimpleBackendReference::Backend(_)
	));
	assert!(matches!(
		&wiretap.targets[1].backend,
		SimpleBackendReference::Backend(_)
	));
	assert!(matches!(
		&wiretap.targets[2].backend,
		SimpleBackendReference::Backend(_)
	));
}

/// Test percentage sampling edge cases
#[test]
fn test_wiretap_percentage_sampling() {
	// 100% should always sample
	let target_100 = TapTarget {
		backend: SimpleBackendReference::Backend("always".into()),
		percentage: 1.0,
	};
	for _ in 0..100 {
		assert!(WireTap::should_sample(&target_100));
	}

	// 0% should never sample
	let target_0 = TapTarget {
		backend: SimpleBackendReference::Backend("never".into()),
		percentage: 0.0,
	};
	for _ in 0..100 {
		assert!(!WireTap::should_sample(&target_0));
	}

	// Values > 1.0 should always sample
	let target_over = TapTarget {
		backend: SimpleBackendReference::Backend("over".into()),
		percentage: 1.5,
	};
	for _ in 0..100 {
		assert!(WireTap::should_sample(&target_over));
	}

	// Negative values should never sample
	let target_neg = TapTarget {
		backend: SimpleBackendReference::Backend("negative".into()),
		percentage: -0.5,
	};
	for _ in 0..100 {
		assert!(!WireTap::should_sample(&target_neg));
	}
}

/// Test default tap point is After
#[test]
fn test_wiretap_default_tap_point() {
	let tap_point = TapPoint::default();
	assert_eq!(tap_point, TapPoint::After);
}

/// Test serialization/deserialization
#[test]
fn test_wiretap_serde() {
	let wiretap = WireTap {
		targets: vec![TapTarget {
			backend: SimpleBackendReference::Backend("audit".into()),
			percentage: 0.75,
		}],
		tap_point: TapPoint::Both,
	};

	let json = serde_json::to_string(&wiretap).expect("serialize");
	let deserialized: WireTap = serde_json::from_str(&json).expect("deserialize");

	assert_eq!(deserialized.tap_point, TapPoint::Both);
	assert_eq!(deserialized.targets.len(), 1);
	assert_eq!(deserialized.targets[0].percentage, 0.75);
}

/// Test tap point serialization uses snake_case
#[test]
fn test_tap_point_serde() {
	// Test each variant
	assert_eq!(
		serde_json::to_string(&TapPoint::Before).unwrap(),
		"\"before\""
	);
	assert_eq!(
		serde_json::to_string(&TapPoint::After).unwrap(),
		"\"after\""
	);
	assert_eq!(serde_json::to_string(&TapPoint::Both).unwrap(), "\"both\"");

	// Test deserialization
	assert_eq!(
		serde_json::from_str::<TapPoint>("\"before\"").unwrap(),
		TapPoint::Before
	);
	assert_eq!(
		serde_json::from_str::<TapPoint>("\"after\"").unwrap(),
		TapPoint::After
	);
	assert_eq!(
		serde_json::from_str::<TapPoint>("\"both\"").unwrap(),
		TapPoint::Both
	);
}
