//! Stateful patterns for HTTP request handling.
//!
//! This module implements stateful patterns like CircuitBreaker that require
//! persistent state across requests.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::*;

/// Error type for circuit breaker operations
#[derive(Debug, Clone)]
pub enum CircuitBreakerError {
	/// Circuit is open - request rejected
	CircuitOpen {
		/// Name of the circuit breaker
		name: String,
		/// When the circuit will attempt to close (if available)
		retry_after: Option<Duration>,
	},
	/// State store operation failed
	StateError(String),
	/// Underlying operation failed
	OperationFailed(String),
}

impl std::fmt::Display for CircuitBreakerError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CircuitBreakerError::CircuitOpen { name, retry_after } => {
				write!(f, "circuit '{}' is open", name)?;
				if let Some(dur) = retry_after {
					write!(f, ", retry after {:?}", dur)?;
				}
				Ok(())
			},
			CircuitBreakerError::StateError(msg) => write!(f, "state error: {}", msg),
			CircuitBreakerError::OperationFailed(msg) => write!(f, "operation failed: {}", msg),
		}
	}
}

impl std::error::Error for CircuitBreakerError {}

/// Trait for state storage used by stateful patterns like CircuitBreaker.
///
/// Implementations may use in-memory storage, Redis, or other backends.
#[async_trait]
pub trait StateStore: Send + Sync + 'static {
	/// Load circuit breaker state by name.
	/// Returns None if no state exists (new circuit).
	async fn load_circuit_state(&self, name: &str) -> Result<Option<CircuitState>, String>;

	/// Save circuit breaker state.
	async fn save_circuit_state(&self, name: &str, state: &CircuitState) -> Result<(), String>;
}

/// In-memory state store for testing and single-instance deployments.
#[derive(Default)]
pub struct InMemoryStateStore {
	circuits: std::sync::RwLock<std::collections::HashMap<String, CircuitState>>,
}

impl InMemoryStateStore {
	pub fn new() -> Self {
		Self::default()
	}
}

#[async_trait]
impl StateStore for InMemoryStateStore {
	async fn load_circuit_state(&self, name: &str) -> Result<Option<CircuitState>, String> {
		let guard = self.circuits.read().map_err(|e| e.to_string())?;
		Ok(guard.get(name).cloned())
	}

	async fn save_circuit_state(&self, name: &str, state: &CircuitState) -> Result<(), String> {
		let mut guard = self.circuits.write().map_err(|e| e.to_string())?;
		guard.insert(name.to_string(), state.clone());
		Ok(())
	}
}

/// The state of a circuit breaker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CircuitStateEnum {
	/// Circuit is closed - requests flow normally
	#[default]
	Closed,
	/// Circuit is open - requests are rejected
	Open,
	/// Circuit is half-open - allowing test requests
	HalfOpen,
}

/// Persistent state for a circuit breaker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitState {
	/// Current state of the circuit
	pub state: CircuitStateEnum,
	/// Number of consecutive failures
	pub failure_count: u32,
	/// Timestamp of the last failure (as Unix timestamp in milliseconds)
	pub last_failure_time_ms: Option<u64>,
	/// Number of successful requests in half-open state
	pub success_count_in_half_open: u32,
	/// Timestamp when the circuit transitioned to open state (Unix ms)
	pub opened_at_ms: Option<u64>,
}

impl Default for CircuitState {
	fn default() -> Self {
		Self {
			state: CircuitStateEnum::Closed,
			failure_count: 0,
			last_failure_time_ms: None,
			success_count_in_half_open: 0,
			opened_at_ms: None,
		}
	}
}

impl CircuitState {
	/// Get the time since the last failure, if any
	pub fn time_since_last_failure(&self) -> Option<Duration> {
		let last_failure_ms = self.last_failure_time_ms?;
		let now_ms = SystemTime::now()
			.duration_since(SystemTime::UNIX_EPOCH)
			.ok()?
			.as_millis() as u64;
		if now_ms >= last_failure_ms {
			Some(Duration::from_millis(now_ms - last_failure_ms))
		} else {
			None
		}
	}

	/// Get the time since the circuit opened, if any
	pub fn time_since_opened(&self) -> Option<Duration> {
		let opened_ms = self.opened_at_ms?;
		let now_ms = SystemTime::now()
			.duration_since(SystemTime::UNIX_EPOCH)
			.ok()?
			.as_millis() as u64;
		if now_ms >= opened_ms {
			Some(Duration::from_millis(now_ms - opened_ms))
		} else {
			None
		}
	}

	/// Get current timestamp in milliseconds
	fn now_ms() -> u64 {
		SystemTime::now()
			.duration_since(SystemTime::UNIX_EPOCH)
			.map(|d| d.as_millis() as u64)
			.unwrap_or(0)
	}

	/// Record a failure
	pub fn record_failure(&mut self) {
		self.failure_count += 1;
		self.last_failure_time_ms = Some(Self::now_ms());
	}

	/// Record a success
	pub fn record_success(&mut self) {
		if self.state == CircuitStateEnum::HalfOpen {
			self.success_count_in_half_open += 1;
		}
	}

	/// Transition to open state
	pub fn transition_to_open(&mut self) {
		self.state = CircuitStateEnum::Open;
		self.opened_at_ms = Some(Self::now_ms());
		self.success_count_in_half_open = 0;
	}

	/// Transition to half-open state
	pub fn transition_to_half_open(&mut self) {
		self.state = CircuitStateEnum::HalfOpen;
		self.success_count_in_half_open = 0;
	}

	/// Transition to closed state
	pub fn transition_to_closed(&mut self) {
		self.state = CircuitStateEnum::Closed;
		self.failure_count = 0;
		self.last_failure_time_ms = None;
		self.opened_at_ms = None;
		self.success_count_in_half_open = 0;
	}

	/// Check if failures are within the specified window
	pub fn failures_within_window(&self, window: Duration) -> bool {
		match self.time_since_last_failure() {
			Some(elapsed) => elapsed <= window,
			None => false,
		}
	}
}

/// Configuration for a circuit breaker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(crate::JsonSchema))]
pub struct CircuitBreakerSpec {
	/// Unique name for this circuit breaker
	pub name: String,

	/// Number of failures before the circuit opens
	#[serde(default = "default_failure_threshold")]
	pub failure_threshold: u32,

	/// Duration the circuit stays open before transitioning to half-open
	#[serde(default = "default_reset_timeout", with = "serde_dur")]
	#[cfg_attr(feature = "schema", schemars(with = "String"))]
	pub reset_timeout: Duration,

	/// Time window in which failures are counted
	/// Failures outside this window don't count toward the threshold
	#[serde(default = "default_failure_window", with = "serde_dur")]
	#[cfg_attr(feature = "schema", schemars(with = "String"))]
	pub failure_window: Duration,

	/// Number of successful requests in half-open state before closing the circuit
	#[serde(default = "default_success_threshold")]
	pub success_threshold: u32,

	/// Optional fallback value to return when the circuit is open
	#[serde(default)]
	pub fallback: Option<serde_json::Value>,
}

fn default_failure_threshold() -> u32 {
	5
}

fn default_reset_timeout() -> Duration {
	Duration::from_secs(30)
}

fn default_failure_window() -> Duration {
	Duration::from_secs(60)
}

fn default_success_threshold() -> u32 {
	3
}

impl Default for CircuitBreakerSpec {
	fn default() -> Self {
		Self {
			name: String::new(),
			failure_threshold: default_failure_threshold(),
			reset_timeout: default_reset_timeout(),
			failure_window: default_failure_window(),
			success_threshold: default_success_threshold(),
			fallback: None,
		}
	}
}

/// Circuit breaker executor that manages state and controls request flow.
pub struct CircuitBreakerExecutor<S: StateStore> {
	store: Arc<S>,
}

impl<S: StateStore> CircuitBreakerExecutor<S> {
	/// Create a new circuit breaker executor with the given state store
	pub fn new(store: Arc<S>) -> Self {
		Self { store }
	}

	/// Execute a request through the circuit breaker.
	///
	/// State machine:
	/// ```text
	/// CLOSED --[failures >= threshold]--> OPEN
	/// OPEN --[reset_timeout elapsed]--> HALF_OPEN
	/// HALF_OPEN --[success]--> CLOSED (after success_threshold successes)
	/// HALF_OPEN --[failure]--> OPEN
	/// ```
	pub async fn execute<F, T, E>(
		&self,
		spec: &CircuitBreakerSpec,
		operation: F,
	) -> Result<T, CircuitBreakerError>
	where
		F: std::future::Future<Output = Result<T, E>>,
		E: std::fmt::Display,
	{
		// Load current state
		let mut state = self
			.store
			.load_circuit_state(&spec.name)
			.await
			.map_err(CircuitBreakerError::StateError)?
			.unwrap_or_default();

		match state.state {
			CircuitStateEnum::Closed => {
				self
					.try_with_failure_tracking(spec, &mut state, operation)
					.await
			},
			CircuitStateEnum::Open => {
				if self.should_attempt_reset(&state, spec) {
					// Transition to half-open and try
					state.transition_to_half_open();
					self
						.store
						.save_circuit_state(&spec.name, &state)
						.await
						.map_err(CircuitBreakerError::StateError)?;
					self.try_for_recovery(spec, &mut state, operation).await
				} else {
					// Fast fail
					self.execute_fallback_or_error(spec, &state)
				}
			},
			CircuitStateEnum::HalfOpen => self.try_for_recovery(spec, &mut state, operation).await,
		}
	}

	/// Check if enough time has passed to attempt reset
	fn should_attempt_reset(&self, state: &CircuitState, spec: &CircuitBreakerSpec) -> bool {
		match state.time_since_opened() {
			Some(elapsed) => elapsed >= spec.reset_timeout,
			None => true, // No open time recorded, allow attempt
		}
	}

	/// Execute operation with failure tracking (used in closed state)
	async fn try_with_failure_tracking<F, T, E>(
		&self,
		spec: &CircuitBreakerSpec,
		state: &mut CircuitState,
		operation: F,
	) -> Result<T, CircuitBreakerError>
	where
		F: std::future::Future<Output = Result<T, E>>,
		E: std::fmt::Display,
	{
		match operation.await {
			Ok(result) => {
				// Success in closed state - reset failure count if window expired
				if !state.failures_within_window(spec.failure_window) {
					state.failure_count = 0;
				}
				// No need to save state on every success in closed state
				Ok(result)
			},
			Err(e) => {
				// Record failure
				if state.failures_within_window(spec.failure_window) {
					state.record_failure();
				} else {
					// Window expired, start fresh
					state.failure_count = 1;
					state.last_failure_time_ms = Some(CircuitState::now_ms());
				}

				// Check if we should trip the circuit
				if state.failure_count >= spec.failure_threshold {
					state.transition_to_open();
				}

				// Save updated state
				self
					.store
					.save_circuit_state(&spec.name, state)
					.await
					.map_err(CircuitBreakerError::StateError)?;

				Err(CircuitBreakerError::OperationFailed(e.to_string()))
			},
		}
	}

	/// Execute operation for recovery (used in half-open state)
	async fn try_for_recovery<F, T, E>(
		&self,
		spec: &CircuitBreakerSpec,
		state: &mut CircuitState,
		operation: F,
	) -> Result<T, CircuitBreakerError>
	where
		F: std::future::Future<Output = Result<T, E>>,
		E: std::fmt::Display,
	{
		match operation.await {
			Ok(result) => {
				state.record_success();
				// Check if we've had enough successes to close the circuit
				if state.success_count_in_half_open >= spec.success_threshold {
					state.transition_to_closed();
				}
				self
					.store
					.save_circuit_state(&spec.name, state)
					.await
					.map_err(CircuitBreakerError::StateError)?;
				Ok(result)
			},
			Err(e) => {
				// Failure in half-open state - back to open
				state.transition_to_open();
				self
					.store
					.save_circuit_state(&spec.name, state)
					.await
					.map_err(CircuitBreakerError::StateError)?;
				Err(CircuitBreakerError::OperationFailed(e.to_string()))
			},
		}
	}

	/// Return fallback value or error when circuit is open
	fn execute_fallback_or_error<T>(
		&self,
		spec: &CircuitBreakerSpec,
		state: &CircuitState,
	) -> Result<T, CircuitBreakerError> {
		// Calculate retry_after hint
		let retry_after = state.time_since_opened().and_then(|elapsed| {
			if elapsed < spec.reset_timeout {
				Some(spec.reset_timeout - elapsed)
			} else {
				None
			}
		});

		Err(CircuitBreakerError::CircuitOpen {
			name: spec.name.clone(),
			retry_after,
		})
	}
}

#[cfg(test)]
mod tests;
