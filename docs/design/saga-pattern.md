# Saga Pattern Design

## Overview

The Saga pattern provides distributed transaction management through a sequence of steps with compensating actions. When a step fails, previously completed steps are compensated in reverse order to maintain data consistency.

## Use Cases

### Travel Booking
```
1. Book flight → Cancel flight
2. Book hotel → Cancel hotel
3. Book car → Cancel car
```

### Supply Chain Order
```
1. Reserve inventory → Release inventory
2. Process payment → Refund payment
3. Schedule shipping → Cancel shipment
4. Send notification (no compensation)
```

## Rust IR Types

Located in `crates/agentgateway/src/saga/types.rs`:

```rust
/// A saga defines a distributed transaction as a sequence of steps
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Saga {
    /// Unique identifier for this saga definition
    pub id: Option<String>,
    /// Human-readable name
    pub name: Option<String>,
    /// Ordered list of steps to execute
    pub steps: Vec<SagaStep>,
    /// How to construct the final output from step results
    #[serde(default)]
    pub output: Option<OutputBinding>,
    /// Overall timeout for the entire saga execution
    #[serde(default)]
    pub timeout: Option<Duration>,
}

/// A single step in a saga
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SagaStep {
    /// Unique identifier for this step (used in bindings)
    pub id: String,
    /// Human-readable name for logging
    pub name: Option<String>,
    /// The action to execute (forward direction)
    pub action: StepAction,
    /// The compensating action (reverse direction on failure)
    #[serde(default)]
    pub compensate: Option<StepAction>,
    /// How to construct input for this step
    #[serde(default)]
    pub input: Option<InputBinding>,
    /// Per-step timeout (overrides saga timeout for this step)
    #[serde(default)]
    pub timeout: Option<Duration>,
}

/// An action that can be executed (either forward or compensation)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum StepAction {
    /// Execute an MCP tool
    Tool { name: String },
    /// Call an HTTP endpoint
    Http {
        method: String,
        url: String,
    },
    /// Reference a backend by name
    Backend { name: String },
}

/// Binding to construct step input from saga input and previous results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum InputBinding {
    /// JSONPath into the original saga input
    Input { path: String },
    /// Reference a previous step's output
    Step { id: String, path: Option<String> },
    /// Merge multiple sources
    Merge(Vec<InputBinding>),
    /// Static value
    Static(serde_json::Value),
}

/// Binding to construct saga output from step results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum OutputBinding {
    /// Return a specific step's result
    Step { id: String, path: Option<String> },
    /// Combine results into an object
    Object(HashMap<String, OutputBinding>),
    /// Return all step results keyed by step id
    All,
}
```

## Execution State

```rust
/// Runtime state during saga execution
#[derive(Debug, Clone)]
pub struct SagaExecution {
    /// The saga definition
    pub saga: Saga,
    /// Original input to the saga
    pub input: serde_json::Value,
    /// Results from completed steps (keyed by step id)
    pub step_results: HashMap<String, StepResult>,
    /// Current execution status
    pub status: SagaStatus,
    /// Index of current step being executed
    pub current_step: usize,
    /// When execution started
    pub started_at: Instant,
}

#[derive(Debug, Clone)]
pub enum SagaStatus {
    /// Executing forward steps
    Executing,
    /// A step failed, running compensation
    Compensating { failed_step: usize, error: String },
    /// All steps completed successfully
    Completed,
    /// Compensation completed after failure
    CompensatedFailure { original_error: String },
    /// Saga timed out
    TimedOut,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_id: String,
    pub output: serde_json::Value,
    pub duration: Duration,
}
```

## Executor Interface

```rust
pub struct SagaExecutor {
    /// Tool/backend router for executing actions
    router: Arc<dyn ActionRouter>,
}

#[async_trait]
pub trait ActionRouter: Send + Sync {
    async fn execute_action(
        &self,
        action: &StepAction,
        input: serde_json::Value,
        timeout: Option<Duration>,
    ) -> Result<serde_json::Value, SagaError>;
}

impl SagaExecutor {
    /// Execute a saga to completion
    pub async fn execute(
        &self,
        saga: Saga,
        input: serde_json::Value,
    ) -> Result<SagaResult, SagaError>;
}

#[derive(Debug)]
pub struct SagaResult {
    pub output: serde_json::Value,
    pub step_results: HashMap<String, StepResult>,
    pub duration: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum SagaError {
    #[error("Step '{step_id}' failed: {message}")]
    StepFailed { step_id: String, message: String },

    #[error("Compensation failed for step '{step_id}': {message}")]
    CompensationFailed { step_id: String, message: String },

    #[error("Saga timed out after {duration:?}")]
    Timeout { duration: Duration },

    #[error("Invalid binding: {0}")]
    InvalidBinding(String),

    #[error("Step '{step_id}' not found")]
    StepNotFound { step_id: String },
}
```

## Execution Algorithm

### Forward Execution

```
for step in saga.steps:
    input = resolve_input_binding(step.input, saga_input, step_results)

    result = timeout(step.timeout.or(saga.timeout)):
        router.execute_action(step.action, input)

    if result.is_err():
        return compensate(step_results, result.err())

    step_results[step.id] = result

return construct_output(saga.output, step_results)
```

### Compensation

```
fn compensate(completed_steps, original_error):
    for step in completed_steps.reverse():
        if step.compensate.is_some():
            comp_input = step_results[step.id]
            router.execute_action(step.compensate, comp_input)
            // Log but don't fail if compensation fails

    return SagaError::CompensatedFailure(original_error)
```

## JSON Schema Example

### Travel Booking Saga

```json
{
  "saga": {
    "name": "Travel Booking",
    "steps": [
      {
        "id": "flight",
        "name": "Book flight",
        "action": { "tool": { "name": "airline.book" } },
        "compensate": { "tool": { "name": "airline.cancel" } },
        "input": { "input": { "path": "$.flight" } }
      },
      {
        "id": "hotel",
        "name": "Book hotel",
        "action": { "tool": { "name": "hotel.reserve" } },
        "compensate": { "tool": { "name": "hotel.cancel" } },
        "input": { "input": { "path": "$.hotel" } }
      },
      {
        "id": "car",
        "name": "Rent car",
        "action": { "tool": { "name": "rental.book" } },
        "compensate": { "tool": { "name": "rental.cancel" } },
        "input": { "input": { "path": "$.car" } }
      },
      {
        "id": "confirmation",
        "name": "Send confirmation",
        "action": { "tool": { "name": "email.send" } },
        "input": {
          "merge": [
            { "step": { "id": "flight", "path": "$.confirmation_number" } },
            { "step": { "id": "hotel", "path": "$.reservation_id" } },
            { "step": { "id": "car", "path": "$.booking_ref" } }
          ]
        }
      }
    ],
    "output": {
      "object": {
        "flight": { "step": { "id": "flight" } },
        "hotel": { "step": { "id": "hotel" } },
        "car": { "step": { "id": "car" } }
      }
    },
    "timeout": "30s"
  }
}
```

### Step Binding Example

Input saga:
```json
{
  "flight": { "from": "SFO", "to": "JFK", "date": "2024-03-15" },
  "hotel": { "city": "NYC", "checkin": "2024-03-15", "nights": 3 }
}
```

Step 1 (flight) input binding `{ "input": { "path": "$.flight" } }` resolves to:
```json
{ "from": "SFO", "to": "JFK", "date": "2024-03-15" }
```

Step 1 result:
```json
{ "confirmation_number": "AA123", "price": 450.00 }
```

Step 2 (hotel) with binding `{ "step": { "id": "flight", "path": "$.confirmation_number" } }` would receive:
```json
"AA123"
```

## Integration with AgentGateway

### As a Backend Type

Add `Saga` variant to the `Backend` enum:

```rust
pub enum Backend {
    Service(Arc<Service>, u16),
    Opaque(ResourceName, Target),
    MCP(ResourceName, McpBackend),
    AI(ResourceName, AIBackend),
    Dynamic(ResourceName, ()),
    Saga(ResourceName, SagaBackend),  // New
    Invalid,
}
```

### As a Policy

Sagas can also be applied as request transformation policies that orchestrate multiple backend calls.

### Configuration

```yaml
backends:
  - name: travel-booking
    saga:
      steps:
        - id: flight
          action:
            backend: airline-api
          compensate:
            backend: airline-api-cancel
          input:
            input:
              path: "$.flight"
        - id: hotel
          action:
            backend: hotel-api
          input:
            input:
              path: "$.hotel"
```

## Test Plan

### Unit Tests

1. `test_saga_happy_path` - All steps succeed, verify output binding
2. `test_saga_compensation_on_failure` - Step 3 fails, verify steps 2,1 compensated
3. `test_saga_partial_compensation` - Some steps have no compensate action
4. `test_saga_with_step_bindings` - Steps reference previous step outputs
5. `test_saga_timeout` - Entire saga times out
6. `test_saga_step_timeout` - Individual step times out
7. `test_saga_output_binding` - Custom output construction

### Integration Tests

1. Travel booking saga with mock MCP tools
2. Supply chain saga with mock HTTP backends
