//! Tests for the Cache pattern executor.
//!
//! These tests follow TDD principles and cover:
//! - Cache miss then hit behavior
//! - TTL expiration
//! - Key derivation from multiple paths
//! - Conditional caching with predicates
//! - Stale-while-revalidate behavior

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use super::*;
use crate::stateful::memory::MemoryStore;

/// Helper to create a simple inner execution future that returns a value
async fn simple_inner(value: Value) -> Result<Value, String> {
    Ok(value)
}

/// Helper to create an inner execution that tracks call count
fn counting_inner(
    counter: Arc<AtomicU32>,
    value: Value,
) -> impl std::future::Future<Output = Result<Value, String>> {
    async move {
        counter.fetch_add(1, Ordering::SeqCst);
        Ok(value)
    }
}

#[tokio::test]
async fn test_cache_miss_then_hit() {
    let store = MemoryStore::new();
    let call_count = Arc::new(AtomicU32::new(0));

    let spec = CacheSpec::new(vec!["user_id".to_string()], 60);
    let input = json!({"user_id": "user123"});
    let expected_result = json!({"data": "result"});

    // First call - cache miss
    let result = CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), expected_result.clone()),
    )
    .await
    .unwrap();

    assert_eq!(result, expected_result);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Second call - cache hit
    let result = CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), json!({"data": "different"})),
    )
    .await
    .unwrap();

    // Should return cached value, not the new one
    assert_eq!(result, expected_result);
    // Inner should NOT have been called again
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_cache_ttl_expiry() {
    let store = MemoryStore::new();
    let call_count = Arc::new(AtomicU32::new(0));

    // Very short TTL for testing
    let spec = CacheSpec::new(vec!["id".to_string()], 1); // 1 second TTL
    let input = json!({"id": "test"});
    let first_result = json!({"version": 1});
    let second_result = json!({"version": 2});

    // First call
    let result = CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), first_result.clone()),
    )
    .await
    .unwrap();

    assert_eq!(result, first_result);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // After TTL expiry, should get new result
    let result = CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), second_result.clone()),
    )
    .await
    .unwrap();

    assert_eq!(result, second_result);
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_cache_key_derivation() {
    let store = MemoryStore::new();

    // Multiple key paths
    let spec = CacheSpec::new(
        vec!["user.id".to_string(), "action".to_string()],
        60,
    );

    let input1 = json!({
        "user": {"id": "user1"},
        "action": "read"
    });
    let input2 = json!({
        "user": {"id": "user1"},
        "action": "write"
    });
    let input3 = json!({
        "user": {"id": "user2"},
        "action": "read"
    });

    // Cache result for user1:read
    let result1 = json!({"cached": "user1-read"});
    CacheExecutor::execute(&spec, input1.clone(), &store, simple_inner(result1.clone()))
        .await
        .unwrap();

    // Different action should miss cache
    let result2 = json!({"cached": "user1-write"});
    let call_count = Arc::new(AtomicU32::new(0));
    CacheExecutor::execute(
        &spec,
        input2.clone(),
        &store,
        counting_inner(call_count.clone(), result2.clone()),
    )
    .await
    .unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Different user should miss cache
    let result3 = json!({"cached": "user2-read"});
    let call_count = Arc::new(AtomicU32::new(0));
    CacheExecutor::execute(
        &spec,
        input3.clone(),
        &store,
        counting_inner(call_count.clone(), result3.clone()),
    )
    .await
    .unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Same input as input1 should hit cache
    let call_count = Arc::new(AtomicU32::new(0));
    let cached = CacheExecutor::execute(
        &spec,
        input1.clone(),
        &store,
        counting_inner(call_count.clone(), json!({"different": true})),
    )
    .await
    .unwrap();
    assert_eq!(cached, result1);
    assert_eq!(call_count.load(Ordering::SeqCst), 0); // No call made
}

#[tokio::test]
async fn test_cache_conditional() {
    let store = MemoryStore::new();

    // Only cache if status is "success"
    let spec = CacheSpec::new(vec!["id".to_string()], 60)
        .with_cache_if("status".to_string(), json!("success"));

    let input = json!({"id": "test"});

    // First call with error status - should NOT cache
    let error_result = json!({"status": "error", "message": "failed"});
    let call_count = Arc::new(AtomicU32::new(0));
    CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), error_result.clone()),
    )
    .await
    .unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Second call should still execute because error wasn't cached
    let success_result = json!({"status": "success", "data": "good"});
    let call_count = Arc::new(AtomicU32::new(0));
    let result = CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), success_result.clone()),
    )
    .await
    .unwrap();
    assert_eq!(result, success_result);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Third call should hit cache because success was cached
    let call_count = Arc::new(AtomicU32::new(0));
    let result = CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), json!({"different": true})),
    )
    .await
    .unwrap();
    assert_eq!(result, success_result);
    assert_eq!(call_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_cache_stale_while_revalidate() {
    let store = MemoryStore::new();

    // 1 second TTL with 2 second SWR window
    let spec = CacheSpec::new(vec!["id".to_string()], 1)
        .with_stale_while_revalidate(2);

    let input = json!({"id": "swr-test"});
    let original_result = json!({"version": "original"});

    // Initial call - cache miss
    let call_count = Arc::new(AtomicU32::new(0));
    let result = CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), original_result.clone()),
    )
    .await
    .unwrap();
    assert_eq!(result, original_result);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Wait for TTL to expire but stay within SWR window
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // Should return stale value without calling inner
    let call_count = Arc::new(AtomicU32::new(0));
    let result = CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), json!({"version": "new"})),
    )
    .await
    .unwrap();
    assert_eq!(result, original_result); // Still returns stale value
    assert_eq!(call_count.load(Ordering::SeqCst), 0); // No call made (in basic SWR)

    // Wait for SWR window to also expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Now should call inner and get new value
    let new_result = json!({"version": "new"});
    let call_count = Arc::new(AtomicU32::new(0));
    let result = CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), new_result.clone()),
    )
    .await
    .unwrap();
    assert_eq!(result, new_result);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_derive_cache_key_simple() {
    let input = json!({"user": "alice", "action": "login"});

    let key = derive_cache_key(&["user".to_string()], &input).unwrap();
    assert_eq!(key, "alice");

    let key = derive_cache_key(&["user".to_string(), "action".to_string()], &input).unwrap();
    assert_eq!(key, "alice:login");
}

#[tokio::test]
async fn test_derive_cache_key_nested() {
    let input = json!({
        "request": {
            "user": {"id": 123, "name": "Bob"},
            "method": "GET"
        }
    });

    let key = derive_cache_key(&["request.user.id".to_string()], &input).unwrap();
    assert_eq!(key, "123");

    let key = derive_cache_key(
        &["request.user.name".to_string(), "request.method".to_string()],
        &input,
    )
    .unwrap();
    assert_eq!(key, "Bob:GET");
}

#[tokio::test]
async fn test_derive_cache_key_missing_path() {
    let input = json!({"user": "alice"});

    let result = derive_cache_key(&["missing".to_string()], &input);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CacheError::KeyDerivation(_)));
}

#[tokio::test]
async fn test_evaluate_predicate_simple() {
    let result = json!({"status": "success", "code": 200});

    assert!(evaluate_predicate(
        &CachePredicate {
            field: "status".to_string(),
            equals: json!("success"),
        },
        &result
    ));

    assert!(!evaluate_predicate(
        &CachePredicate {
            field: "status".to_string(),
            equals: json!("error"),
        },
        &result
    ));

    assert!(evaluate_predicate(
        &CachePredicate {
            field: "code".to_string(),
            equals: json!(200),
        },
        &result
    ));
}

#[tokio::test]
async fn test_evaluate_predicate_nested() {
    let result = json!({
        "response": {
            "status": {"code": "OK"}
        }
    });

    assert!(evaluate_predicate(
        &CachePredicate {
            field: "response.status.code".to_string(),
            equals: json!("OK"),
        },
        &result
    ));
}

#[tokio::test]
async fn test_cache_with_array_key() {
    let store = MemoryStore::new();
    let spec = CacheSpec::new(vec!["ids".to_string()], 60);

    let input = json!({"ids": [1, 2, 3]});
    let result = json!({"sum": 6});

    // Cache with array key
    CacheExecutor::execute(&spec, input.clone(), &store, simple_inner(result.clone()))
        .await
        .unwrap();

    // Same array should hit cache
    let call_count = Arc::new(AtomicU32::new(0));
    let cached = CacheExecutor::execute(
        &spec,
        input.clone(),
        &store,
        counting_inner(call_count.clone(), json!({"sum": 999})),
    )
    .await
    .unwrap();
    assert_eq!(cached, result);
    assert_eq!(call_count.load(Ordering::SeqCst), 0);

    // Different array should miss cache
    let different_input = json!({"ids": [1, 2, 4]});
    let call_count = Arc::new(AtomicU32::new(0));
    CacheExecutor::execute(
        &spec,
        different_input,
        &store,
        counting_inner(call_count.clone(), json!({"sum": 7})),
    )
    .await
    .unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}
