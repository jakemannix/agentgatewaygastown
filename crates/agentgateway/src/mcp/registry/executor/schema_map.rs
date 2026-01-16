// Schema Map pattern executor

use std::collections::HashMap;

use serde_json::Value;
use serde_json_path::JsonPath;

use super::ExecutionError;
use crate::mcp::registry::patterns::{FieldSource, SchemaMapSpec};

/// Executor for schema-map patterns
pub struct SchemaMapExecutor;

impl SchemaMapExecutor {
	/// Execute a schema-map pattern
	pub async fn execute(spec: &SchemaMapSpec, input: Value) -> Result<Value, ExecutionError> {
		let mut result = serde_json::Map::new();

		for (field_name, source) in &spec.mappings {
			let value = Self::extract_field_source(source, &input)?;
			result.insert(field_name.clone(), value);
		}

		Ok(Value::Object(result))
	}

	/// Extract a value from a field source
	fn extract_field_source(source: &FieldSource, input: &Value) -> Result<Value, ExecutionError> {
		match source {
			FieldSource::Path(path) => Self::extract_path(path, input),
			FieldSource::Literal(lit) => Ok(lit.to_json_value()),
			FieldSource::Coalesce(c) => Self::coalesce(&c.paths, input),
			FieldSource::Template(t) => Self::template(&t.template, &t.vars, input),
			FieldSource::Concat(c) => Self::concat(&c.paths, c.separator.as_deref(), input),
			FieldSource::Nested(nested) => {
				let nested_spec = SchemaMapSpec { mappings: nested.mappings.clone() };
				Box::pin(Self::execute(&nested_spec, input.clone())).now_or_never().unwrap()
			},
		}
	}

	/// Extract value using JSONPath
	fn extract_path(path: &str, input: &Value) -> Result<Value, ExecutionError> {
		// Handle root path
		if path == "$" {
			return Ok(input.clone());
		}

		let jsonpath =
			JsonPath::parse(path).map_err(|e| ExecutionError::JsonPathError(format!("{}: {}", path, e)))?;

		let nodes = jsonpath.query(input);
		let results: Vec<_> = nodes.iter().map(|v| (*v).clone()).collect();

		Ok(match results.len() {
			0 => Value::Null,
			1 => results.into_iter().next().unwrap(),
			_ => Value::Array(results),
		})
	}

	/// Coalesce: return first non-null value from paths
	fn coalesce(paths: &[String], input: &Value) -> Result<Value, ExecutionError> {
		for path in paths {
			let value = Self::extract_path(path, input)?;
			if !value.is_null() {
				return Ok(value);
			}
		}
		Ok(Value::Null)
	}

	/// Template: string interpolation
	fn template(template: &str, vars: &HashMap<String, String>, input: &Value) -> Result<Value, ExecutionError> {
		let mut result = template.to_string();

		for (name, path) in vars {
			let value = Self::extract_path(path, input)?;
			let str_value = match &value {
				Value::String(s) => s.clone(),
				Value::Number(n) => n.to_string(),
				Value::Bool(b) => b.to_string(),
				Value::Null => String::new(),
				_ => value.to_string(),
			};
			result = result.replace(&format!("{{{}}}", name), &str_value);
		}

		Ok(Value::String(result))
	}

	/// Concatenate values from multiple paths
	fn concat(paths: &[String], separator: Option<&str>, input: &Value) -> Result<Value, ExecutionError> {
		let sep = separator.unwrap_or("");
		let mut parts = Vec::new();

		for path in paths {
			let value = Self::extract_path(path, input)?;
			if let Some(s) = value.as_str() {
				parts.push(s.to_string());
			} else if !value.is_null() {
				parts.push(value.to_string());
			}
		}

		Ok(Value::String(parts.join(sep)))
	}
}

// Helper trait for sync execution of async in nested case
trait NowOrNever {
	type Output;
	fn now_or_never(self) -> Option<Self::Output>;
}

impl<F: std::future::Future> NowOrNever for F {
	type Output = F::Output;
	fn now_or_never(self) -> Option<Self::Output> {
		let waker = futures::task::noop_waker();
		let mut cx = std::task::Context::from_waker(&waker);
		let mut pinned = std::pin::pin!(self);
		match pinned.as_mut().poll(&mut cx) {
			std::task::Poll::Ready(result) => Some(result),
			std::task::Poll::Pending => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mcp::registry::patterns::{CoalesceSource, ConcatSource, LiteralValue, TemplateSource};
	use serde_json::json;

	#[tokio::test]
	async fn test_schema_map_path() {
		let spec = SchemaMapSpec {
			mappings: HashMap::from([
				("title".to_string(), FieldSource::Path("$.paper.title".to_string())),
				("author".to_string(), FieldSource::Path("$.paper.author".to_string())),
			]),
		};

		let input = json!({
			"paper": {
				"title": "Deep Learning",
				"author": "John Doe"
			}
		});

		let result = SchemaMapExecutor::execute(&spec, input).await.unwrap();

		assert_eq!(result["title"], "Deep Learning");
		assert_eq!(result["author"], "John Doe");
	}

	#[tokio::test]
	async fn test_schema_map_literal() {
		let spec = SchemaMapSpec {
			mappings: HashMap::from([
				("source".to_string(), FieldSource::Literal(LiteralValue::StringValue("arxiv".to_string()))),
				("relevance".to_string(), FieldSource::Literal(LiteralValue::NumberValue(0.95))),
				("verified".to_string(), FieldSource::Literal(LiteralValue::BoolValue(true))),
			]),
		};

		let result = SchemaMapExecutor::execute(&spec, json!({})).await.unwrap();

		assert_eq!(result["source"], "arxiv");
		assert_eq!(result["relevance"], 0.95);
		assert_eq!(result["verified"], true);
	}

	#[tokio::test]
	async fn test_schema_map_coalesce() {
		let spec = SchemaMapSpec {
			mappings: HashMap::from([(
				"url".to_string(),
				FieldSource::Coalesce(CoalesceSource {
					paths: vec!["$.pdf_url".to_string(), "$.web_url".to_string(), "$.fallback".to_string()],
				}),
			)]),
		};

		// First path has value
		let input1 = json!({"pdf_url": "http://pdf.example.com"});
		let result1 = SchemaMapExecutor::execute(&spec, input1).await.unwrap();
		assert_eq!(result1["url"], "http://pdf.example.com");

		// First path null, second has value
		let input2 = json!({"pdf_url": null, "web_url": "http://web.example.com"});
		let result2 = SchemaMapExecutor::execute(&spec, input2).await.unwrap();
		assert_eq!(result2["url"], "http://web.example.com");

		// All null
		let input3 = json!({});
		let result3 = SchemaMapExecutor::execute(&spec, input3).await.unwrap();
		assert_eq!(result3["url"], Value::Null);
	}

	#[tokio::test]
	async fn test_schema_map_template() {
		let spec = SchemaMapSpec {
			mappings: HashMap::from([(
				"citation".to_string(),
				FieldSource::Template(TemplateSource {
					template: "{author} ({year}). {title}".to_string(),
					vars: HashMap::from([
						("author".to_string(), "$.author".to_string()),
						("year".to_string(), "$.year".to_string()),
						("title".to_string(), "$.title".to_string()),
					]),
				}),
			)]),
		};

		let input = json!({
			"author": "Smith",
			"year": 2024,
			"title": "A Study"
		});

		let result = SchemaMapExecutor::execute(&spec, input).await.unwrap();
		assert_eq!(result["citation"], "Smith (2024). A Study");
	}

	#[tokio::test]
	async fn test_schema_map_concat() {
		let spec = SchemaMapSpec {
			mappings: HashMap::from([(
				"full_name".to_string(),
				FieldSource::Concat(ConcatSource {
					paths: vec!["$.first".to_string(), "$.last".to_string()],
					separator: Some(" ".to_string()),
				}),
			)]),
		};

		let input = json!({
			"first": "John",
			"last": "Doe"
		});

		let result = SchemaMapExecutor::execute(&spec, input).await.unwrap();
		assert_eq!(result["full_name"], "John Doe");
	}

	#[tokio::test]
	async fn test_schema_map_nested() {
		let inner = SchemaMapSpec {
			mappings: HashMap::from([
				("name".to_string(), FieldSource::Path("$.author.name".to_string())),
				("affiliation".to_string(), FieldSource::Path("$.author.org".to_string())),
			]),
		};

		let spec = SchemaMapSpec {
			mappings: HashMap::from([
				("title".to_string(), FieldSource::Path("$.title".to_string())),
				("author_info".to_string(), FieldSource::Nested(Box::new(inner))),
			]),
		};

		let input = json!({
			"title": "Paper Title",
			"author": {
				"name": "Jane Doe",
				"org": "University"
			}
		});

		let result = SchemaMapExecutor::execute(&spec, input).await.unwrap();
		assert_eq!(result["title"], "Paper Title");
		assert_eq!(result["author_info"]["name"], "Jane Doe");
		assert_eq!(result["author_info"]["affiliation"], "University");
	}
}

