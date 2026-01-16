// Schema Map pattern types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// SchemaMapSpec transforms input to output using field mappings
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaMapSpec {
	/// Field name -> source mapping
	pub mappings: HashMap<String, FieldSource>,
}

impl SchemaMapSpec {
	/// Create a new schema map with the given mappings
	pub fn new(mappings: HashMap<String, FieldSource>) -> Self {
		Self { mappings }
	}

	/// Create an empty schema map
	pub fn empty() -> Self {
		Self { mappings: HashMap::new() }
	}

	/// Add a path mapping
	pub fn with_path(mut self, field: impl Into<String>, path: impl Into<String>) -> Self {
		self.mappings.insert(field.into(), FieldSource::Path(path.into()));
		self
	}

	/// Add a literal value mapping
	pub fn with_literal(mut self, field: impl Into<String>, value: LiteralValue) -> Self {
		self.mappings.insert(field.into(), FieldSource::Literal(value));
		self
	}
}

/// FieldSource defines where a field value comes from
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldSource {
	/// JSONPath extraction from input
	Path(String),

	/// Constant value
	Literal(LiteralValue),

	/// First non-null from multiple paths
	Coalesce(CoalesceSource),

	/// String template with variable substitution
	Template(TemplateSource),

	/// Concatenate multiple fields
	Concat(ConcatSource),

	/// Nested object mapping
	Nested(Box<SchemaMapSpec>),
}

impl FieldSource {
	/// Create a path source
	pub fn path(p: impl Into<String>) -> Self {
		FieldSource::Path(p.into())
	}

	/// Create a string literal source
	pub fn string(s: impl Into<String>) -> Self {
		FieldSource::Literal(LiteralValue::StringValue(s.into()))
	}

	/// Create a number literal source
	pub fn number(n: f64) -> Self {
		FieldSource::Literal(LiteralValue::NumberValue(n))
	}

	/// Create a boolean literal source
	pub fn bool(b: bool) -> Self {
		FieldSource::Literal(LiteralValue::BoolValue(b))
	}

	/// Create a null literal source
	pub fn null() -> Self {
		FieldSource::Literal(LiteralValue::NullValue(true))
	}

	/// Create a coalesce source
	pub fn coalesce(paths: Vec<String>) -> Self {
		FieldSource::Coalesce(CoalesceSource { paths })
	}
}

/// Literal value in a schema mapping
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LiteralValue {
	/// String constant
	StringValue(String),

	/// Numeric constant
	NumberValue(f64),

	/// Boolean constant
	BoolValue(bool),

	/// Null value (true = null)
	NullValue(bool),
}

impl LiteralValue {
	/// Convert to serde_json::Value
	pub fn to_json_value(&self) -> serde_json::Value {
		match self {
			LiteralValue::StringValue(s) => serde_json::Value::String(s.clone()),
			LiteralValue::NumberValue(n) => serde_json::json!(n),
			LiteralValue::BoolValue(b) => serde_json::Value::Bool(*b),
			LiteralValue::NullValue(_) => serde_json::Value::Null,
		}
	}
}

/// Coalesce source - returns first non-null value from paths
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoalesceSource {
	/// JSONPaths to try in order
	pub paths: Vec<String>,
}

/// Template source - string interpolation
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSource {
	/// Template string with {var} placeholders
	pub template: String,

	/// Variable name -> JSONPath binding
	pub vars: HashMap<String, String>,
}

/// Concat source - concatenate multiple fields
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConcatSource {
	/// JSONPaths to concatenate
	pub paths: Vec<String>,

	/// Separator between values
	#[serde(default)]
	pub separator: Option<String>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_schema_map() {
		let json = r#"{
			"mappings": {
				"title": { "path": "$.paper_title" },
				"url": { "coalesce": { "paths": ["$.pdf_url", "$.arxiv_id"] } },
				"source": { "literal": { "stringValue": "arxiv" } },
				"relevance": { "literal": { "numberValue": 0.85 } }
			}
		}"#;

		let schema_map: SchemaMapSpec = serde_json::from_str(json).unwrap();
		assert_eq!(schema_map.mappings.len(), 4);
		assert!(matches!(schema_map.mappings.get("title"), Some(FieldSource::Path(_))));
		assert!(matches!(schema_map.mappings.get("url"), Some(FieldSource::Coalesce(_))));
		assert!(matches!(schema_map.mappings.get("source"), Some(FieldSource::Literal(_))));
	}

	#[test]
	fn test_parse_field_source_path() {
		let json = r#"{ "path": "$.data.field" }"#;
		let source: FieldSource = serde_json::from_str(json).unwrap();
		assert!(matches!(source, FieldSource::Path(ref p) if p == "$.data.field"));
	}

	#[test]
	fn test_parse_field_source_literal() {
		let json = r#"{ "literal": { "stringValue": "constant" } }"#;
		let source: FieldSource = serde_json::from_str(json).unwrap();
		assert!(matches!(source, FieldSource::Literal(LiteralValue::StringValue(ref s)) if s == "constant"));
	}

	#[test]
	fn test_parse_field_source_coalesce() {
		let json = r#"{ "coalesce": { "paths": ["$.primary", "$.fallback"] } }"#;
		let source: FieldSource = serde_json::from_str(json).unwrap();
		if let FieldSource::Coalesce(c) = source {
			assert_eq!(c.paths.len(), 2);
		} else {
			panic!("Expected Coalesce");
		}
	}

	#[test]
	fn test_parse_field_source_template() {
		let json = r#"{
			"template": {
				"template": "{source}:{id}",
				"vars": {
					"source": "$.source_name",
					"id": "$.item_id"
				}
			}
		}"#;

		let source: FieldSource = serde_json::from_str(json).unwrap();
		if let FieldSource::Template(t) = source {
			assert_eq!(t.template, "{source}:{id}");
			assert_eq!(t.vars.len(), 2);
		} else {
			panic!("Expected Template");
		}
	}

	#[test]
	fn test_parse_field_source_concat() {
		let json = r#"{
			"concat": {
				"paths": ["$.first", "$.last"],
				"separator": " "
			}
		}"#;

		let source: FieldSource = serde_json::from_str(json).unwrap();
		if let FieldSource::Concat(c) = source {
			assert_eq!(c.paths.len(), 2);
			assert_eq!(c.separator, Some(" ".to_string()));
		} else {
			panic!("Expected Concat");
		}
	}

	#[test]
	fn test_parse_field_source_nested() {
		let json = r#"{
			"nested": {
				"mappings": {
					"name": { "path": "$.inner.name" }
				}
			}
		}"#;

		let source: FieldSource = serde_json::from_str(json).unwrap();
		assert!(matches!(source, FieldSource::Nested(_)));
	}

	#[test]
	fn test_builder_pattern() {
		let schema = SchemaMapSpec::empty()
			.with_path("title", "$.name")
			.with_literal("type", LiteralValue::StringValue("doc".to_string()));

		assert_eq!(schema.mappings.len(), 2);
	}

	#[test]
	fn test_literal_to_json() {
		assert_eq!(LiteralValue::StringValue("test".to_string()).to_json_value(), serde_json::json!("test"));
		assert_eq!(LiteralValue::NumberValue(42.0).to_json_value(), serde_json::json!(42.0));
		assert_eq!(LiteralValue::BoolValue(true).to_json_value(), serde_json::json!(true));
		assert_eq!(LiteralValue::NullValue(true).to_json_value(), serde_json::Value::Null);
	}
}

