//! Saga pattern intermediate representation types.
//!
//! These types define the structure of a saga - a distributed transaction
//! composed of steps with compensating actions.

use std::collections::HashMap;
use std::time::Duration;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::serde_dur_option;

/// A saga defines a distributed transaction as a sequence of steps.
///
/// Each step has an action and an optional compensating action. If a step
/// fails, previously completed steps are compensated in reverse order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Saga {
    /// Unique identifier for this saga definition
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Human-readable name for logging and debugging
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Ordered list of steps to execute
    pub steps: Vec<SagaStep>,

    /// How to construct the final output from step results
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputBinding>,

    /// Overall timeout for the entire saga execution
    #[serde(default, with = "serde_dur_option")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub timeout: Option<Duration>,
}

/// A single step in a saga.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SagaStep {
    /// Unique identifier for this step (used in bindings to reference outputs)
    pub id: String,

    /// Human-readable name for logging
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The action to execute (forward direction)
    pub action: StepAction,

    /// The compensating action (executed on failure, in reverse order)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compensate: Option<StepAction>,

    /// How to construct input for this step from saga input and prior results
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<InputBinding>,

    /// Per-step timeout (overrides saga timeout for this step)
    #[serde(default, with = "serde_dur_option")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub timeout: Option<Duration>,
}

/// An action that can be executed as part of a saga step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum StepAction {
    /// Execute an MCP tool by name
    Tool {
        /// The tool name (may include target prefix for multiplexed backends)
        name: String,
    },

    /// Call an HTTP endpoint
    Http {
        /// HTTP method (GET, POST, etc.)
        method: String,
        /// URL to call
        url: String,
    },

    /// Reference a backend by name
    Backend {
        /// Backend name as configured in the gateway
        name: String,
    },
}

/// Binding to construct step input from saga input and previous step results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum InputBinding {
    /// Extract from the original saga input using JSONPath
    Input {
        /// JSONPath expression (e.g., "$.flight" or "$.user.id")
        path: String,
    },

    /// Reference a previous step's output
    Step {
        /// The step id to reference
        id: String,
        /// Optional JSONPath into that step's output
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },

    /// Merge multiple bindings into a single object
    Merge(Vec<InputBinding>),

    /// A static/literal value
    Static(serde_json::Value),
}

/// Binding to construct saga output from step results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum OutputBinding {
    /// Return a specific step's result
    Step {
        /// The step id
        id: String,
        /// Optional JSONPath into that step's output
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },

    /// Combine results into a custom object
    Object(HashMap<String, OutputBinding>),

    /// Return all step results keyed by step id
    All,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saga_deserialize_basic() {
        let json = r#"{
            "name": "Travel Booking",
            "steps": [
                {
                    "id": "flight",
                    "name": "Book flight",
                    "action": { "tool": { "name": "airline.book" } }
                }
            ]
        }"#;

        let saga: Saga = serde_json::from_str(json).unwrap();
        assert_eq!(saga.name, Some("Travel Booking".to_string()));
        assert_eq!(saga.steps.len(), 1);
        assert_eq!(saga.steps[0].id, "flight");
    }

    #[test]
    fn test_saga_deserialize_with_compensation() {
        let json = r#"{
            "steps": [
                {
                    "id": "flight",
                    "action": { "tool": { "name": "airline.book" } },
                    "compensate": { "tool": { "name": "airline.cancel" } },
                    "input": { "input": { "path": "$.flight" } }
                },
                {
                    "id": "hotel",
                    "action": { "tool": { "name": "hotel.reserve" } },
                    "compensate": { "tool": { "name": "hotel.cancel" } },
                    "input": { "input": { "path": "$.hotel" } }
                }
            ],
            "timeout": "30s"
        }"#;

        let saga: Saga = serde_json::from_str(json).unwrap();
        assert_eq!(saga.steps.len(), 2);
        assert!(saga.steps[0].compensate.is_some());
        assert!(saga.steps[1].compensate.is_some());
        assert_eq!(saga.timeout, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_saga_deserialize_with_step_binding() {
        let json = r#"{
            "steps": [
                {
                    "id": "step1",
                    "action": { "backend": { "name": "api-1" } }
                },
                {
                    "id": "step2",
                    "action": { "backend": { "name": "api-2" } },
                    "input": { "step": { "id": "step1", "path": "$.result" } }
                }
            ]
        }"#;

        let saga: Saga = serde_json::from_str(json).unwrap();
        assert_eq!(saga.steps.len(), 2);

        if let Some(InputBinding::Step { id, path }) = &saga.steps[1].input {
            assert_eq!(id, "step1");
            assert_eq!(path.as_deref(), Some("$.result"));
        } else {
            panic!("Expected Step binding");
        }
    }

    #[test]
    fn test_saga_deserialize_with_output_binding() {
        let json = r#"{
            "steps": [
                { "id": "a", "action": { "tool": { "name": "tool1" } } },
                { "id": "b", "action": { "tool": { "name": "tool2" } } }
            ],
            "output": {
                "object": {
                    "first": { "step": { "id": "a" } },
                    "second": { "step": { "id": "b", "path": "$.value" } }
                }
            }
        }"#;

        let saga: Saga = serde_json::from_str(json).unwrap();

        if let Some(OutputBinding::Object(obj)) = &saga.output {
            assert!(obj.contains_key("first"));
            assert!(obj.contains_key("second"));
        } else {
            panic!("Expected Object output binding");
        }
    }

    #[test]
    fn test_saga_serialize_roundtrip() {
        let saga = Saga {
            id: Some("test-saga".to_string()),
            name: Some("Test Saga".to_string()),
            steps: vec![
                SagaStep {
                    id: "step1".to_string(),
                    name: Some("First Step".to_string()),
                    action: StepAction::Tool {
                        name: "my.tool".to_string(),
                    },
                    compensate: Some(StepAction::Tool {
                        name: "my.undo".to_string(),
                    }),
                    input: Some(InputBinding::Input {
                        path: "$.data".to_string(),
                    }),
                    timeout: Some(Duration::from_secs(10)),
                },
            ],
            output: Some(OutputBinding::All),
            timeout: Some(Duration::from_secs(60)),
        };

        let json = serde_json::to_string(&saga).unwrap();
        let deserialized: Saga = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, saga.id);
        assert_eq!(deserialized.name, saga.name);
        assert_eq!(deserialized.steps.len(), 1);
        assert_eq!(deserialized.steps[0].id, "step1");
    }

    #[test]
    fn test_http_action() {
        let json = r#"{
            "steps": [
                {
                    "id": "call",
                    "action": {
                        "http": {
                            "method": "POST",
                            "url": "https://api.example.com/action"
                        }
                    }
                }
            ]
        }"#;

        let saga: Saga = serde_json::from_str(json).unwrap();
        if let StepAction::Http { method, url } = &saga.steps[0].action {
            assert_eq!(method, "POST");
            assert_eq!(url, "https://api.example.com/action");
        } else {
            panic!("Expected Http action");
        }
    }

    #[test]
    fn test_merge_binding() {
        let json = r#"{
            "steps": [
                { "id": "a", "action": { "tool": { "name": "t1" } } },
                { "id": "b", "action": { "tool": { "name": "t2" } } },
                {
                    "id": "c",
                    "action": { "tool": { "name": "t3" } },
                    "input": {
                        "merge": [
                            { "step": { "id": "a" } },
                            { "step": { "id": "b" } },
                            { "static": { "extra": "value" } }
                        ]
                    }
                }
            ]
        }"#;

        let saga: Saga = serde_json::from_str(json).unwrap();
        if let Some(InputBinding::Merge(bindings)) = &saga.steps[2].input {
            assert_eq!(bindings.len(), 3);
        } else {
            panic!("Expected Merge binding");
        }
    }
}
