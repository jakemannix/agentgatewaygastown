//! Pattern executors for resilient operation composition.
//!
//! This module provides executors for stateful composition patterns like
//! Timeout, Retry, Circuit Breaker, etc.

mod timeout;

pub use timeout::TimeoutExecutor;

use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;

use crate::types::stateful::Operation;

/// Errors that can occur during pattern execution.
#[derive(Error, Debug, Clone)]
pub enum ExecutionError {
    #[error("operation timed out after {0}ms")]
    Timeout(u64),

    #[error("operation timed out: {0}")]
    TimeoutWithMessage(String),

    #[error("operation failed: {0}")]
    OperationFailed(String),

    #[error("tool not found: {0}")]
    ToolNotFound(String),

    #[error("internal error: {0}")]
    Internal(String),
}

/// Context for executing operations.
///
/// Provides access to execution environment, tool registry, and other
/// resources needed during operation execution.
#[derive(Clone)]
pub struct ExecutionContext {
    /// Optional tool executor for handling tool calls.
    tool_executor: Option<Arc<dyn ToolExecutor>>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionContext {
    /// Create a new execution context.
    pub fn new() -> Self {
        Self {
            tool_executor: None,
        }
    }

    /// Create an execution context with a tool executor.
    pub fn with_tool_executor(executor: Arc<dyn ToolExecutor>) -> Self {
        Self {
            tool_executor: Some(executor),
        }
    }

    /// Get the tool executor if available.
    pub fn tool_executor(&self) -> Option<&Arc<dyn ToolExecutor>> {
        self.tool_executor.as_ref()
    }
}

impl std::fmt::Debug for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("has_tool_executor", &self.tool_executor.is_some())
            .finish()
    }
}

/// Trait for executing tools.
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool with the given name and arguments.
    async fn execute(&self, tool_name: &str, arguments: Option<&Value>) -> Result<Value, ExecutionError>;
}

/// Main executor for composed operations.
///
/// Handles execution of all operation types including pattern-wrapped operations.
pub struct CompositionExecutor;

impl CompositionExecutor {
    /// Create a new composition executor.
    pub fn new() -> Self {
        Self
    }

    /// Execute an operation.
    ///
    /// Returns a boxed future to handle recursive pattern compositions.
    pub fn execute_operation<'a>(
        &'a self,
        operation: &'a Operation,
        input: Value,
        ctx: &'a ExecutionContext,
    ) -> Pin<Box<dyn Future<Output = Result<Value, ExecutionError>> + Send + 'a>> {
        Box::pin(async move {
            match operation {
                Operation::Constant { value } => Ok(value.clone()),
                Operation::ToolCall { tool_name, arguments } => {
                    let executor = ctx
                        .tool_executor()
                        .ok_or_else(|| ExecutionError::Internal("No tool executor configured".into()))?;

                    // Merge input with arguments if provided
                    let args = match arguments {
                        Some(args) => Some(args),
                        None if !input.is_null() => Some(&input),
                        None => None,
                    };

                    executor.execute(tool_name, args).await
                }
                Operation::Timeout(spec) => {
                    TimeoutExecutor::execute(spec, input, ctx, self).await
                }
            }
        })
    }
}

impl Default for CompositionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_execute_constant_operation() {
        let executor = CompositionExecutor::new();
        let ctx = ExecutionContext::new();
        let op = Operation::constant(json!({"result": "success"}));

        let result = executor.execute_operation(&op, Value::Null, &ctx).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"result": "success"}));
    }

    #[tokio::test]
    async fn test_execute_tool_call_without_executor() {
        let executor = CompositionExecutor::new();
        let ctx = ExecutionContext::new();
        let op = Operation::tool_call("test_tool");

        let result = executor.execute_operation(&op, Value::Null, &ctx).await;

        assert!(result.is_err());
        match result {
            Err(ExecutionError::Internal(msg)) => {
                assert!(msg.contains("No tool executor"));
            }
            _ => panic!("Expected Internal error"),
        }
    }
}
