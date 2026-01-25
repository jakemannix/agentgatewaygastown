# Saga Pattern Design

## Overview

The Saga pattern is a distributed transaction pattern that manages long-running business processes by breaking them into a sequence of local transactions. Each step in a saga has a compensating action that undoes its effects if a subsequent step fails.

In the context of AgentGateway, sagas enable orchestrating multiple tool calls with automatic compensation (rollback) on failure, making multi-step agent workflows more reliable.

## Use Cases

### Travel Booking
Book a flight, hotel, and car rental. If any step fails, cancel the previous bookings.

### Supply Chain
Create order, reserve inventory, process payment, schedule shipping. If shipping fails, refund payment and release inventory.

### User Onboarding
Create account, set up profile, configure notifications, send welcome email. If any step fails, clean up partial state.

## Design

### Saga Definition

A saga consists of:
- **steps**: Ordered list of saga steps to execute
- **timeout**: Optional overall saga timeout
- **output**: Optional output binding to construct the final result

Each step contains:
- **id**: Unique identifier for the step (used in output binding)
- **name**: Human-readable name for logging/debugging
- **action**: The tool call to execute
- **compensate**: Optional tool call to undo this step on failure
- **input**: Input binding (from saga input or previous step outputs)

### Execution Semantics

1. **Forward Execution**: Execute steps in order, tracking each result
2. **On Failure**: Stop forward execution, run compensation in reverse order
3. **Compensation**: Best-effort; individual compensation failures are logged but don't stop the compensation chain
4. **Output**: Construct final output from step results using the output binding

### Input/Output Binding

Steps can reference:
- `$.input` - The saga's input
- `$.steps.<step_id>` - Output of a previous step

Output binding uses JSONPath-like syntax:
```json
{
  "output": {
    "flightConfirmation": { "path": "$.steps.flight.confirmationNumber" },
    "hotelConfirmation": { "path": "$.steps.hotel.confirmationNumber" }
  }
}
```

## Rust IR Types

```rust
/// A saga is a sequence of steps with compensation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saga {
    /// Ordered list of steps to execute
    pub steps: Vec<SagaStep>,
    /// Optional overall timeout for the saga
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Duration>,
    /// Optional output binding to construct the result
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    /// Unique identifier for this step (used in output binding)
    pub id: String,
    /// Human-readable name for logging
    pub name: String,
    /// The action to execute
    pub action: ToolCall,
    /// Optional compensation action to run on failure
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compensate: Option<ToolCall>,
    /// Input binding for this step
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<InputBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name to invoke
    pub name: String,
    /// Arguments (can include path bindings)
    #[serde(default)]
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputBinding {
    /// Direct path reference
    Path { path: String },
    /// Object with nested bindings
    Object(HashMap<String, InputBinding>),
    /// Literal value
    Literal(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputBinding {
    /// Map of output field names to path bindings
    #[serde(flatten)]
    pub fields: HashMap<String, PathBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathBinding {
    /// JSONPath-like path to extract value
    pub path: String,
}
```

## JSON Schema Example

```json
{
  "saga": {
    "steps": [
      {
        "id": "flight",
        "name": "Book flight",
        "action": {
          "name": "airline.book",
          "arguments": {}
        },
        "compensate": {
          "name": "airline.cancel",
          "arguments": {}
        },
        "input": {
          "path": "$.input.flight"
        }
      },
      {
        "id": "hotel",
        "name": "Book hotel",
        "action": {
          "name": "hotel.reserve",
          "arguments": {}
        },
        "compensate": {
          "name": "hotel.cancel",
          "arguments": {}
        },
        "input": {
          "path": "$.input.hotel"
        }
      },
      {
        "id": "car",
        "name": "Book car rental",
        "action": {
          "name": "rental.book",
          "arguments": {}
        },
        "compensate": {
          "name": "rental.cancel",
          "arguments": {}
        },
        "input": {
          "flightArrival": { "path": "$.steps.flight.arrivalTime" },
          "hotelAddress": { "path": "$.steps.hotel.address" }
        }
      }
    ],
    "timeout": "30s",
    "output": {
      "flightConfirmation": { "path": "$.steps.flight.confirmationNumber" },
      "hotelConfirmation": { "path": "$.steps.hotel.confirmationNumber" },
      "carConfirmation": { "path": "$.steps.car.confirmationNumber" }
    }
  }
}
```

## SagaExecutor Implementation

The `SagaExecutor` is responsible for executing sagas:

```rust
pub struct SagaExecutor<T: ToolExecutor> {
    tool_executor: T,
}

impl<T: ToolExecutor> SagaExecutor<T> {
    pub async fn execute(
        &self,
        saga: &Saga,
        input: serde_json::Value,
    ) -> Result<SagaResult, SagaError> {
        // Implementation follows the TDD tests
    }
}

pub struct SagaResult {
    pub output: serde_json::Value,
    pub step_results: HashMap<String, serde_json::Value>,
    pub status: SagaStatus,
}

pub enum SagaStatus {
    Completed,
    Failed { step: String, error: String },
    CompensationFailed { original_error: String, compensation_errors: Vec<String> },
    TimedOut,
}
```

## TDD Test Cases

### Phase 2: Unit Tests

1. **test_saga_happy_path** - All steps succeed
2. **test_saga_compensation_on_failure** - Step 3 fails, compensates 2,1
3. **test_saga_partial_compensation** - Some steps have no compensate
4. **test_saga_with_step_bindings** - Steps reference previous step outputs
5. **test_saga_timeout** - Entire saga times out
6. **test_saga_output_binding** - Custom output construction

### Phase 3: Implementation Order

1. `SagaExecutor::execute()` - Main execution loop
2. Forward execution with step result tracking
3. Compensation on failure (reverse order)
4. Output binding resolution
5. Timeout handling (wrap in tokio::time::timeout)

### Phase 4: Integration Tests

- Travel booking saga with mock tools
- Supply chain saga with mock tools

## Future Extensions

- **Parallel steps**: Allow independent steps to run concurrently
- **Conditional steps**: Skip steps based on conditions
- **Retry policies**: Per-step retry configuration
- **Checkpointing**: Save state for long-running sagas
- **Nested sagas**: Compose sagas from other sagas
