//! Stateful patterns for agent gateway operations.
//!
//! This module provides patterns for stateful operations like caching,
//! rate limiting, and other state-dependent behaviors.

mod cache;
mod store;

pub use cache::{CacheError, CacheExecutor, CacheSpec, derive_cache_key, evaluate_predicate};
pub use store::{StateStore, StateStoreExt, StoreError};

#[cfg(any(test, feature = "testing"))]
pub mod memory;
