// Filter pattern executor

use serde_json::Value;
use serde_json_path::JsonPath;

use super::ExecutionError;
use crate::mcp::registry::patterns::{FilterSpec, PredicateValue};

/// Executor for filter patterns
pub struct FilterExecutor;

impl FilterExecutor {
	/// Execute a filter pattern
	pub async fn execute(spec: &FilterSpec, input: Value) -> Result<Value, ExecutionError> {
		let arr = input.as_array().ok_or_else(|| ExecutionError::TypeError {
			expected: "array".to_string(),
			actual: Self::value_type_name(&input),
		})?;

		let jsonpath = JsonPath::parse(&spec.predicate.field)
			.map_err(|e| ExecutionError::JsonPathError(format!("{}: {}", spec.predicate.field, e)))?;

		let mut result = Vec::new();

		for item in arr {
			let query_result = jsonpath.query(item);
			let field_value = query_result.iter().next().copied();

			if Self::evaluate_predicate(&spec.predicate.op, field_value, &spec.predicate.value)? {
				result.push(item.clone());
			}
		}

		Ok(Value::Array(result))
	}

	/// Evaluate a predicate
	fn evaluate_predicate(
		op: &str,
		field_value: Option<&Value>,
		predicate_value: &PredicateValue,
	) -> Result<bool, ExecutionError> {
		let target = predicate_value.to_json_value();

		match op {
			"eq" => Ok(
				field_value
					.map(|v| v == &target)
					.unwrap_or(target.is_null()),
			),
			"ne" => Ok(
				field_value
					.map(|v| v != &target)
					.unwrap_or(!target.is_null()),
			),
			"gt" => Self::compare_numeric(field_value, &target, |a, b| a > b),
			"gte" => Self::compare_numeric(field_value, &target, |a, b| a >= b),
			"lt" => Self::compare_numeric(field_value, &target, |a, b| a < b),
			"lte" => Self::compare_numeric(field_value, &target, |a, b| a <= b),
			"contains" => Self::contains(field_value, &target),
			"in" => Self::in_list(field_value, &target),
			other => Err(ExecutionError::PredicateError(format!(
				"unknown operator: {}",
				other
			))),
		}
	}

	/// Numeric comparison
	fn compare_numeric<F>(
		field_value: Option<&Value>,
		target: &Value,
		cmp: F,
	) -> Result<bool, ExecutionError>
	where
		F: Fn(f64, f64) -> bool,
	{
		let field_num = field_value
			.and_then(|v| v.as_f64())
			.ok_or_else(|| ExecutionError::PredicateError("field is not a number".to_string()))?;

		let target_num = target
			.as_f64()
			.ok_or_else(|| ExecutionError::PredicateError("target is not a number".to_string()))?;

		Ok(cmp(field_num, target_num))
	}

	/// String contains check
	fn contains(field_value: Option<&Value>, target: &Value) -> Result<bool, ExecutionError> {
		let field_str = field_value
			.and_then(|v| v.as_str())
			.ok_or_else(|| ExecutionError::PredicateError("field is not a string".to_string()))?;

		let target_str = target
			.as_str()
			.ok_or_else(|| ExecutionError::PredicateError("target is not a string".to_string()))?;

		Ok(field_str.contains(target_str))
	}

	/// Check if value is in list
	fn in_list(field_value: Option<&Value>, target: &Value) -> Result<bool, ExecutionError> {
		let list = target
			.as_array()
			.ok_or_else(|| ExecutionError::PredicateError("target is not an array".to_string()))?;

		let field_val =
			field_value.ok_or_else(|| ExecutionError::PredicateError("field is null".to_string()))?;

		Ok(list.iter().any(|item| item == field_val))
	}

	fn value_type_name(value: &Value) -> String {
		match value {
			Value::Null => "null",
			Value::Bool(_) => "boolean",
			Value::Number(_) => "number",
			Value::String(_) => "string",
			Value::Array(_) => "array",
			Value::Object(_) => "object",
		}
		.to_string()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mcp::registry::patterns::FieldPredicate;
	use serde_json::json;

	#[tokio::test]
	async fn test_filter_eq() {
		let spec = FilterSpec {
			predicate: FieldPredicate {
				field: "$.type".to_string(),
				op: "eq".to_string(),
				value: PredicateValue::StringValue("pdf".to_string()),
			},
		};

		let input = json!([
			{"type": "pdf", "name": "doc1"},
			{"type": "html", "name": "doc2"},
			{"type": "pdf", "name": "doc3"}
		]);

		let result = FilterExecutor::execute(&spec, input).await.unwrap();
		let arr = result.as_array().unwrap();

		assert_eq!(arr.len(), 2);
		assert_eq!(arr[0]["name"], "doc1");
		assert_eq!(arr[1]["name"], "doc3");
	}

	#[tokio::test]
	async fn test_filter_gt() {
		let spec = FilterSpec {
			predicate: FieldPredicate {
				field: "$.score".to_string(),
				op: "gt".to_string(),
				value: PredicateValue::NumberValue(0.5),
			},
		};

		let input = json!([
			{"score": 0.3, "name": "low"},
			{"score": 0.7, "name": "high"},
			{"score": 0.5, "name": "exact"}
		]);

		let result = FilterExecutor::execute(&spec, input).await.unwrap();
		let arr = result.as_array().unwrap();

		assert_eq!(arr.len(), 1);
		assert_eq!(arr[0]["name"], "high");
	}

	#[tokio::test]
	async fn test_filter_gte() {
		let spec = FilterSpec {
			predicate: FieldPredicate {
				field: "$.score".to_string(),
				op: "gte".to_string(),
				value: PredicateValue::NumberValue(0.5),
			},
		};

		let input = json!([
			{"score": 0.3, "name": "low"},
			{"score": 0.7, "name": "high"},
			{"score": 0.5, "name": "exact"}
		]);

		let result = FilterExecutor::execute(&spec, input).await.unwrap();
		let arr = result.as_array().unwrap();

		assert_eq!(arr.len(), 2);
	}

	#[tokio::test]
	async fn test_filter_contains() {
		let spec = FilterSpec {
			predicate: FieldPredicate {
				field: "$.title".to_string(),
				op: "contains".to_string(),
				value: PredicateValue::StringValue("AI".to_string()),
			},
		};

		let input = json!([
			{"title": "Introduction to AI"},
			{"title": "Machine Learning Basics"},
			{"title": "AI in Healthcare"}
		]);

		let result = FilterExecutor::execute(&spec, input).await.unwrap();
		let arr = result.as_array().unwrap();

		assert_eq!(arr.len(), 2);
	}

	#[tokio::test]
	async fn test_filter_in() {
		let spec = FilterSpec {
			predicate: FieldPredicate {
				field: "$.status".to_string(),
				op: "in".to_string(),
				value: PredicateValue::ListValue(vec![
					PredicateValue::StringValue("active".to_string()),
					PredicateValue::StringValue("pending".to_string()),
				]),
			},
		};

		let input = json!([
			{"status": "active", "id": 1},
			{"status": "closed", "id": 2},
			{"status": "pending", "id": 3}
		]);

		let result = FilterExecutor::execute(&spec, input).await.unwrap();
		let arr = result.as_array().unwrap();

		assert_eq!(arr.len(), 2);
		assert_eq!(arr[0]["id"], 1);
		assert_eq!(arr[1]["id"], 3);
	}

	#[tokio::test]
	async fn test_filter_ne() {
		let spec = FilterSpec {
			predicate: FieldPredicate {
				field: "$.active".to_string(),
				op: "ne".to_string(),
				value: PredicateValue::BoolValue(false),
			},
		};

		let input = json!([
			{"active": true, "id": 1},
			{"active": false, "id": 2},
			{"active": true, "id": 3}
		]);

		let result = FilterExecutor::execute(&spec, input).await.unwrap();
		let arr = result.as_array().unwrap();

		assert_eq!(arr.len(), 2);
	}

	#[tokio::test]
	async fn test_filter_non_array_error() {
		let spec = FilterSpec {
			predicate: FieldPredicate {
				field: "$.x".to_string(),
				op: "eq".to_string(),
				value: PredicateValue::NumberValue(1.0),
			},
		};

		let input = json!({"not": "an array"});
		let result = FilterExecutor::execute(&spec, input).await;

		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			ExecutionError::TypeError { .. }
		));
	}
}
