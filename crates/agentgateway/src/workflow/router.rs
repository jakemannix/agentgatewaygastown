//! ContentRouter pattern implementation.
//!
//! Routes input to different operations based on matching predicates.

use serde_json::Value;

use super::types::{
    ExecutionError, ExecutionResult, FieldPredicate, PredicateOperator, RouterSpec,
    StepOperation,
};

/// Evaluates a field predicate against input data.
pub fn evaluate_predicate(predicate: &FieldPredicate, input: &Value) -> Result<bool, ExecutionError> {
    // Get the field value from the input
    let field_value = get_field_value(input, &predicate.field)?;

    // Evaluate the predicate based on operator
    match predicate.operator {
        PredicateOperator::Eq => Ok(values_equal(&field_value, &predicate.value)),
        PredicateOperator::Neq => Ok(!values_equal(&field_value, &predicate.value)),
        PredicateOperator::Gt => compare_values(&field_value, &predicate.value, |a, b| a > b),
        PredicateOperator::Gte => compare_values(&field_value, &predicate.value, |a, b| a >= b),
        PredicateOperator::Lt => compare_values(&field_value, &predicate.value, |a, b| a < b),
        PredicateOperator::Lte => compare_values(&field_value, &predicate.value, |a, b| a <= b),
        PredicateOperator::Contains => evaluate_contains(&field_value, &predicate.value),
        PredicateOperator::StartsWith => evaluate_starts_with(&field_value, &predicate.value),
        PredicateOperator::EndsWith => evaluate_ends_with(&field_value, &predicate.value),
        PredicateOperator::Matches => evaluate_matches(&field_value, &predicate.value),
        PredicateOperator::Exists => Ok(!field_value.is_null()),
        PredicateOperator::In => evaluate_in(&field_value, &predicate.value),
    }
}

/// Gets a field value from a JSON object using dot notation.
fn get_field_value(input: &Value, path: &str) -> Result<Value, ExecutionError> {
    let mut current = input.clone();

    for part in path.split('.') {
        current = match current {
            Value::Object(map) => map
                .get(part)
                .cloned()
                .unwrap_or(Value::Null),
            Value::Array(arr) => {
                if let Ok(index) = part.parse::<usize>() {
                    arr.get(index).cloned().unwrap_or(Value::Null)
                } else {
                    return Err(ExecutionError::InvalidFieldPath(format!(
                        "cannot index array with non-numeric key: {}",
                        part
                    )));
                }
            }
            _ => Value::Null,
        };
    }

    Ok(current)
}

/// Checks if two JSON values are equal.
fn values_equal(a: &Value, b: &Value) -> bool {
    a == b
}

/// Compares two numeric values using a comparison function.
fn compare_values<F>(a: &Value, b: &Value, cmp: F) -> Result<bool, ExecutionError>
where
    F: Fn(f64, f64) -> bool,
{
    let a_num = value_to_f64(a)?;
    let b_num = value_to_f64(b)?;
    Ok(cmp(a_num, b_num))
}

/// Converts a JSON value to f64 for numeric comparison.
fn value_to_f64(v: &Value) -> Result<f64, ExecutionError> {
    match v {
        Value::Number(n) => n.as_f64().ok_or_else(|| ExecutionError::TypeMismatch {
            expected: "f64".to_string(),
            actual: "number out of range".to_string(),
        }),
        _ => Err(ExecutionError::TypeMismatch {
            expected: "number".to_string(),
            actual: format!("{:?}", v),
        }),
    }
}

/// Evaluates the 'contains' operator.
fn evaluate_contains(haystack: &Value, needle: &Value) -> Result<bool, ExecutionError> {
    match (haystack, needle) {
        (Value::String(s), Value::String(substr)) => Ok(s.contains(substr.as_str())),
        (Value::Array(arr), val) => Ok(arr.contains(val)),
        _ => Err(ExecutionError::TypeMismatch {
            expected: "string or array".to_string(),
            actual: format!("{:?}", haystack),
        }),
    }
}

/// Evaluates the 'starts_with' operator.
fn evaluate_starts_with(value: &Value, prefix: &Value) -> Result<bool, ExecutionError> {
    match (value, prefix) {
        (Value::String(s), Value::String(p)) => Ok(s.starts_with(p.as_str())),
        _ => Err(ExecutionError::TypeMismatch {
            expected: "string".to_string(),
            actual: format!("{:?}", value),
        }),
    }
}

/// Evaluates the 'ends_with' operator.
fn evaluate_ends_with(value: &Value, suffix: &Value) -> Result<bool, ExecutionError> {
    match (value, suffix) {
        (Value::String(s), Value::String(p)) => Ok(s.ends_with(p.as_str())),
        _ => Err(ExecutionError::TypeMismatch {
            expected: "string".to_string(),
            actual: format!("{:?}", value),
        }),
    }
}

/// Evaluates the 'matches' operator using regex.
fn evaluate_matches(value: &Value, pattern: &Value) -> Result<bool, ExecutionError> {
    match (value, pattern) {
        (Value::String(s), Value::String(p)) => {
            let re = regex::Regex::new(p).map_err(|e| {
                ExecutionError::PredicateError(format!("invalid regex: {}", e))
            })?;
            Ok(re.is_match(s))
        }
        _ => Err(ExecutionError::TypeMismatch {
            expected: "string".to_string(),
            actual: format!("{:?}", value),
        }),
    }
}

/// Evaluates the 'in' operator.
fn evaluate_in(value: &Value, list: &Value) -> Result<bool, ExecutionError> {
    match list {
        Value::Array(arr) => Ok(arr.contains(value)),
        _ => Err(ExecutionError::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{:?}", list),
        }),
    }
}

/// Executor for the ContentRouter pattern.
pub struct RouterExecutor;

impl RouterExecutor {
    /// Executes the router, returning the result of the matched route.
    pub fn execute(spec: &RouterSpec, input: Value) -> ExecutionResult {
        // Evaluate each route in order
        for route in &spec.routes {
            if evaluate_predicate(&route.when, &input)? {
                return Self::execute_operation(&route.then, input);
            }
        }

        // No route matched - check for otherwise
        if let Some(ref otherwise) = spec.otherwise {
            Self::execute_operation(otherwise, input)
        } else {
            Err(ExecutionError::NoRouteMatch)
        }
    }

    /// Executes a single step operation.
    pub fn execute_operation(op: &StepOperation, input: Value) -> ExecutionResult {
        match op {
            StepOperation::Passthrough => Ok(input),
            StepOperation::Transform { expression: _ } => {
                // TODO: Implement CEL/template transformation
                Ok(input)
            }
            StepOperation::ToolCall { tool: _, args: _ } => {
                // TODO: Implement tool invocation
                Ok(input)
            }
            StepOperation::Router(router_spec) => Self::execute(router_spec, input),
            StepOperation::Sequence { steps } => {
                let mut current = input;
                for step in steps {
                    current = Self::execute_operation(step, current)?;
                }
                Ok(current)
            }
            StepOperation::Parallel { branches: _ } => {
                // TODO: Implement parallel execution
                Ok(input)
            }
        }
    }
}
