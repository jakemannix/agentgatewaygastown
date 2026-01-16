//! Timeout pattern executor.
//!
//! Executes operations with a timeout, optionally falling back to an
//! alternative operation if the timeout is exceeded.

use serde_json::Value;
use std::time::Duration;

use crate::types::stateful::TimeoutSpec;

use super::{CompositionExecutor, ExecutionContext, ExecutionError};

/// Executor for the Timeout pattern.
pub struct TimeoutExecutor;

impl TimeoutExecutor {
    /// Execute an operation with a timeout.
    ///
    /// If the inner operation completes before the timeout, returns its result.
    /// If the timeout is exceeded:
    /// - Executes the fallback operation if one is configured
    /// - Returns a timeout error otherwise
    pub async fn execute(
        spec: &TimeoutSpec,
        input: Value,
        ctx: &ExecutionContext,
        executor: &CompositionExecutor,
    ) -> Result<Value, ExecutionError> {
        let duration = Duration::from_millis(spec.duration_ms);

        match tokio::time::timeout(
            duration,
            executor.execute_operation(&spec.inner, input.clone(), ctx),
        )
        .await
        {
            Ok(result) => result,
            Err(_elapsed) => {
                // Timeout occurred
                if let Some(ref fallback) = spec.fallback {
                    executor.execute_operation(fallback, input, ctx).await
                } else if let Some(ref message) = spec.error_message {
                    Err(ExecutionError::TimeoutWithMessage(message.clone()))
                } else {
                    Err(ExecutionError::Timeout(spec.duration_ms))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patterns::ToolExecutor;
    use crate::types::stateful::Operation;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use tokio::time::Duration;

    /// A mock tool executor for testing.
    struct MockToolExecutor {
        delay_ms: AtomicU64,
        response: Value,
    }

    impl MockToolExecutor {
        fn new(response: Value) -> Self {
            Self {
                delay_ms: AtomicU64::new(0),
                response,
            }
        }

        fn with_delay(mut self, delay_ms: u64) -> Self {
            self.delay_ms = AtomicU64::new(delay_ms);
            self
        }
    }

    #[async_trait]
    impl ToolExecutor for MockToolExecutor {
        async fn execute(
            &self,
            _tool_name: &str,
            _arguments: Option<&Value>,
        ) -> Result<Value, ExecutionError> {
            let delay = self.delay_ms.load(Ordering::Relaxed);
            if delay > 0 {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn test_timeout_success() {
        // Inner operation completes before timeout
        let mock = Arc::new(MockToolExecutor::new(json!({"result": "success"})));
        let ctx = ExecutionContext::with_tool_executor(mock);
        let executor = CompositionExecutor::new();

        let spec = TimeoutSpec::new(1000, Operation::tool_call("test_tool"));

        let result = TimeoutExecutor::execute(&spec, Value::Null, &ctx, &executor).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"result": "success"}));
    }

    #[tokio::test]
    async fn test_timeout_exceeded() {
        // Inner operation takes longer than timeout
        let mock = Arc::new(
            MockToolExecutor::new(json!({"result": "success"})).with_delay(500),
        );
        let ctx = ExecutionContext::with_tool_executor(mock);
        let executor = CompositionExecutor::new();

        let spec = TimeoutSpec::new(100, Operation::tool_call("slow_tool"));

        let result = TimeoutExecutor::execute(&spec, Value::Null, &ctx, &executor).await;

        assert!(result.is_err());
        match result {
            Err(ExecutionError::Timeout(ms)) => {
                assert_eq!(ms, 100);
            }
            _ => panic!("Expected Timeout error"),
        }
    }

    #[tokio::test]
    async fn test_timeout_with_fallback() {
        // Timeout triggers fallback operation
        let mock = Arc::new(
            MockToolExecutor::new(json!({"result": "success"})).with_delay(500),
        );
        let ctx = ExecutionContext::with_tool_executor(mock);
        let executor = CompositionExecutor::new();

        let inner = Operation::tool_call("slow_tool");
        let fallback = Operation::constant(json!({"result": "fallback", "reason": "timeout"}));
        let spec = TimeoutSpec::new(100, inner).with_fallback(fallback);

        let result = TimeoutExecutor::execute(&spec, Value::Null, &ctx, &executor).await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            json!({"result": "fallback", "reason": "timeout"})
        );
    }

    #[tokio::test]
    async fn test_timeout_custom_message() {
        // Custom error message for timeout
        let mock = Arc::new(
            MockToolExecutor::new(json!({"result": "success"})).with_delay(500),
        );
        let ctx = ExecutionContext::with_tool_executor(mock);
        let executor = CompositionExecutor::new();

        let spec = TimeoutSpec::new(100, Operation::tool_call("slow_tool"))
            .with_error_message("Operation exceeded time limit");

        let result = TimeoutExecutor::execute(&spec, Value::Null, &ctx, &executor).await;

        assert!(result.is_err());
        match result {
            Err(ExecutionError::TimeoutWithMessage(msg)) => {
                assert_eq!(msg, "Operation exceeded time limit");
            }
            _ => panic!("Expected TimeoutWithMessage error"),
        }
    }

    #[tokio::test]
    async fn test_timeout_with_constant_inner() {
        // Constant operations should always succeed quickly
        let ctx = ExecutionContext::new();
        let executor = CompositionExecutor::new();

        let spec = TimeoutSpec::new(100, Operation::constant(json!({"data": "instant"})));

        let result = TimeoutExecutor::execute(&spec, Value::Null, &ctx, &executor).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"data": "instant"}));
    }

    #[tokio::test]
    async fn test_timeout_fallback_can_also_timeout() {
        // If fallback is a timeout operation, it can also timeout
        // This tests nested timeout behavior
        let mock = Arc::new(
            MockToolExecutor::new(json!({"result": "success"})).with_delay(500),
        );
        let ctx = ExecutionContext::with_tool_executor(mock);
        let executor = CompositionExecutor::new();

        // Inner times out, fallback also times out (since it uses same slow tool)
        let inner = Operation::tool_call("slow_tool");
        let fallback = Operation::tool_call("slow_tool"); // Also slow
        let spec = TimeoutSpec::new(100, inner)
            .with_fallback(Operation::Timeout(TimeoutSpec::new(100, fallback)));

        let result = TimeoutExecutor::execute(&spec, Value::Null, &ctx, &executor).await;

        // Fallback also times out
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_timeout_preserves_input() {
        // Verify that input is passed to fallback
        struct InputCapturingExecutor {
            captured: tokio::sync::Mutex<Option<Value>>,
        }

        #[async_trait]
        impl ToolExecutor for InputCapturingExecutor {
            async fn execute(
                &self,
                tool_name: &str,
                arguments: Option<&Value>,
            ) -> Result<Value, ExecutionError> {
                if tool_name == "slow" {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                if let Some(args) = arguments {
                    *self.captured.lock().await = Some(args.clone());
                }
                Ok(json!({"captured": true}))
            }
        }

        let capturing = Arc::new(InputCapturingExecutor {
            captured: tokio::sync::Mutex::new(None),
        });
        let ctx = ExecutionContext::with_tool_executor(capturing.clone());
        let executor = CompositionExecutor::new();

        let inner = Operation::tool_call("slow");
        let fallback = Operation::tool_call("capture");
        let spec = TimeoutSpec::new(100, inner).with_fallback(fallback);

        let input = json!({"key": "value"});
        let _ = TimeoutExecutor::execute(&spec, input.clone(), &ctx, &executor).await;

        let captured = capturing.captured.lock().await;
        assert_eq!(*captured, Some(input));
    }
}
