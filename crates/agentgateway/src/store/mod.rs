mod binds;

use std::sync::Arc;

pub use binds::{
	BackendPolicies, FrontendPolices, GatewayPolicies, LLMRequestPolicies, LLMResponsePolicies,
	RoutePath, RoutePolicies, Store as BindStore,
};
use serde::{Serialize, Serializer};
mod discovery;
use std::sync::RwLock;

pub use binds::PreviousState as BindPreviousState;
pub use discovery::{
	LocalWorkload, PreviousState as DiscoveryPreviousState, Store as DiscoveryStore, WorkloadStore,
};

use crate::mcp::registry::RegistryStoreRef;
use crate::store;

#[derive(Clone, Debug)]
pub enum Event<T> {
	Add(T),
	Remove(T),
}

#[derive(Clone, Debug)]
pub struct Stores {
	pub discovery: discovery::StoreUpdater,
	pub binds: binds::StoreUpdater,
	/// Tool registry store for virtual tool mappings
	pub registry: Arc<RwLock<Option<RegistryStoreRef>>>,
}

impl Default for Stores {
	fn default() -> Self {
		Self::new()
	}
}

impl Stores {
	pub fn new() -> Stores {
		Stores {
			discovery: discovery::StoreUpdater::new(Arc::new(RwLock::new(discovery::Store::new()))),
			binds: binds::StoreUpdater::new(Arc::new(RwLock::new(binds::Store::new()))),
			registry: Arc::new(RwLock::new(None)),
		}
	}
	pub fn read_binds(&self) -> std::sync::RwLockReadGuard<'_, store::BindStore> {
		self.binds.read()
	}

	pub fn read_discovery(&self) -> std::sync::RwLockReadGuard<'_, store::DiscoveryStore> {
		self.discovery.read()
	}

	/// Set the registry store
	pub fn set_registry(&self, registry: Option<RegistryStoreRef>) {
		if let Ok(mut reg) = self.registry.write() {
			*reg = registry;
		}
	}

	/// Get the registry store
	pub fn get_registry(&self) -> Option<RegistryStoreRef> {
		self.registry.read().ok().and_then(|r| r.clone())
	}
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StoresDump {
	#[serde(flatten)]
	discovery: discovery::Dump,
	#[serde(flatten)]
	binds: binds::Dump,
}

impl Serialize for Stores {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let serializable = StoresDump {
			discovery: self.discovery.dump(),
			binds: self.binds.dump(),
		};
		serializable.serialize(serializer)
	}
}
