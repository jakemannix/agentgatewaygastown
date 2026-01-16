use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::store::{StateStore, StoreError};

/// Error type for cache execution
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
	#[error("store error: {0}")]
	Store(#[from] StoreError),
	#[error("key derivation error: {0}")]
	KeyDerivation(String),
	#[error("inner execution error: {0}")]
	InnerExecution(String),
	#[error("serialization error: {0}")]
	Serialization(String),
}

/// Specification for the cache pattern.
///
/// The cache pattern wraps an inner operation and caches its results
/// based on key paths derived from the input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheSpec {
	/// JSON paths to extract from the input to form the cache key.
	/// Multiple paths will be concatenated with ":" separator.
	pub key_paths: Vec<String>,

	/// Time-to-live in seconds for cached values.
	pub ttl_seconds: u32,

	/// Optional predicate to determine if the result should be cached.
	/// If not set, all successful results are cached.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cache_if: Option<CachePredicate>,

	/// Optional stale-while-revalidate duration in seconds.
	/// If set, stale values can be returned while revalidating in the background.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stale_while_revalidate_seconds: Option<u32>,
}

/// Predicate for conditional caching
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachePredicate {
	/// The field to check in the result
	pub field: String,
	/// The expected value for caching to occur
	pub equals: Value,
}

/// Cache entry metadata stored alongside the value
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
	/// The cached value
	value: Value,
	/// When the entry was created (Unix timestamp in millis)
	created_at: u64,
	/// Original TTL in seconds (for SWR calculations)
	ttl_seconds: u32,
}

impl CacheSpec {
	/// Create a new CacheSpec with the given key paths and TTL.
	pub fn new(key_paths: Vec<String>, ttl_seconds: u32) -> Self {
		Self {
			key_paths,
			ttl_seconds,
			cache_if: None,
			stale_while_revalidate_seconds: None,
		}
	}

	/// Set a conditional cache predicate.
	pub fn with_cache_if(mut self, field: String, equals: Value) -> Self {
		self.cache_if = Some(CachePredicate { field, equals });
		self
	}

	/// Set stale-while-revalidate duration.
	pub fn with_stale_while_revalidate(mut self, seconds: u32) -> Self {
		self.stale_while_revalidate_seconds = Some(seconds);
		self
	}
}

/// Derive a cache key from the input using the specified key paths.
pub fn derive_cache_key(key_paths: &[String], input: &Value) -> Result<String, CacheError> {
	let mut parts = Vec::with_capacity(key_paths.len());

	for path in key_paths {
		let value = get_json_path(input, path)
			.ok_or_else(|| CacheError::KeyDerivation(format!("path '{}' not found in input", path)))?;

		let part = match value {
			Value::String(s) => s.clone(),
			Value::Number(n) => n.to_string(),
			Value::Bool(b) => b.to_string(),
			Value::Null => "null".to_string(),
			// For arrays/objects, use JSON serialization
			_ => serde_json::to_string(value).map_err(|e| CacheError::Serialization(e.to_string()))?,
		};
		parts.push(part);
	}

	Ok(parts.join(":"))
}

/// Get a value from a JSON object using a dot-separated path.
fn get_json_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
	let mut current = value;

	for segment in path.split('.') {
		match current {
			Value::Object(map) => {
				current = map.get(segment)?;
			},
			Value::Array(arr) => {
				let index: usize = segment.parse().ok()?;
				current = arr.get(index)?;
			},
			_ => return None,
		}
	}

	Some(current)
}

/// Evaluate a cache predicate against a result value.
pub fn evaluate_predicate(predicate: &CachePredicate, result: &Value) -> bool {
	match get_json_path(result, &predicate.field) {
		Some(value) => value == &predicate.equals,
		None => false,
	}
}

/// The cache executor handles cache lookup, miss handling, and storage.
pub struct CacheExecutor;

impl CacheExecutor {
	/// Execute a cache-wrapped operation.
	///
	/// This method:
	/// 1. Derives a cache key from the input
	/// 2. Checks if a cached value exists
	/// 3. If cache hit: returns the cached value
	/// 4. If cache miss: executes the inner operation and caches the result
	///
	/// # Arguments
	/// * `spec` - The cache specification
	/// * `input` - The input value to the operation
	/// * `store` - The state store for caching
	/// * `execute_inner` - A future that executes the inner operation
	///
	/// # Type Parameters
	/// * `F` - The future type for the inner execution
	/// * `E` - The error type for the inner execution (must convert to CacheError)
	pub async fn execute<F, E>(
		spec: &CacheSpec,
		input: Value,
		store: &dyn StateStore,
		execute_inner: F,
	) -> Result<Value, CacheError>
	where
		F: std::future::Future<Output = Result<Value, E>>,
		E: std::fmt::Display,
	{
		let key = derive_cache_key(&spec.key_paths, &input)?;

		// Check cache
		if let Some(cached_bytes) = store.get(&key).await? {
			let entry: CacheEntry = serde_json::from_slice(&cached_bytes)
				.map_err(|e| CacheError::Serialization(e.to_string()))?;

			let now = std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_millis() as u64;

			let age_seconds = (now - entry.created_at) / 1000;
			let is_stale = age_seconds > entry.ttl_seconds as u64;

			// If within TTL, return cached value
			if !is_stale {
				return Ok(entry.value);
			}

			// Check stale-while-revalidate window
			if let Some(swr_seconds) = spec.stale_while_revalidate_seconds {
				let swr_window = entry.ttl_seconds as u64 + swr_seconds as u64;
				if age_seconds <= swr_window {
					// Return stale value, but we could trigger background revalidation here
					// For simplicity, we just return the stale value
					return Ok(entry.value);
				}
			}
		}

		// Cache miss - execute inner operation
		let result = execute_inner
			.await
			.map_err(|e| CacheError::InnerExecution(e.to_string()))?;

		// Check if we should cache the result
		let should_cache = spec
			.cache_if
			.as_ref()
			.map_or(true, |p| evaluate_predicate(p, &result));

		if should_cache {
			let entry = CacheEntry {
				value: result.clone(),
				created_at: std::time::SystemTime::now()
					.duration_since(std::time::UNIX_EPOCH)
					.unwrap()
					.as_millis() as u64,
				ttl_seconds: spec.ttl_seconds,
			};

			let bytes =
				serde_json::to_vec(&entry).map_err(|e| CacheError::Serialization(e.to_string()))?;

			// Use a longer storage TTL to support SWR
			let storage_ttl = spec
				.stale_while_revalidate_seconds
				.map(|swr| spec.ttl_seconds + swr)
				.unwrap_or(spec.ttl_seconds);

			store
				.set(&key, bytes, Some(Duration::from_secs(storage_ttl as u64)))
				.await?;
		}

		Ok(result)
	}
}

#[cfg(test)]
mod tests;
