//! Idempotent request pattern implementation.
//!
//! This module provides idempotency handling for HTTP requests, preventing duplicate
//! processing by caching request signatures and their results.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::de::Error;

use crate::cel::{self, Executor, Expression};
use crate::http::PolicyResponse;
use crate::proxy::ProxyError;
use crate::*;

/// What to do when a duplicate request is detected.
#[apply(schema!)]
#[derive(Default, Eq, PartialEq, Copy)]
pub enum OnDuplicate {
	/// Return the cached result from the first request.
	#[serde(rename = "cached")]
	#[default]
	Cached,
	/// Skip processing and return null/empty response.
	#[serde(rename = "skip")]
	Skip,
	/// Return an error indicating duplicate request.
	#[serde(rename = "error")]
	Error,
}

/// Configuration specification for idempotent request handling.
#[apply(schema!)]
pub struct IdempotentSpec {
	/// CEL expressions to derive the idempotency key from the request.
	/// Multiple paths are concatenated to form the final key.
	#[serde(deserialize_with = "de_key_paths")]
	#[cfg_attr(feature = "schema", schemars(with = "Vec<String>"))]
	pub key_paths: Arc<Vec<Expression>>,

	/// What to do when a duplicate request is detected.
	#[serde(default)]
	pub on_duplicate: OnDuplicate,

	/// Time-to-live for idempotency keys.
	#[serde(with = "serde_dur")]
	#[cfg_attr(feature = "schema", schemars(with = "String"))]
	pub ttl: Duration,
}

fn de_key_paths<'de, D>(deserializer: D) -> Result<Arc<Vec<Expression>>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let raw = Vec::<String>::deserialize(deserializer)?;
	let parsed: Vec<_> = raw
		.into_iter()
		.map(cel::Expression::new_strict)
		.collect::<Result<_, _>>()
		.map_err(|e| serde::de::Error::custom(e.to_string()))?;
	Ok(Arc::new(parsed))
}

/// Cached entry with expiration tracking.
#[derive(Debug, Clone)]
struct CacheEntry {
	/// Cached response body (if available).
	response: Option<bytes::Bytes>,
	/// When this entry expires.
	expires_at: Instant,
}

/// In-memory idempotency store with TTL support.
#[derive(Debug, Default)]
struct IdempotencyStore {
	entries: RwLock<HashMap<String, CacheEntry>>,
}

impl IdempotencyStore {
	/// Attempt to set a key if it doesn't exist.
	/// Returns true if the key was set (first request), false if it already exists (duplicate).
	fn set_if_not_exists(&self, key: &str, ttl: Duration) -> bool {
		let now = Instant::now();
		let Ok(mut entries) = self.entries.write() else {
			// Lock poisoned - treat as first request to avoid blocking
			tracing::error!("idempotency store lock poisoned in set_if_not_exists");
			return true;
		};

		// Check if key exists and is not expired
		if let Some(entry) = entries.get(key)
			&& entry.expires_at > now
		{
			return false; // Duplicate
		}

		// Set the key
		entries.insert(
			key.to_string(),
			CacheEntry {
				response: None,
				expires_at: now + ttl,
			},
		);
		true
	}

	/// Get the cached response for a key if it exists and hasn't expired.
	fn get(&self, key: &str) -> Option<bytes::Bytes> {
		let now = Instant::now();
		let Ok(entries) = self.entries.read() else {
			tracing::error!("idempotency store lock poisoned in get");
			return None;
		};

		entries.get(key).and_then(|entry| {
			if entry.expires_at > now {
				entry.response.clone()
			} else {
				None
			}
		})
	}

	/// Store a response for a key.
	fn set_response(&self, key: &str, response: bytes::Bytes, ttl: Duration) {
		let now = Instant::now();
		let Ok(mut entries) = self.entries.write() else {
			tracing::error!("idempotency store lock poisoned in set_response");
			return;
		};

		entries.insert(
			key.to_string(),
			CacheEntry {
				response: Some(response),
				expires_at: now + ttl,
			},
		);
	}

	/// Clean up expired entries.
	#[allow(dead_code)]
	fn cleanup(&self) {
		let now = Instant::now();
		let Ok(mut entries) = self.entries.write() else {
			tracing::error!("idempotency store lock poisoned in cleanup");
			return;
		};
		entries.retain(|_, entry| entry.expires_at > now);
	}
}

/// Runtime idempotent policy that wraps the spec with a store.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "schema", schemars(with = "IdempotentSpec"))]
#[derive(serde::Serialize)]
pub struct Idempotent {
	#[serde(skip_serializing)]
	store: Arc<IdempotencyStore>,
	#[serde(flatten)]
	pub spec: IdempotentSpec,
}

impl<'de> serde::Deserialize<'de> for Idempotent {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let spec = IdempotentSpec::deserialize(deserializer)?;
		Idempotent::try_from(spec).map_err(D::Error::custom)
	}
}

impl TryFrom<IdempotentSpec> for Idempotent {
	type Error = &'static str;

	fn try_from(spec: IdempotentSpec) -> Result<Self, Self::Error> {
		if spec.key_paths.is_empty() {
			return Err("at least one key path is required");
		}
		Ok(Idempotent {
			store: Arc::new(IdempotencyStore::default()),
			spec,
		})
	}
}

impl Idempotent {
	/// Derive the idempotency key from the request using configured CEL expressions.
	fn derive_key(&self, exec: &Executor<'_>) -> Result<String, ProxyError> {
		let mut key_parts = Vec::with_capacity(self.spec.key_paths.len());

		for expr in self.spec.key_paths.iter() {
			match exec.eval(expr) {
				Ok(value) => {
					let Some(string_value) = cel::value_as_string(&value) else {
						return Err(ProxyError::ProcessingString(format!(
							"idempotent key expression did not evaluate to string: {:?}",
							expr
						)));
					};
					key_parts.push(string_value);
				},
				Err(e) => {
					return Err(ProxyError::ProcessingString(format!(
						"failed to evaluate idempotent key expression: {}",
						e
					)));
				},
			}
		}

		Ok(key_parts.join(":"))
	}

	/// Check if this request is a duplicate and handle accordingly.
	///
	/// Returns `Ok(PolicyResponse)` if the request should proceed or if a cached response
	/// is returned. Returns `Err(ProxyResponse)` if the request should be rejected.
	pub fn check(&self, exec: &Executor<'_>) -> Result<IdempotentCheckResult, ProxyError> {
		let key = self.derive_key(exec)?;

		// Try to acquire the idempotency lock
		if self.store.set_if_not_exists(&key, self.spec.ttl) {
			// First request with this key - proceed
			Ok(IdempotentCheckResult::Proceed { key })
		} else {
			// Duplicate request detected
			match self.spec.on_duplicate {
				OnDuplicate::Cached => {
					if let Some(cached_response) = self.store.get(&key) {
						Ok(IdempotentCheckResult::CachedResponse {
							key,
							response: cached_response,
						})
					} else {
						// Request is being processed, return 409 Conflict
						Err(ProxyError::DuplicateRequest)
					}
				},
				OnDuplicate::Skip => Ok(IdempotentCheckResult::Skip { key }),
				OnDuplicate::Error => Err(ProxyError::DuplicateRequest),
			}
		}
	}

	/// Store the response for a key that was previously checked.
	pub fn store_response(&self, key: &str, response: bytes::Bytes) {
		self.store.set_response(key, response, self.spec.ttl);
	}

	/// Get all CEL expressions used by this policy for validation.
	pub fn expressions(&self) -> impl Iterator<Item = &Expression> {
		self.spec.key_paths.iter()
	}
}

/// Result of checking idempotency for a request.
#[derive(Debug, Clone)]
pub enum IdempotentCheckResult {
	/// This is the first request with this key - proceed with normal processing.
	Proceed { key: String },
	/// A cached response is available for this duplicate request.
	CachedResponse { key: String, response: bytes::Bytes },
	/// This is a duplicate request that should be skipped (return empty response).
	Skip { key: String },
}

impl IdempotentCheckResult {
	/// Get the idempotency key.
	pub fn key(&self) -> &str {
		match self {
			IdempotentCheckResult::Proceed { key } => key,
			IdempotentCheckResult::CachedResponse { key, .. } => key,
			IdempotentCheckResult::Skip { key } => key,
		}
	}

	/// Check if this result indicates the request should proceed normally.
	pub fn should_proceed(&self) -> bool {
		matches!(self, IdempotentCheckResult::Proceed { .. })
	}

	/// Convert to a PolicyResponse if this is a cached or skip result.
	pub fn to_policy_response(&self) -> Option<PolicyResponse> {
		match self {
			IdempotentCheckResult::CachedResponse { response, .. } => {
				let resp = ::http::Response::builder()
					.status(http::StatusCode::OK)
					.header(http::header::CONTENT_TYPE, "application/json")
					.header("x-idempotent-replayed", "true")
					.body(http::Body::from(response.clone()))
					.ok()?;
				Some(PolicyResponse {
					direct_response: Some(resp),
					response_headers: None,
				})
			},
			IdempotentCheckResult::Skip { .. } => {
				let resp = ::http::Response::builder()
					.status(http::StatusCode::NO_CONTENT)
					.header("x-idempotent-skipped", "true")
					.body(http::Body::empty())
					.ok()?;
				Some(PolicyResponse {
					direct_response: Some(resp),
					response_headers: None,
				})
			},
			IdempotentCheckResult::Proceed { .. } => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn make_spec(key_paths: Vec<&str>, on_duplicate: OnDuplicate, ttl_secs: u64) -> IdempotentSpec {
		IdempotentSpec {
			key_paths: Arc::new(
				key_paths
					.into_iter()
					.map(|s| cel::Expression::new_strict(s.to_string()).unwrap())
					.collect(),
			),
			on_duplicate,
			ttl: Duration::from_secs(ttl_secs),
		}
	}

	#[test]
	fn test_idempotent_spec_creation() {
		let spec = make_spec(
			vec!["request.headers['x-idempotency-key']"],
			OnDuplicate::Cached,
			300,
		);
		let idempotent = Idempotent::try_from(spec).unwrap();
		assert_eq!(idempotent.spec.on_duplicate, OnDuplicate::Cached);
	}

	#[test]
	fn test_idempotent_empty_key_paths_fails() {
		let spec = IdempotentSpec {
			key_paths: Arc::new(vec![]),
			on_duplicate: OnDuplicate::Cached,
			ttl: Duration::from_secs(300),
		};
		assert!(Idempotent::try_from(spec).is_err());
	}

	#[test]
	fn test_store_set_if_not_exists_first_request() {
		let store = IdempotencyStore::default();
		assert!(store.set_if_not_exists("key1", Duration::from_secs(60)));
	}

	#[test]
	fn test_store_set_if_not_exists_duplicate() {
		let store = IdempotencyStore::default();
		assert!(store.set_if_not_exists("key1", Duration::from_secs(60)));
		assert!(!store.set_if_not_exists("key1", Duration::from_secs(60)));
	}

	#[test]
	fn test_store_set_and_get_response() {
		let store = IdempotencyStore::default();
		store.set_if_not_exists("key1", Duration::from_secs(60));
		store.set_response(
			"key1",
			bytes::Bytes::from("response data"),
			Duration::from_secs(60),
		);

		let response = store.get("key1");
		assert!(response.is_some());
		assert_eq!(response.unwrap(), bytes::Bytes::from("response data"));
	}

	#[test]
	fn test_store_get_nonexistent_key() {
		let store = IdempotencyStore::default();
		assert!(store.get("nonexistent").is_none());
	}

	#[test]
	fn test_on_duplicate_variants() {
		assert_eq!(OnDuplicate::default(), OnDuplicate::Cached);
		assert_ne!(OnDuplicate::Skip, OnDuplicate::Error);
	}

	#[test]
	fn test_idempotent_check_result_should_proceed() {
		let result = IdempotentCheckResult::Proceed {
			key: "test".to_string(),
		};
		assert!(result.should_proceed());

		let result = IdempotentCheckResult::Skip {
			key: "test".to_string(),
		};
		assert!(!result.should_proceed());

		let result = IdempotentCheckResult::CachedResponse {
			key: "test".to_string(),
			response: bytes::Bytes::from("cached"),
		};
		assert!(!result.should_proceed());
	}

	#[test]
	fn test_idempotent_check_result_key() {
		let result = IdempotentCheckResult::Proceed {
			key: "my-key".to_string(),
		};
		assert_eq!(result.key(), "my-key");
	}

	#[test]
	fn test_idempotent_check_result_to_policy_response_proceed() {
		let result = IdempotentCheckResult::Proceed {
			key: "test".to_string(),
		};
		assert!(result.to_policy_response().is_none());
	}

	#[test]
	fn test_idempotent_check_result_to_policy_response_skip() {
		let result = IdempotentCheckResult::Skip {
			key: "test".to_string(),
		};
		let response = result.to_policy_response();
		assert!(response.is_some());
		let pr = response.unwrap();
		assert!(pr.direct_response.is_some());
		let dr = pr.direct_response.unwrap();
		assert_eq!(dr.status(), http::StatusCode::NO_CONTENT);
	}

	#[test]
	fn test_idempotent_check_result_to_policy_response_cached() {
		let result = IdempotentCheckResult::CachedResponse {
			key: "test".to_string(),
			response: bytes::Bytes::from(r#"{"result": "cached"}"#),
		};
		let response = result.to_policy_response();
		assert!(response.is_some());
		let pr = response.unwrap();
		assert!(pr.direct_response.is_some());
		let dr = pr.direct_response.unwrap();
		assert_eq!(dr.status(), http::StatusCode::OK);
	}
}
