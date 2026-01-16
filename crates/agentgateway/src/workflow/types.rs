//! Core types for workflow patterns.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A field predicate that evaluates to true or false based on input data.
///
/// Uses CEL-like expressions to evaluate conditions on JSON input.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldPredicate {
	/// The field path to evaluate (e.g., "input.type", "data.status")
	pub field: String,
	/// The operator for comparison
	pub operator: PredicateOperator,
	/// The value to compare against
	pub value: Value,
}

/// Operators for field predicate comparisons.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PredicateOperator {
	/// Equals comparison
	Eq,
	/// Not equals comparison
	Neq,
	/// Greater than
	Gt,
	/// Greater than or equal
	Gte,
	/// Less than
	Lt,
	/// Less than or equal
	Lte,
	/// Contains (for strings or arrays)
	Contains,
	/// Starts with (for strings)
	StartsWith,
	/// Ends with (for strings)
	EndsWith,
	/// Matches regex pattern
	Matches,
	/// Value exists (is not null)
	Exists,
	/// Value is in a list
	In,
}

/// A step operation in a workflow.
///
/// This is the base type for all workflow operations that can be executed.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum StepOperation {
	/// A no-op that passes through the input unchanged
	Passthrough,

	/// Transforms the input using a template or expression
	Transform {
		/// CEL expression or template for transformation
		expression: String,
	},

	/// Calls an external tool/service
	ToolCall {
		/// The tool name to invoke
		tool: String,
		/// Arguments to pass to the tool
		#[serde(default)]
		args: Value,
	},

	/// Routes to different operations based on predicates
	Router(Box<RouterSpec>),

	/// A sequence of operations to execute in order
	Sequence {
		/// The steps to execute
		steps: Vec<StepOperation>,
	},

	/// Parallel execution of operations
	Parallel {
		/// The operations to execute in parallel
		branches: Vec<StepOperation>,
	},
}

/// Specification for a content router.
///
/// Routes input to different operations based on matching predicates.
/// Evaluates routes in order and executes the first match.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RouterSpec {
	/// The routes to evaluate, in order
	pub routes: Vec<RouteCase>,
	/// Operation to execute if no route matches
	#[serde(default)]
	pub otherwise: Option<Box<StepOperation>>,
}

/// A single route case with a predicate and target operation.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RouteCase {
	/// The condition that must be true for this route to match
	pub when: FieldPredicate,
	/// The operation to execute when the condition matches
	pub then: StepOperation,
}

/// Error types for workflow execution.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExecutionError {
	/// No route matched and no otherwise clause was provided
	#[error("no route matched and no otherwise clause provided")]
	NoRouteMatch,

	/// Failed to evaluate a predicate
	#[error("predicate evaluation failed: {0}")]
	PredicateError(String),

	/// Failed to execute an operation
	#[error("operation execution failed: {0}")]
	OperationError(String),

	/// Invalid field path
	#[error("invalid field path: {0}")]
	InvalidFieldPath(String),

	/// Type mismatch in comparison
	#[error("type mismatch: expected {expected}, got {actual}")]
	TypeMismatch { expected: String, actual: String },
}

/// Result of executing a workflow step.
pub type ExecutionResult = Result<Value, ExecutionError>;
