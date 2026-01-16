// Filter pattern types

use serde::{Deserialize, Serialize};

/// FilterSpec filters array elements based on a predicate
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterSpec {
	/// The predicate to evaluate for each element
	pub predicate: FieldPredicate,
}

/// A predicate that compares a field value
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldPredicate {
	/// JSONPath to the field to evaluate
	pub field: String,

	/// Comparison operator
	pub op: String,

	/// Value to compare against
	pub value: PredicateValue,
}

impl FieldPredicate {
	/// Create a new predicate
	pub fn new(field: impl Into<String>, op: impl Into<String>, value: PredicateValue) -> Self {
		Self { field: field.into(), op: op.into(), value }
	}

	/// Create an equality predicate
	pub fn eq(field: impl Into<String>, value: impl Into<PredicateValue>) -> Self {
		Self::new(field, "eq", value.into())
	}

	/// Create a greater-than predicate
	pub fn gt(field: impl Into<String>, value: f64) -> Self {
		Self::new(field, "gt", PredicateValue::Number(value))
	}

	/// Create a less-than predicate
	pub fn lt(field: impl Into<String>, value: f64) -> Self {
		Self::new(field, "lt", PredicateValue::Number(value))
	}

	/// Create a contains predicate (for strings)
	pub fn contains(field: impl Into<String>, value: impl Into<String>) -> Self {
		Self::new(field, "contains", PredicateValue::String(value.into()))
	}
}

/// A value used in predicate comparisons
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PredicateValue {
	/// String value
	StringValue(String),

	/// Numeric value
	NumberValue(f64),

	/// Boolean value
	BoolValue(bool),

	/// Null value
	NullValue(bool),

	/// List of values (for "in" operator)
	ListValue(Vec<PredicateValue>),
}

impl PredicateValue {
	/// Create a string value
	pub fn String(s: impl Into<String>) -> Self {
		PredicateValue::StringValue(s.into())
	}

	/// Create a number value
	pub fn Number(n: f64) -> Self {
		PredicateValue::NumberValue(n)
	}

	/// Create a boolean value
	pub fn Bool(b: bool) -> Self {
		PredicateValue::BoolValue(b)
	}

	/// Create a null value
	pub fn Null() -> Self {
		PredicateValue::NullValue(true)
	}

	/// Create a list value
	pub fn List(values: Vec<PredicateValue>) -> Self {
		PredicateValue::ListValue(values)
	}

	/// Convert to serde_json::Value for comparison
	pub fn to_json_value(&self) -> serde_json::Value {
		match self {
			PredicateValue::StringValue(s) => serde_json::Value::String(s.clone()),
			PredicateValue::NumberValue(n) => serde_json::json!(n),
			PredicateValue::BoolValue(b) => serde_json::Value::Bool(*b),
			PredicateValue::NullValue(_) => serde_json::Value::Null,
			PredicateValue::ListValue(values) => {
				serde_json::Value::Array(values.iter().map(|v| v.to_json_value()).collect())
			},
		}
	}
}

impl From<String> for PredicateValue {
	fn from(s: String) -> Self {
		PredicateValue::StringValue(s)
	}
}

impl From<&str> for PredicateValue {
	fn from(s: &str) -> Self {
		PredicateValue::StringValue(s.to_string())
	}
}

impl From<f64> for PredicateValue {
	fn from(n: f64) -> Self {
		PredicateValue::NumberValue(n)
	}
}

impl From<i64> for PredicateValue {
	fn from(n: i64) -> Self {
		PredicateValue::NumberValue(n as f64)
	}
}

impl From<bool> for PredicateValue {
	fn from(b: bool) -> Self {
		PredicateValue::BoolValue(b)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_filter() {
		let json = r#"{
			"predicate": {
				"field": "$.score",
				"op": "gt",
				"value": { "numberValue": 0.7 }
			}
		}"#;

		let filter: FilterSpec = serde_json::from_str(json).unwrap();
		assert_eq!(filter.predicate.field, "$.score");
		assert_eq!(filter.predicate.op, "gt");
		assert!(matches!(filter.predicate.value, PredicateValue::NumberValue(n) if (n - 0.7).abs() < f64::EPSILON));
	}

	#[test]
	fn test_parse_predicate_string_value() {
		let json = r#"{
			"field": "$.source",
			"op": "eq",
			"value": { "stringValue": "arxiv" }
		}"#;

		let pred: FieldPredicate = serde_json::from_str(json).unwrap();
		assert!(matches!(pred.value, PredicateValue::StringValue(ref s) if s == "arxiv"));
	}

	#[test]
	fn test_parse_predicate_list_value() {
		let json = r#"{
			"field": "$.type",
			"op": "in",
			"value": {
				"listValue": [
					{ "stringValue": "pdf" },
					{ "stringValue": "html" }
				]
			}
		}"#;

		let pred: FieldPredicate = serde_json::from_str(json).unwrap();
		assert!(matches!(pred.value, PredicateValue::ListValue(_)));
	}

	#[test]
	fn test_predicate_builders() {
		let eq = FieldPredicate::eq("$.name", "test");
		assert_eq!(eq.op, "eq");

		let gt = FieldPredicate::gt("$.score", 0.5);
		assert_eq!(gt.op, "gt");

		let contains = FieldPredicate::contains("$.text", "keyword");
		assert_eq!(contains.op, "contains");
	}

	#[test]
	fn test_predicate_value_to_json() {
		assert_eq!(PredicateValue::String("hello").to_json_value(), serde_json::json!("hello"));
		assert_eq!(PredicateValue::Number(42.0).to_json_value(), serde_json::json!(42.0));
		assert_eq!(PredicateValue::Bool(true).to_json_value(), serde_json::json!(true));
		assert_eq!(PredicateValue::Null().to_json_value(), serde_json::Value::Null);
	}
}

