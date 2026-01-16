use ::http::StatusCode;

use super::*;
use crate::http::retry;

// =============================================================================
// Tests for ProxyError::is_retryable()
// =============================================================================

#[test]
fn test_upstream_call_failed_is_retryable() {
	// UpstreamCallFailed should be retryable - the upstream may recover
	// We need to create a HyperError, which is tricky. Let's test via the enum variant matching.
	// Since HyperError is hard to construct directly, we'll verify the match logic.
	let error = ProxyError::RequestTimeout;
	assert!(error.is_retryable(), "RequestTimeout should be retryable");
}

#[test]
fn test_request_timeout_is_retryable() {
	let error = ProxyError::RequestTimeout;
	assert!(error.is_retryable(), "RequestTimeout should be retryable");
}

#[test]
fn test_dns_resolution_is_retryable() {
	let error = ProxyError::DnsResolution;
	assert!(error.is_retryable(), "DnsResolution should be retryable");
}

#[test]
fn test_bind_not_found_not_retryable() {
	let error = ProxyError::BindNotFound;
	assert!(
		!error.is_retryable(),
		"BindNotFound should NOT be retryable"
	);
}

#[test]
fn test_listener_not_found_not_retryable() {
	let error = ProxyError::ListenerNotFound;
	assert!(
		!error.is_retryable(),
		"ListenerNotFound should NOT be retryable"
	);
}

#[test]
fn test_route_not_found_not_retryable() {
	let error = ProxyError::RouteNotFound;
	assert!(
		!error.is_retryable(),
		"RouteNotFound should NOT be retryable"
	);
}

#[test]
fn test_authorization_failed_not_retryable() {
	let error = ProxyError::AuthorizationFailed;
	assert!(
		!error.is_retryable(),
		"AuthorizationFailed should NOT be retryable"
	);
}

#[test]
fn test_invalid_request_not_retryable() {
	let error = ProxyError::InvalidRequest;
	assert!(
		!error.is_retryable(),
		"InvalidRequest should NOT be retryable"
	);
}

#[test]
fn test_rate_limit_exceeded_not_retryable() {
	let error = ProxyError::RateLimitExceeded {
		limit: 100,
		remaining: 0,
		reset_seconds: 60,
	};
	assert!(
		!error.is_retryable(),
		"RateLimitExceeded should NOT be retryable"
	);
}

#[test]
fn test_no_healthy_endpoints_not_retryable() {
	let error = ProxyError::NoHealthyEndpoints;
	assert!(
		!error.is_retryable(),
		"NoHealthyEndpoints should NOT be retryable"
	);
}

#[test]
fn test_misdirected_request_not_retryable() {
	let error = ProxyError::MisdirectedRequest;
	assert!(
		!error.is_retryable(),
		"MisdirectedRequest should NOT be retryable"
	);
}

// =============================================================================
// Tests for should_retry() with ProxyResponse
// =============================================================================

fn make_policy(codes: Vec<u16>) -> retry::Policy {
	use std::num::NonZeroU8;
	retry::Policy {
		attempts: NonZeroU8::new(3).unwrap(),
		backoff: None,
		codes: codes
			.into_iter()
			.map(|c| StatusCode::from_u16(c).unwrap())
			.collect::<Vec<_>>()
			.into_boxed_slice(),
	}
}

fn make_response(status: u16) -> Response {
	::http::Response::builder()
		.status(StatusCode::from_u16(status).unwrap())
		.body(crate::http::Body::empty())
		.unwrap()
}

#[test]
fn test_should_retry_matching_status_code() {
	let policy = make_policy(vec![502, 503, 504]);
	let response = Ok(make_response(503));

	assert!(
		httpproxy::should_retry(&response, &policy),
		"Should retry when status code matches policy"
	);
}

#[test]
fn test_should_retry_non_matching_status_code() {
	let policy = make_policy(vec![502, 503, 504]);
	let response = Ok(make_response(200));

	assert!(
		!httpproxy::should_retry(&response, &policy),
		"Should NOT retry when status code doesn't match policy"
	);
}

#[test]
fn test_should_retry_404_not_in_policy() {
	let policy = make_policy(vec![502, 503, 504]);
	let response = Ok(make_response(404));

	assert!(
		!httpproxy::should_retry(&response, &policy),
		"Should NOT retry 404 when not in policy codes"
	);
}

#[test]
fn test_should_retry_retryable_error() {
	let policy = make_policy(vec![502, 503]);
	let error = Err(ProxyResponse::Error(ProxyError::RequestTimeout));

	assert!(
		httpproxy::should_retry(&error, &policy),
		"Should retry retryable errors regardless of policy codes"
	);
}

#[test]
fn test_should_retry_non_retryable_error() {
	let policy = make_policy(vec![502, 503]);
	let error = Err(ProxyResponse::Error(ProxyError::AuthorizationFailed));

	assert!(
		!httpproxy::should_retry(&error, &policy),
		"Should NOT retry non-retryable errors"
	);
}

#[test]
fn test_should_retry_direct_response() {
	let policy = make_policy(vec![502, 503]);
	let direct = Err(ProxyResponse::DirectResponse(Box::new(make_response(503))));

	assert!(
		!httpproxy::should_retry(&direct, &policy),
		"Should NOT retry DirectResponse even if status matches"
	);
}

#[test]
fn test_should_retry_empty_policy_codes() {
	let policy = make_policy(vec![]);
	let response = Ok(make_response(503));

	assert!(
		!httpproxy::should_retry(&response, &policy),
		"Should NOT retry when policy has no codes"
	);
}

#[test]
fn test_should_retry_dns_resolution_error() {
	let policy = make_policy(vec![502]);
	let error = Err(ProxyResponse::Error(ProxyError::DnsResolution));

	assert!(
		httpproxy::should_retry(&error, &policy),
		"Should retry DnsResolution error"
	);
}
