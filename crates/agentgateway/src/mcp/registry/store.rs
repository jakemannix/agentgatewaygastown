// Registry store for hot-reloadable registry management

use std::path::{PathBuf, absolute};
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use notify::{EventKind, RecursiveMode};
use tracing::{error, info, warn};

use super::client::RegistryClient;
use super::compiled::CompiledRegistry;
use super::error::RegistryError;
use super::types::Registry;

/// Store for managing the compiled registry with hot-reload support
#[derive(Debug)]
pub struct RegistryStore {
	/// Current compiled registry (atomically swappable)
	current: Arc<ArcSwap<Option<CompiledRegistry>>>,
	/// Client for fetching updates (optional - None means static registry)
	client: Option<RegistryClient>,
}

impl Clone for RegistryStore {
	fn clone(&self) -> Self {
		Self {
			current: Arc::clone(&self.current),
			client: self.client.clone(),
		}
	}
}

impl Default for RegistryStore {
	fn default() -> Self {
		Self::new()
	}
}

impl RegistryStore {
	/// Create a new empty registry store
	pub fn new() -> Self {
		Self {
			current: Arc::new(ArcSwap::new(Arc::new(None))),
			client: None,
		}
	}

	/// Create a registry store with a client for fetching updates
	pub fn with_client(mut self, client: RegistryClient) -> Self {
		self.client = Some(client);
		self
	}

	/// Get current compiled registry (returns None if no registry configured)
	///
	/// Returns a guard that provides access to the registry. The registry
	/// remains valid as long as the guard is held.
	pub fn get(&self) -> arc_swap::Guard<Arc<Option<CompiledRegistry>>> {
		self.current.load()
	}

	/// Check if a registry is loaded
	pub fn has_registry(&self) -> bool {
		self.current.load().is_some()
	}

	/// Update registry with new data
	pub fn update(&self, registry: Registry) -> Result<(), RegistryError> {
		let compiled = CompiledRegistry::compile(registry)?;
		self.current.store(Arc::new(Some(compiled)));
		info!("Registry updated successfully");
		Ok(())
	}

	/// Update registry with pre-compiled data
	pub fn update_compiled(&self, compiled: CompiledRegistry) {
		self.current.store(Arc::new(Some(compiled)));
		info!("Registry updated with compiled data");
	}

	/// Clear the registry
	pub fn clear(&self) {
		self.current.store(Arc::new(None));
		info!("Registry cleared");
	}

	/// Get the configured client
	pub fn client(&self) -> Option<&RegistryClient> {
		self.client.as_ref()
	}

	/// Initial load from configured source
	pub async fn initial_load(&self) -> Result<(), RegistryError> {
		let Some(client) = &self.client else {
			return Ok(());
		};

		let registry = client.fetch().await?;
		self.update(registry)?;
		Ok(())
	}

	/// Start background refresh loop (for HTTP sources)
	pub fn spawn_refresh_loop(self: Arc<Self>) -> Option<tokio::task::JoinHandle<()>> {
		let client = self.client.as_ref()?;

		// Only spawn for HTTP sources
		if client.is_file_source() {
			return None;
		}

		let interval = client.refresh_interval();
		let store = self;

		Some(tokio::spawn(async move {
			info!(
				"Starting registry refresh loop with interval {:?}",
				interval
			);

			loop {
				tokio::time::sleep(interval).await;

				let Some(client) = &store.client else {
					break;
				};

				match client.fetch().await {
					Ok(registry) => {
						if let Err(e) = store.update(registry) {
							warn!("Failed to compile registry: {}", e);
						}
					},
					Err(e) => {
						warn!("Failed to fetch registry: {}", e);
						// Keep the old registry on fetch failure
					},
				}
			}
		}))
	}

	/// Start file watcher (for file:// sources)
	pub fn spawn_file_watcher(self: Arc<Self>) -> Result<Option<tokio::task::JoinHandle<()>>, RegistryError> {
		let Some(client) = &self.client else {
			return Ok(None);
		};

		let Some(path) = client.file_path() else {
			return Ok(None);
		};

		let path = path.clone();
		let store = self;

		let handle = tokio::spawn(async move {
			if let Err(e) = store.watch_file(&path).await {
				error!("File watcher error: {}", e);
			}
		});

		Ok(Some(handle))
	}

	/// Watch a file for changes
	async fn watch_file(&self, path: &PathBuf) -> Result<(), RegistryError> {
		let (tx, mut rx) = tokio::sync::mpsc::channel(1);

		// Create a watcher with a 250ms debounce
		let mut watcher =
			notify_debouncer_full::new_debouncer(Duration::from_millis(250), None, move |res| {
				futures::executor::block_on(async {
					let _ = tx.send(res).await;
				})
			})
			.map_err(|e| RegistryError::FetchError(format!("Failed to create file watcher: {}", e)))?;

		// Watch the parent directory
		let abspath = absolute(path)
			.map_err(|e| RegistryError::FetchError(format!("Failed to get absolute path: {}", e)))?;
		let parent = abspath
			.parent()
			.ok_or_else(|| RegistryError::FetchError("Failed to get parent directory".into()))?;

		watcher
			.watch(parent, RecursiveMode::NonRecursive)
			.map_err(|e| RegistryError::FetchError(format!("Failed to watch file: {}", e)))?;

		info!("Watching registry file: {}", path.display());

		// Handle file change events
		while let Some(Ok(events)) = rx.recv().await {
			// Check if any event matches our file
			if events.iter().any(|e| {
				matches!(e.kind, EventKind::Modify(_) | EventKind::Create(_))
					&& e.paths.iter().any(|p| p == &abspath)
			}) {
				info!("Registry file changed, reloading...");

				if let Some(client) = &self.client {
					match client.fetch().await {
						Ok(registry) => {
							if let Err(e) = self.update(registry) {
								error!("Failed to compile registry: {}", e);
							} else {
								info!("Registry reloaded successfully");
							}
						},
						Err(e) => {
							error!("Failed to reload registry: {}", e);
						},
					}
				}
			}
		}

		drop(watcher);
		Ok(())
	}
}

/// Wrapper for thread-safe access to the registry store
#[derive(Debug, Clone)]
pub struct RegistryStoreRef {
	inner: Arc<RegistryStore>,
}

impl RegistryStoreRef {
	/// Create a new registry store reference
	pub fn new(store: RegistryStore) -> Self {
		Self {
			inner: Arc::new(store),
		}
	}

	/// Get the inner Arc
	pub fn inner(&self) -> &Arc<RegistryStore> {
		&self.inner
	}

	/// Get the current compiled registry
	pub fn get(&self) -> arc_swap::Guard<Arc<Option<CompiledRegistry>>> {
		self.inner.get()
	}

	/// Check if a registry is loaded
	pub fn has_registry(&self) -> bool {
		self.inner.has_registry()
	}

	/// Update the registry
	pub fn update(&self, registry: Registry) -> Result<(), RegistryError> {
		self.inner.update(registry)
	}

	/// Initial load
	pub async fn initial_load(&self) -> Result<(), RegistryError> {
		self.inner.initial_load().await
	}

	/// Start background tasks (refresh loop or file watcher)
	pub fn start_background_tasks(&self) -> Vec<tokio::task::JoinHandle<()>> {
		let mut handles = Vec::new();

		// Try refresh loop (for HTTP sources)
		if let Some(handle) = Arc::clone(&self.inner).spawn_refresh_loop() {
			handles.push(handle);
		}

		// Try file watcher (for file sources)
		if let Ok(Some(handle)) = Arc::clone(&self.inner).spawn_file_watcher() {
			handles.push(handle);
		}

		handles
	}
}

impl Default for RegistryStoreRef {
	fn default() -> Self {
		Self::new(RegistryStore::new())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mcp::registry::types::VirtualToolDef;

	#[test]
	fn test_empty_store() {
		let store = RegistryStore::new();
		assert!(!store.has_registry());
	}

	#[test]
	fn test_update_store() {
		let store = RegistryStore::new();

		let tool = VirtualToolDef::new("test_tool", "backend", "source_tool");
		let registry = Registry::with_tools(vec![tool]);

		store.update(registry).unwrap();
		assert!(store.has_registry());
	}

	#[test]
	fn test_clear_store() {
		let store = RegistryStore::new();

		let tool = VirtualToolDef::new("test_tool", "backend", "source_tool");
		let registry = Registry::with_tools(vec![tool]);

		store.update(registry).unwrap();
		assert!(store.has_registry());

		store.clear();
		assert!(!store.has_registry());
	}

	#[test]
	fn test_store_ref() {
		let store = RegistryStoreRef::default();
		assert!(!store.has_registry());

		let tool = VirtualToolDef::new("test_tool", "backend", "source_tool");
		let registry = Registry::with_tools(vec![tool]);

		store.update(registry).unwrap();
		assert!(store.has_registry());
	}
}
