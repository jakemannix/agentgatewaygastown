//! Enricher pattern implementation for augmenting requests with parallel enrichment calls.
//!
//! The Enricher pattern allows incoming requests to be augmented with data from multiple
//! backend services in parallel. Results are merged back into the original request before
//! forwarding to the destination.

use std::sync::Arc;

use crate::types::agent::SimpleBackendReference;
use crate::*;

#[cfg(test)]
#[path = "enricher_test.rs"]
mod tests;

/// Configuration for the Enricher pattern.
///
/// The Enricher calls multiple backend services in parallel to gather additional data,
/// then merges the results into the original request.
#[apply(schema!)]
pub struct EnricherSpec {
	/// List of enrichment sources to call in parallel
	pub enrichments: Vec<EnrichmentSource>,
	/// Strategy for merging enrichment results into the original data
	pub merge: MergeStrategy,
	/// Whether to ignore failures from individual enrichment sources
	#[serde(default)]
	pub ignore_failures: bool,
	/// Timeout in milliseconds for enrichment calls
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub timeout_ms: Option<u32>,
}

/// A single source of enrichment data.
///
/// Each enrichment source specifies a field name for the result, a backend to call,
/// and optionally an input expression to transform the request data before calling.
#[apply(schema!)]
pub struct EnrichmentSource {
	/// The field name under which the enrichment result will be stored
	pub field: Strng,
	/// The backend service to call for enrichment
	pub backend: SimpleBackendReference,
	/// Optional CEL expression to extract/transform input data for the enrichment call
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub input: Option<Arc<crate::cel::Expression>>,
}

/// Strategy for merging enrichment results into the original request data.
#[apply(schema!)]
pub enum MergeStrategy {
	/// Spread all enrichment fields directly into the root object
	Spread,
	/// Put all enrichment results under a nested key
	Nested {
		/// The key under which to nest all enrichment results
		key: Strng,
	},
	/// Map enrichment results according to a schema
	SchemaMap(SchemaMapSpec),
}

/// Configuration for schema-based mapping of enrichment results.
#[apply(schema!)]
pub struct SchemaMapSpec {
	/// Mapping of target field names to source paths
	#[serde_as(as = "serde_with::Map<_, _>")]
	pub mappings: Vec<(Strng, Strng)>,
}

#[derive(Debug, thiserror::Error)]
pub enum EnricherError {
	#[error("enrichment source '{0}' failed: {1}")]
	SourceFailed(String, String),
	#[error("timeout waiting for enrichment")]
	Timeout,
	#[error("merge failed: {0}")]
	MergeFailed(String),
	#[error("invalid backend reference: {0}")]
	InvalidBackend(String),
	#[error("failed to read request body: {0}")]
	BodyReadFailed(String),
	#[error("failed to parse body as JSON: {0}")]
	JsonParseFailed(String),
}

impl EnricherSpec {
	/// Returns an iterator over all CEL expressions used in this enricher.
	pub fn expressions(&self) -> impl Iterator<Item = &crate::cel::Expression> {
		self.enrichments.iter().filter_map(|e| e.input.as_deref())
	}

	/// Execute the enricher, making parallel calls to all enrichment backends
	/// and merging the results into the request body.
	pub async fn execute(
		&self,
		client: crate::proxy::httpproxy::PolicyClient,
		req: &mut crate::http::Request,
		exec: &crate::cel::Executor<'_>,
	) -> Result<(), EnricherError> {
		use futures::future::join_all;
		use serde_json::{Map, Value};

		// Read the original request body
		let body_bytes = crate::http::inspect_body(req)
			.await
			.map_err(|e| EnricherError::BodyReadFailed(e.to_string()))?;

		// Parse original body as JSON (or use empty object if empty/non-JSON)
		let original: Value = if body_bytes.is_empty() {
			Value::Object(Map::new())
		} else {
			serde_json::from_slice(&body_bytes)
				.map_err(|e| EnricherError::JsonParseFailed(e.to_string()))?
		};

		// Build futures for all enrichment calls
		let enrichment_futures: Vec<_> = self
			.enrichments
			.iter()
			.map(|source| {
				let client = client.clone();
				let field = source.field.clone();
				let backend = source.backend.clone();
				let input_expr = source.input.clone();
				let body_bytes = body_bytes.clone();

				async move {
					// Build the enrichment request body
					let enrichment_body = if let Some(expr) = input_expr {
						// Evaluate CEL expression to get input data
						match exec.eval(&expr) {
							Ok(value) => {
								// Convert CEL Value to string
								if let Some(s) = crate::cel::value_as_string(&value) {
									s.into_bytes()
								} else {
									body_bytes.to_vec()
								}
							},
							Err(_) => body_bytes.to_vec(),
						}
					} else {
						// Send the original body to the enrichment backend
						body_bytes.to_vec()
					};

					// Build HTTP request to enrichment backend
					let enrichment_req = ::http::Request::builder()
						.method(::http::Method::POST)
						.header(::http::header::CONTENT_TYPE, "application/json")
						.body(crate::http::Body::from(enrichment_body))
						.map_err(|e| EnricherError::SourceFailed(field.to_string(), e.to_string()))?;

					// Call the backend
					let response = client.call_reference(enrichment_req, &backend).await.map_err(
						|e| EnricherError::SourceFailed(field.to_string(), e.to_string()),
					)?;

					// Read response body
					let (parts, body) = response.into_parts();
					if !parts.status.is_success() {
						return Err(EnricherError::SourceFailed(
							field.to_string(),
							format!("backend returned status {}", parts.status),
						));
					}

					let response_bytes = crate::http::read_body_with_limit(body, 1024 * 1024)
						.await
						.map_err(|e| EnricherError::SourceFailed(field.to_string(), e.to_string()))?;

					// Parse response as JSON
					let response_value: Value = serde_json::from_slice(&response_bytes)
						.map_err(|e| EnricherError::SourceFailed(field.to_string(), e.to_string()))?;

					Ok::<_, EnricherError>((field, response_value))
				}
			})
			.collect();

		// Execute all enrichment calls in parallel with optional timeout
		let results = if let Some(timeout_ms) = self.timeout_ms {
			let timeout = Duration::from_millis(timeout_ms as u64);
			tokio::time::timeout(timeout, join_all(enrichment_futures))
				.await
				.map_err(|_| EnricherError::Timeout)?
		} else {
			join_all(enrichment_futures).await
		};

		// Collect enrichment results, handling failures
		let mut enrichments: Map<String, Value> = Map::new();
		for result in results {
			match result {
				Ok((field, value)) => {
					enrichments.insert(field.to_string(), value);
				},
				Err(e) => {
					if !self.ignore_failures {
						return Err(e);
					}
					// Log and continue if ignoring failures
					tracing::warn!("Enrichment failed (ignored): {}", e);
				},
			}
		}

		// Merge enrichments into original body according to strategy
		let merged = self.merge_results(original, enrichments)?;

		// Replace request body with merged data
		let merged_bytes =
			serde_json::to_vec(&merged).map_err(|e| EnricherError::MergeFailed(e.to_string()))?;
		*req.body_mut() = crate::http::Body::from(merged_bytes);

		// Update content-length header
		req
			.headers_mut()
			.remove(::http::header::CONTENT_LENGTH);

		Ok(())
	}

	/// Merge enrichment results into the original body according to the merge strategy.
	fn merge_results(
		&self,
		mut original: serde_json::Value,
		enrichments: serde_json::Map<String, serde_json::Value>,
	) -> Result<serde_json::Value, EnricherError> {
		use serde_json::{Map, Value};

		match &self.merge {
			MergeStrategy::Spread => {
				// Spread all enrichment fields directly into the root object
				if let Value::Object(ref mut obj) = original {
					for (key, value) in enrichments {
						obj.insert(key, value);
					}
					Ok(original)
				} else {
					// If original is not an object, wrap it
					let mut result = Map::new();
					result.insert("_original".to_string(), original);
					for (key, value) in enrichments {
						result.insert(key, value);
					}
					Ok(Value::Object(result))
				}
			},
			MergeStrategy::Nested { key } => {
				// Put all enrichment results under a nested key
				if let Value::Object(ref mut obj) = original {
					obj.insert(key.to_string(), Value::Object(enrichments));
					Ok(original)
				} else {
					let mut result = Map::new();
					result.insert("_original".to_string(), original);
					result.insert(key.to_string(), Value::Object(enrichments));
					Ok(Value::Object(result))
				}
			},
			MergeStrategy::SchemaMap(schema) => {
				// Map enrichment results according to schema mappings
				if let Value::Object(ref mut obj) = original {
					for (target_field, source_path) in &schema.mappings {
						if let Some(value) = get_nested_value(&enrichments, source_path.as_str()) {
							obj.insert(target_field.to_string(), value.clone());
						}
					}
					Ok(original)
				} else {
					Err(EnricherError::MergeFailed(
						"SchemaMap merge requires original body to be a JSON object".to_string(),
					))
				}
			},
		}
	}
}

/// Helper to get a nested value from a JSON map using dot notation (e.g., "user.name")
fn get_nested_value<'a>(
	enrichments: &'a serde_json::Map<String, serde_json::Value>,
	path: &str,
) -> Option<&'a serde_json::Value> {
	let parts: Vec<&str> = path.split('.').collect();
	if parts.is_empty() {
		return None;
	}

	// First part is the enrichment field name
	let mut current: &serde_json::Value = enrichments.get(parts[0])?;

	// Navigate through remaining path parts
	for part in &parts[1..] {
		current = current.get(*part)?;
	}

	Some(current)
}
