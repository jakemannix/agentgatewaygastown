// Tool Registry Module
//
// Provides virtual tool abstraction allowing:
// - Tool renaming and aliasing
// - Field hiding and default injection
// - Output transformation via JSONPath
// - Hot-reloadable registry from file or HTTP sources

mod client;
mod compiled;
mod error;
mod store;
mod types;

pub use client::{parse_duration, AuthConfig, RegistryClient, RegistrySource};
pub use compiled::{CompiledOutputField, CompiledRegistry, CompiledVirtualTool};
pub use error::RegistryError;
pub use store::{RegistryStore, RegistryStoreRef};
pub use types::{OutputField, OutputSchema, Registry, ToolSource, VirtualToolDef};
