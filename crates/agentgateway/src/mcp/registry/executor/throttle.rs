// Throttle pattern executor
//
// Implements rate limiting for tool invocations with multiple strategies:
// - SlidingWindow: Accurate rate limiting with sliding time window
// - TokenBucket: Allows bursts up to bucket capacity
// - FixedWindow: Simple window-based counting
// - LeakyBucket: Smooths out request rate

use super::ExecutionError;
use crate::mcp::registry::patterns::{ThrottleSpec, ThrottleStrategy};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// In-memory rate limiter state for single-instance throttling
#[derive(Debug, Default)]
pub struct RateLimiterState {
	/// Sliding window: timestamps of recent requests
	sliding_window_timestamps: Vec<Instant>,
	/// Token bucket: current token count and last refill time
	token_bucket: Option<(f64, Instant)>,
	/// Fixed window: count and window start time
	fixed_window: Option<(u32, Instant)>,
	/// Leaky bucket: current level and last drain time
	leaky_bucket: Option<(f64, Instant)>,
}

/// Global rate limiter registry for in-memory throttling
#[derive(Debug, Default)]
pub struct RateLimiterRegistry {
	limiters: HashMap<String, RateLimiterState>,
}

impl RateLimiterRegistry {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn get_or_create(&mut self, key: &str) -> &mut RateLimiterState {
		self.limiters.entry(key.to_string()).or_default()
	}
}

/// Shared rate limiter registry wrapped in Arc<Mutex<>>
pub type SharedRateLimiterRegistry = Arc<Mutex<RateLimiterRegistry>>;

pub struct ThrottleExecutor;

impl ThrottleExecutor {
	/// Check if a request is allowed under the rate limit.
	/// Returns Ok(true) if allowed, Ok(false) if rate limited.
	pub async fn check_rate_limit(
		spec: &ThrottleSpec,
		registry: &SharedRateLimiterRegistry,
		key: &str,
	) -> Result<bool, ExecutionError> {
		let mut registry = registry.lock().await;
		let state = registry.get_or_create(key);
		let now = Instant::now();
		let window = Duration::from_millis(spec.window_ms as u64);
		let rate = spec.rate;

		match spec.strategy {
			ThrottleStrategy::SlidingWindow => Self::check_sliding_window(state, now, window, rate),
			ThrottleStrategy::TokenBucket => Self::check_token_bucket(state, now, window, rate),
			ThrottleStrategy::FixedWindow => Self::check_fixed_window(state, now, window, rate),
			ThrottleStrategy::LeakyBucket => Self::check_leaky_bucket(state, now, window, rate),
		}
	}

	fn check_sliding_window(
		state: &mut RateLimiterState,
		now: Instant,
		window: Duration,
		rate: u32,
	) -> Result<bool, ExecutionError> {
		// Remove timestamps outside the window
		let cutoff = now - window;
		state.sliding_window_timestamps.retain(|&t| t > cutoff);

		if state.sliding_window_timestamps.len() < rate as usize {
			state.sliding_window_timestamps.push(now);
			Ok(true)
		} else {
			Ok(false)
		}
	}

	fn check_token_bucket(
		state: &mut RateLimiterState,
		now: Instant,
		window: Duration,
		rate: u32,
	) -> Result<bool, ExecutionError> {
		let (tokens, last_refill) = state.token_bucket.get_or_insert((rate as f64, now));

		// Calculate token refill
		let elapsed = now.duration_since(*last_refill);
		let refill_rate = rate as f64 / window.as_secs_f64();
		let new_tokens = (*tokens + elapsed.as_secs_f64() * refill_rate).min(rate as f64);

		if new_tokens >= 1.0 {
			state.token_bucket = Some((new_tokens - 1.0, now));
			Ok(true)
		} else {
			state.token_bucket = Some((new_tokens, now));
			Ok(false)
		}
	}

	fn check_fixed_window(
		state: &mut RateLimiterState,
		now: Instant,
		window: Duration,
		rate: u32,
	) -> Result<bool, ExecutionError> {
		let (count, window_start) = state.fixed_window.get_or_insert((0, now));

		// Check if we're in a new window
		if now.duration_since(*window_start) >= window {
			state.fixed_window = Some((1, now));
			return Ok(true);
		}

		if *count < rate {
			state.fixed_window = Some((*count + 1, *window_start));
			Ok(true)
		} else {
			Ok(false)
		}
	}

	fn check_leaky_bucket(
		state: &mut RateLimiterState,
		now: Instant,
		window: Duration,
		rate: u32,
	) -> Result<bool, ExecutionError> {
		let bucket_capacity = rate as f64;
		let leak_rate = rate as f64 / window.as_secs_f64();

		let (level, last_drain) = state.leaky_bucket.get_or_insert((0.0, now));

		// Drain the bucket based on elapsed time
		let elapsed = now.duration_since(*last_drain);
		let drained = elapsed.as_secs_f64() * leak_rate;
		let new_level = (*level - drained).max(0.0);

		if new_level + 1.0 <= bucket_capacity {
			state.leaky_bucket = Some((new_level + 1.0, now));
			Ok(true)
		} else {
			state.leaky_bucket = Some((new_level, now));
			Ok(false)
		}
	}

	/// Execute the throttle pattern
	pub async fn execute(
		_spec: &ThrottleSpec,
		_input: Value,
		_registry: &SharedRateLimiterRegistry,
	) -> Result<Value, ExecutionError> {
		// TODO: Implement full execution with inner operation
		// For now, this is a placeholder that will be implemented when
		// we integrate with the CompositionExecutor
		Err(ExecutionError::StatefulPatternNotImplemented {
			pattern: "throttle".to_string(),
			details: "ThrottleExecutor::execute is not yet fully integrated with CompositionExecutor"
				.to_string(),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mcp::registry::patterns::{OnExceeded, StepOperation, ToolCall};
	use tokio::time::sleep;

	fn create_test_spec(
		rate: u32,
		window_ms: u32,
		strategy: ThrottleStrategy,
		on_exceeded: OnExceeded,
	) -> ThrottleSpec {
		ThrottleSpec {
			inner: Box::new(StepOperation::Tool(ToolCall::new("test_tool"))),
			rate,
			window_ms,
			strategy,
			on_exceeded,
			store: None,
		}
	}

	fn create_registry() -> SharedRateLimiterRegistry {
		Arc::new(Mutex::new(RateLimiterRegistry::new()))
	}

	// =========================================================================
	// TDD Tests - These tests define the expected behavior
	// =========================================================================

	#[tokio::test]
	async fn test_throttle_under_limit() {
		// Requests under limit should pass through
		let spec = create_test_spec(
			10,
			1000,
			ThrottleStrategy::SlidingWindow,
			OnExceeded::Reject,
		);
		let registry = create_registry();

		// Make 5 requests (under the limit of 10)
		for i in 0..5 {
			let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
				.await
				.unwrap();
			assert!(allowed, "Request {} should be allowed (under limit)", i + 1);
		}
	}

	#[tokio::test]
	async fn test_throttle_at_limit() {
		// Requests at exactly the limit should still pass
		let spec = create_test_spec(5, 1000, ThrottleStrategy::SlidingWindow, OnExceeded::Reject);
		let registry = create_registry();

		// Make exactly 5 requests (at the limit)
		for i in 0..5 {
			let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
				.await
				.unwrap();
			assert!(allowed, "Request {} should be allowed (at limit)", i + 1);
		}
	}

	#[tokio::test]
	async fn test_throttle_exceeded_reject() {
		// Requests exceeding limit should be rejected
		let spec = create_test_spec(3, 1000, ThrottleStrategy::SlidingWindow, OnExceeded::Reject);
		let registry = create_registry();

		// Make 3 requests (at limit)
		for _ in 0..3 {
			let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
				.await
				.unwrap();
			assert!(allowed);
		}

		// 4th request should be rejected
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
			.await
			.unwrap();
		assert!(!allowed, "Request exceeding limit should be rejected");
	}

	#[tokio::test]
	async fn test_throttle_sliding_window() {
		// Window should slide correctly, allowing requests after window expires
		let spec = create_test_spec(2, 100, ThrottleStrategy::SlidingWindow, OnExceeded::Reject);
		let registry = create_registry();

		// Make 2 requests (at limit)
		for _ in 0..2 {
			let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
				.await
				.unwrap();
			assert!(allowed);
		}

		// 3rd request should be rejected
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
			.await
			.unwrap();
		assert!(!allowed);

		// Wait for window to slide (100ms)
		sleep(Duration::from_millis(120)).await;

		// Now requests should be allowed again
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
			.await
			.unwrap();
		assert!(allowed, "Request should be allowed after window slides");
	}

	#[tokio::test]
	async fn test_throttle_token_bucket() {
		// Token bucket should refill over time
		let spec = create_test_spec(10, 1000, ThrottleStrategy::TokenBucket, OnExceeded::Reject);
		let registry = create_registry();

		// Use all tokens
		for _ in 0..10 {
			let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
				.await
				.unwrap();
			assert!(allowed);
		}

		// Should be rate limited
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
			.await
			.unwrap();
		assert!(!allowed);

		// Wait for some tokens to refill (100ms = 1 token at 10/sec)
		sleep(Duration::from_millis(150)).await;

		// Should have at least 1 token now
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
			.await
			.unwrap();
		assert!(allowed, "Token bucket should refill over time");
	}

	#[tokio::test]
	async fn test_throttle_fixed_window() {
		// Fixed window resets at window boundary
		let spec = create_test_spec(3, 100, ThrottleStrategy::FixedWindow, OnExceeded::Reject);
		let registry = create_registry();

		// Use all requests in current window
		for _ in 0..3 {
			let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
				.await
				.unwrap();
			assert!(allowed);
		}

		// Should be rate limited
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
			.await
			.unwrap();
		assert!(!allowed);

		// Wait for new window
		sleep(Duration::from_millis(120)).await;

		// New window should allow requests again
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
			.await
			.unwrap();
		assert!(allowed, "Fixed window should reset");
	}

	#[tokio::test]
	async fn test_throttle_leaky_bucket() {
		// Leaky bucket drains over time
		let spec = create_test_spec(5, 500, ThrottleStrategy::LeakyBucket, OnExceeded::Reject);
		let registry = create_registry();

		// Fill the bucket
		for _ in 0..5 {
			let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
				.await
				.unwrap();
			assert!(allowed);
		}

		// Should be rate limited (bucket full)
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
			.await
			.unwrap();
		assert!(!allowed);

		// Wait for bucket to drain some (100ms = 1 request drained at 10/sec)
		sleep(Duration::from_millis(150)).await;

		// Should be allowed as bucket has drained
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "test_key")
			.await
			.unwrap();
		assert!(allowed, "Leaky bucket should drain over time");
	}

	#[tokio::test]
	async fn test_throttle_separate_keys() {
		// Different keys should have separate rate limits
		let spec = create_test_spec(2, 1000, ThrottleStrategy::SlidingWindow, OnExceeded::Reject);
		let registry = create_registry();

		// Use up limit for key1
		for _ in 0..2 {
			let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "key1")
				.await
				.unwrap();
			assert!(allowed);
		}
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "key1")
			.await
			.unwrap();
		assert!(!allowed, "key1 should be rate limited");

		// key2 should still be allowed
		let allowed = ThrottleExecutor::check_rate_limit(&spec, &registry, "key2")
			.await
			.unwrap();
		assert!(allowed, "key2 should have separate limit");
	}
}
