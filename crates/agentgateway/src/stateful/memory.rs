//! In-memory implementation of StateStore for testing.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;

use super::store::{StateStore, StoreError};

/// Entry in the memory store with optional expiration
struct MemoryEntry {
    value: Vec<u8>,
    expires_at: Option<Instant>,
}

impl MemoryEntry {
    fn new(value: Vec<u8>, ttl: Option<Duration>) -> Self {
        Self {
            value,
            expires_at: ttl.map(|d| Instant::now() + d),
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |exp| Instant::now() > exp)
    }
}

/// In-memory implementation of StateStore.
///
/// This implementation is suitable for testing and single-instance deployments.
/// For production use with multiple instances, use a distributed store like Redis.
#[derive(Default)]
pub struct MemoryStore {
    data: Mutex<HashMap<String, MemoryEntry>>,
}

impl MemoryStore {
    /// Create a new empty memory store.
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }

    /// Clear all entries from the store.
    pub fn clear(&self) {
        self.data.lock().unwrap().clear();
    }

    /// Get the number of entries in the store (including expired ones).
    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.data.lock().unwrap().is_empty()
    }
}

#[async_trait]
impl StateStore for MemoryStore {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let mut data = self.data.lock().unwrap();

        // Check if key exists and isn't expired
        if let Some(entry) = data.get(key) {
            if entry.is_expired() {
                data.remove(key);
                return Ok(None);
            }
            return Ok(Some(entry.value.clone()));
        }

        Ok(None)
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<(), StoreError> {
        let mut data = self.data.lock().unwrap();
        data.insert(key.to_string(), MemoryEntry::new(value, ttl));
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), StoreError> {
        let mut data = self.data.lock().unwrap();
        data.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_store_basic() {
        let store = MemoryStore::new();

        // Initially empty
        assert!(store.get("key1").await.unwrap().is_none());

        // Set and get
        store
            .set("key1", b"value1".to_vec(), None)
            .await
            .unwrap();
        assert_eq!(
            store.get("key1").await.unwrap(),
            Some(b"value1".to_vec())
        );

        // Delete
        store.delete("key1").await.unwrap();
        assert!(store.get("key1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_memory_store_ttl() {
        let store = MemoryStore::new();

        // Set with very short TTL
        store
            .set("key1", b"value1".to_vec(), Some(Duration::from_millis(50)))
            .await
            .unwrap();

        // Should exist immediately
        assert!(store.get("key1").await.unwrap().is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should be expired now
        assert!(store.get("key1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_memory_store_overwrite() {
        let store = MemoryStore::new();

        store
            .set("key1", b"value1".to_vec(), None)
            .await
            .unwrap();
        store
            .set("key1", b"value2".to_vec(), None)
            .await
            .unwrap();

        assert_eq!(
            store.get("key1").await.unwrap(),
            Some(b"value2".to_vec())
        );
    }
}
