// Tool Registry Module
//
// Provides virtual tool abstraction and tool composition allowing:
// - Tool renaming and aliasing (1:1 virtual tools)
// - Tool composition and orchestration (N:1 compositions)
// - Field hiding and default injection
// - Output transformation via JSONPath
// - Hot-reloadable registry from file or HTTP sources

mod client;
mod compiled;
mod error;
pub mod execution_graph;
pub mod executor;
pub mod patterns;
mod store;
mod types;

pub use client::{parse_duration, AuthConfig, RegistryClient, RegistrySource};
pub use compiled::{
	CompiledComposition, CompiledFieldSource, CompiledImplementation, CompiledOutputField,
	CompiledOutputTransform, CompiledRegistry, CompiledSourceTool, CompiledTool, CompiledVirtualTool,
};
pub use error::RegistryError;
pub use patterns::{
	AggregationOp, AggregationStrategy, CoalesceSource, ConcatSource, DataBinding, DedupeOp,
	FieldPredicate, FieldSource, FilterSpec, InputBinding, LimitOp, LiteralValue, MapEachInner,
	MapEachSpec, PatternSpec, PipelineSpec, PipelineStep, PredicateValue, ScatterGatherSpec,
	ScatterTarget, SchemaMapSpec, SortOp, StepBinding, StepOperation, TemplateSource, ToolCall,
};
pub use store::{RegistryStore, RegistryStoreRef};
pub use types::{
	OutputField, OutputSchema, OutputTransform, Registry, SourceTool, ToolDefinition,
	ToolImplementation, ToolSource, VirtualToolDef,
};

// Executor exports
pub use execution_graph::{ExecutionGraph, ExecutionNode, NodeInput, NodeOperation};
pub use executor::{
	CompositionExecutor, ExecutionContext, ExecutionError, FilterExecutor, MapEachExecutor,
	PipelineExecutor, ScatterGatherExecutor, SchemaMapExecutor, ToolInvoker,
};
