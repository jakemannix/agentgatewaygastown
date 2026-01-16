//! WireTap pattern implementation.
//!
//! The WireTap pattern sends copies of messages to side channels (like logging,
//! auditing, debugging) without affecting the main flow. This is an Enterprise
//! Integration Pattern.
//!
//! Key characteristics:
//! - Fire-and-forget: Tap failures don't affect main request processing
//! - Configurable tap points: Before, After, or Both
//! - Optional transforms before tapping
//! - Multiple tap targets supported

use crate::types::agent::SimpleBackendReference;
use crate::*;

#[cfg(test)]
#[path = "wiretap_tests.rs"]
mod tests;

/// When to tap the message in the request/response lifecycle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum TapPoint {
	/// Tap before the main operation executes
	Before,
	/// Tap after the main operation completes (default)
	#[default]
	After,
	/// Tap both before and after
	Both,
}

/// A target for wire tapping - where copies of messages should be sent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct TapTarget {
	/// The backend to send tapped messages to
	pub backend: SimpleBackendReference,
	/// Percentage of requests to tap (0.0-1.0, default 1.0 = 100%)
	#[serde(default = "default_percentage")]
	pub percentage: f64,
}

fn default_percentage() -> f64 {
	1.0
}

/// WireTap configuration for sending copies of requests to side channels.
///
/// Unlike RequestMirror, WireTap:
/// - Uses fire-and-forget semantics (failures don't affect main flow)
/// - Supports configurable tap points (before/after/both)
/// - Can tap to multiple targets
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct WireTap {
	/// Targets to send tapped messages to
	pub targets: Vec<TapTarget>,
	/// When to tap the message
	#[serde(default)]
	pub tap_point: TapPoint,
}

impl WireTap {
	/// Create a new WireTap with default settings (tap after, 100% sampling)
	pub fn new(targets: Vec<TapTarget>) -> Self {
		Self {
			targets,
			tap_point: TapPoint::default(),
		}
	}

	/// Create a WireTap that taps before the main operation
	pub fn before(targets: Vec<TapTarget>) -> Self {
		Self {
			targets,
			tap_point: TapPoint::Before,
		}
	}

	/// Create a WireTap that taps after the main operation
	pub fn after(targets: Vec<TapTarget>) -> Self {
		Self {
			targets,
			tap_point: TapPoint::After,
		}
	}

	/// Create a WireTap that taps both before and after
	pub fn both(targets: Vec<TapTarget>) -> Self {
		Self {
			targets,
			tap_point: TapPoint::Both,
		}
	}

	/// Check if this WireTap should execute before the main operation
	pub fn should_tap_before(&self) -> bool {
		matches!(self.tap_point, TapPoint::Before | TapPoint::Both)
	}

	/// Check if this WireTap should execute after the main operation
	pub fn should_tap_after(&self) -> bool {
		matches!(self.tap_point, TapPoint::After | TapPoint::Both)
	}

	/// Returns true if the target should be tapped based on percentage sampling
	pub fn should_sample(target: &TapTarget) -> bool {
		if target.percentage >= 1.0 {
			return true;
		}
		if target.percentage <= 0.0 {
			return false;
		}
		rand::random::<f64>() < target.percentage
	}
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("invalid tap target: {0}")]
	InvalidTapTarget(String),
	#[error("invalid percentage: {0} (must be 0.0-1.0)")]
	InvalidPercentage(f64),
}
