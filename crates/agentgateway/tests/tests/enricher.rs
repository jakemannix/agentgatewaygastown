//! Integration tests for the Enricher pattern.
//!
//! Tests verify that the enricher:
//! - Makes parallel calls to enrichment backends
//! - Merges results according to the configured strategy
//! - Handles timeouts and errors appropriately

use http::{Method, StatusCode};
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::common::gateway::AgentGateway;

/// Helper to read response body as JSON
async fn read_body_json(resp: agentgateway::http::Response) -> Value {
	let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
		.await
		.unwrap();
	serde_json::from_slice(&body).unwrap()
}

/// Test basic enricher with spread merge strategy.
/// The enricher fetches user data from an enrichment backend and spreads it into the request.
#[tokio::test]
async fn test_enricher_spread_merge() -> anyhow::Result<()> {
	// Mock the enrichment backend - returns user profile data
	let enrichment_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"userId": 123,
			"name": "Alice",
			"role": "admin"
		})))
		.mount(&enrichment_mock)
		.await;

	// Mock the final backend - echoes back the request body
	let backend_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(move |req: &wiremock::Request| {
			// Echo back the request body so we can verify enrichment
			ResponseTemplate::new(200).set_body_bytes(req.body.clone())
		})
		.mount(&backend_mock)
		.await;

	let gw = AgentGateway::new(format!(
		r#"config: {{}}
binds:
- port: $PORT
  listeners:
  - name: default
    protocol: HTTP
    routes:
    - name: default
      policies:
        enricher:
          enrichments:
          - field: user_profile
            backend:
              host: "{}"
          merge: spread
      backends:
        - host: {}
"#,
		enrichment_mock.address(),
		backend_mock.address()
	))
	.await?;

	// Send a request with original data
	let resp = gw
		.send_request_json(
			"http://localhost/api/action",
			json!({
				"action": "create_item",
				"data": {"name": "test"}
			}),
		)
		.await;

	assert_eq!(resp.status(), StatusCode::OK);

	// Verify the enriched body includes both original data and enrichment
	let body = read_body_json(resp).await;
	assert_eq!(body["action"], "create_item");
	assert_eq!(body["data"]["name"], "test");
	// Enrichment data should be spread into root
	assert_eq!(body["user_profile"]["userId"], 123);
	assert_eq!(body["user_profile"]["name"], "Alice");
	assert_eq!(body["user_profile"]["role"], "admin");

	Ok(())
}

/// Test enricher with nested merge strategy.
/// All enrichment results are placed under a single key.
#[tokio::test]
async fn test_enricher_nested_merge() -> anyhow::Result<()> {
	// Mock two enrichment backends
	let user_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"id": 1,
			"name": "Bob"
		})))
		.mount(&user_mock)
		.await;

	let perms_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"canRead": true,
			"canWrite": false
		})))
		.mount(&perms_mock)
		.await;

	// Backend echoes request
	let backend_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(move |req: &wiremock::Request| {
			ResponseTemplate::new(200).set_body_bytes(req.body.clone())
		})
		.mount(&backend_mock)
		.await;

	let gw = AgentGateway::new(format!(
		r#"config: {{}}
binds:
- port: $PORT
  listeners:
  - name: default
    protocol: HTTP
    routes:
    - name: default
      policies:
        enricher:
          enrichments:
          - field: user
            backend:
              host: "{}"
          - field: permissions
            backend:
              host: "{}"
          merge:
            nested:
              key: enriched_data
      backends:
        - host: {}
"#,
		user_mock.address(),
		perms_mock.address(),
		backend_mock.address()
	))
	.await?;

	let resp = gw
		.send_request_json(
			"http://localhost/action",
			json!({
				"request_type": "check_access"
			}),
		)
		.await;

	assert_eq!(resp.status(), StatusCode::OK);

	let body = read_body_json(resp).await;
	// Original data preserved
	assert_eq!(body["request_type"], "check_access");
	// Enrichment nested under key
	assert_eq!(body["enriched_data"]["user"]["id"], 1);
	assert_eq!(body["enriched_data"]["user"]["name"], "Bob");
	assert_eq!(body["enriched_data"]["permissions"]["canRead"], true);
	assert_eq!(body["enriched_data"]["permissions"]["canWrite"], false);

	Ok(())
}

/// Test enricher with ignoreFailures=true.
/// When an enrichment backend fails, the request should continue.
#[tokio::test]
async fn test_enricher_ignore_failures() -> anyhow::Result<()> {
	// Working enrichment backend
	let working_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"data": "from_working"
		})))
		.mount(&working_mock)
		.await;

	// Failing enrichment backend
	let failing_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(ResponseTemplate::new(500).set_body_string("Internal error"))
		.mount(&failing_mock)
		.await;

	// Backend echoes request
	let backend_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(move |req: &wiremock::Request| {
			ResponseTemplate::new(200).set_body_bytes(req.body.clone())
		})
		.mount(&backend_mock)
		.await;

	let gw = AgentGateway::new(format!(
		r#"config: {{}}
binds:
- port: $PORT
  listeners:
  - name: default
    protocol: HTTP
    routes:
    - name: default
      policies:
        enricher:
          enrichments:
          - field: working
            backend:
              host: "{}"
          - field: failing
            backend:
              host: "{}"
          merge: spread
          ignoreFailures: true
      backends:
        - host: {}
"#,
		working_mock.address(),
		failing_mock.address(),
		backend_mock.address()
	))
	.await?;

	let resp = gw
		.send_request_json(
			"http://localhost/test",
			json!({
				"original": "data"
			}),
		)
		.await;

	assert_eq!(resp.status(), StatusCode::OK);

	let body = read_body_json(resp).await;
	// Original data preserved
	assert_eq!(body["original"], "data");
	// Working enrichment present
	assert_eq!(body["working"]["data"], "from_working");
	// Failing enrichment not present (was ignored)
	assert!(body.get("failing").is_none());

	Ok(())
}

/// Test enricher fails when ignoreFailures=false (default) and backend fails.
#[tokio::test]
async fn test_enricher_fail_on_error() -> anyhow::Result<()> {
	// Failing enrichment backend
	let failing_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(ResponseTemplate::new(500).set_body_string("Service unavailable"))
		.mount(&failing_mock)
		.await;

	// Backend (should not be reached)
	let backend_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
		.expect(0) // Should NOT be called
		.mount(&backend_mock)
		.await;

	let gw = AgentGateway::new(format!(
		r#"config: {{}}
binds:
- port: $PORT
  listeners:
  - name: default
    protocol: HTTP
    routes:
    - name: default
      policies:
        enricher:
          enrichments:
          - field: data
            backend:
              host: "{}"
          merge: spread
      backends:
        - host: {}
"#,
		failing_mock.address(),
		backend_mock.address()
	))
	.await?;

	let resp = gw
		.send_request_json("http://localhost/test", json!({"test": true}))
		.await;

	// Should fail with 503 since enrichment failed (processing error)
	assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

	Ok(())
}

/// Test enricher with empty request body.
/// Enricher should work even when there's no request body.
#[tokio::test]
async fn test_enricher_empty_body() -> anyhow::Result<()> {
	// Enrichment backend
	let enrichment_mock = MockServer::start().await;
	Mock::given(method("POST"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"injected": "data"
		})))
		.mount(&enrichment_mock)
		.await;

	// Backend echoes request body (or returns what it received)
	let backend_mock = MockServer::start().await;
	Mock::given(wiremock::matchers::any())
		.respond_with(move |req: &wiremock::Request| {
			if req.body.is_empty() {
				ResponseTemplate::new(200).set_body_json(json!({"received": "empty"}))
			} else {
				ResponseTemplate::new(200).set_body_bytes(req.body.clone())
			}
		})
		.mount(&backend_mock)
		.await;

	let gw = AgentGateway::new(format!(
		r#"config: {{}}
binds:
- port: $PORT
  listeners:
  - name: default
    protocol: HTTP
    routes:
    - name: default
      policies:
        enricher:
          enrichments:
          - field: extra
            backend:
              host: "{}"
          merge: spread
      backends:
        - host: {}
"#,
		enrichment_mock.address(),
		backend_mock.address()
	))
	.await?;

	// Send GET request with no body
	let resp = gw.send_request(Method::GET, "http://localhost/test").await;

	assert_eq!(resp.status(), StatusCode::OK);

	let body = read_body_json(resp).await;
	// Enrichment should still be applied (creating an object with enriched data)
	assert_eq!(body["extra"]["injected"], "data");

	Ok(())
}
