// Tool Registry Module
//
// Provides virtual tool abstraction and tool composition allowing:
// - Tool renaming and aliasing (1:1 virtual tools)
// - Tool composition and orchestration (N:1 compositions)
// - Field hiding and default injection
// - Output transformation via JSONPath
// - Hot-reloadable registry from file or HTTP sources
//
// ## Implementation Status
//
// ### Implemented (Runtime Works)
// - Source Tool (1:1 mapping)
// - Pipeline (sequential execution)
// - Scatter-Gather (parallel fan-out with aggregation)
// - Filter (predicate-based filtering)
// - SchemaMap (field transformation)
// - MapEach (array element processing)
// - Output Transform
//
// ### IR Only (No Runtime Executor)
// - Retry, Timeout, Cache, CircuitBreaker, Idempotent, DeadLetter, Saga, ClaimCheck
// - See `tests/fixtures/registry/README.md` for details

mod client;
mod compiled;
mod error;
pub mod execution_graph;
pub mod executor;
pub mod patterns;
pub mod runtime_hooks;
mod store;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

pub use client::{AuthConfig, RegistryClient, RegistrySource, parse_duration};
pub use compiled::{
	CompiledComposition, CompiledFieldSource, CompiledImplementation, CompiledOutputField,
	CompiledOutputTransform, CompiledRegistry, CompiledSourceTool, CompiledTool, CompiledVirtualTool,
	VIRTUAL_SERVER_NAME,
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
	AgentDefinition, Dependency, DependencyType, OutputField, OutputSchema, OutputTransform,
	Registry, Schema, Server, SourceTool, ToolDefinition, ToolImplementation, ToolSource,
	UnknownCallerPolicy, VirtualToolDef,
};
pub use validation::{validate_registry, RegistryValidator, ValidationError, ValidationResult, ValidationWarning};
pub use runtime_hooks::{CallerIdentity, CallContext, DependencyCheckResult, RuntimeHooks, ToolVisibility};

// Executor exports
pub use execution_graph::{ExecutionGraph, ExecutionNode, NodeInput, NodeOperation};
pub use executor::{
	CompositionExecutor, ExecutionContext, ExecutionError, FilterExecutor, MapEachExecutor,
	PipelineExecutor, ScatterGatherExecutor, SchemaMapExecutor, ToolInvoker,
};
