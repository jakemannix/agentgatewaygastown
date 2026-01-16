use std::time::Duration;

use async_trait::async_trait;

/// Error type for StateStore operations
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("key not found")]
    NotFound,
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("storage error: {0}")]
    Storage(String),
}

/// StateStore trait for async key-value storage with TTL support.
///
/// This trait provides the abstraction for caching values with optional
/// time-to-live (TTL) expiration.
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Get a value by key.
    ///
    /// Returns `Ok(Some(bytes))` if the key exists and hasn't expired,
    /// `Ok(None)` if the key doesn't exist or has expired.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError>;

    /// Set a value with an optional TTL.
    ///
    /// If `ttl` is `None`, the value will not expire.
    /// If `ttl` is `Some(duration)`, the value will expire after that duration.
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<(), StoreError>;

    /// Delete a key.
    ///
    /// Returns `Ok(())` regardless of whether the key existed.
    async fn delete(&self, key: &str) -> Result<(), StoreError>;

    /// Check if a key exists and hasn't expired.
    async fn exists(&self, key: &str) -> Result<bool, StoreError> {
        Ok(self.get(key).await?.is_some())
    }
}

/// Extension trait for StateStore that provides convenience methods
#[async_trait]
pub trait StateStoreExt: StateStore {
    /// Get a value and deserialize it from JSON.
    async fn get_json<T: serde::de::DeserializeOwned + Send>(
        &self,
        key: &str,
    ) -> Result<Option<T>, StoreError> {
        match self.get(key).await? {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| StoreError::Serialization(e.to_string())),
            None => Ok(None),
        }
    }

    /// Serialize a value to JSON and store it.
    async fn set_json<T: serde::Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> Result<(), StoreError> {
        let bytes =
            serde_json::to_vec(value).map_err(|e| StoreError::Serialization(e.to_string()))?;
        self.set(key, bytes, ttl).await
    }
}

// Blanket implementation for all StateStore implementations
impl<T: StateStore + ?Sized> StateStoreExt for T {}
