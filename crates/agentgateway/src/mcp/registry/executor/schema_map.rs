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
			FieldSource::Coalesce(c) => Self::coalesce(&c.paths, c.default.as_deref(), input),
			FieldSource::Template(t) => Self::template(&t.template, &t.vars, input),
			FieldSource::Concat(c) => Self::concat(&c.paths, c.separator.as_deref(), input),
			FieldSource::Nested(nested) => {
				let nested_spec = SchemaMapSpec {
					mappings: nested.mappings.clone(),
				};
				Box::pin(Self::execute(&nested_spec, input.clone()))
					.now_or_never()
					.unwrap()
			},
			FieldSource::ArrayMap(am) => Self::array_map(&am.over, &am.each, input),
		}
	}

	/// ArrayMap: iterate over array and apply mappings to each element
	fn array_map(
		over_path: &str,
		each_mappings: &HashMap<String, FieldSource>,
		input: &Value,
	) -> Result<Value, ExecutionError> {
		// Extract the source array
		let array_value = Self::extract_path(over_path, input)?;

		// Get the array items
		let items = match array_value {
			Value::Array(arr) => arr,
			Value::Null => return Ok(Value::Array(vec![])),
			other => vec![other], // Single item becomes array of one
		};

		// Apply the element mappings to each item
		let element_spec = SchemaMapSpec {
			mappings: each_mappings.clone(),
		};

		let transformed: Result<Vec<Value>, ExecutionError> = items
			.into_iter()
			.map(|item| {
				Box::pin(Self::execute(&element_spec, item))
					.now_or_never()
					.unwrap()
			})
			.collect();

		Ok(Value::Array(transformed?))
	}

	/// Extract value using JSONPath
	fn extract_path(path: &str, input: &Value) -> Result<Value, ExecutionError> {
		// Handle root path
		if path == "$" {
			return Ok(input.clone());
		}

		let jsonpath = JsonPath::parse(path)
			.map_err(|e| ExecutionError::JsonPathError(format!("{}: {}", path, e)))?;

		let nodes = jsonpath.query(input);
		let results: Vec<_> = nodes.iter().map(|v| (*v).clone()).collect();

		Ok(match results.len() {
			0 => Value::Null,
			1 => results.into_iter().next().unwrap(),
			_ => Value::Array(results),
		})
	}

	/// Coalesce: return first non-null value from paths, with optional default
	fn coalesce(paths: &[String], default: Option<&str>, input: &Value) -> Result<Value, ExecutionError> {
		for path in paths {
			let value = Self::extract_path(path, input)?;
			if !value.is_null() {
				return Ok(value);
			}
		}
		Ok(match default {
			Some(d) => Value::String(d.to_string()),
			None => Value::Null,
		})
	}

	/// Template: string interpolation
	fn template(
		template: &str,
		vars: &HashMap<String, String>,
		input: &Value,
	) -> Result<Value, ExecutionError> {
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
	fn concat(
		paths: &[String],
		separator: Option<&str>,
		input: &Value,
	) -> Result<Value, ExecutionError> {
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
	use crate::mcp::registry::patterns::{
		ArrayMapSource, CoalesceSource, ConcatSource, LiteralValue, TemplateSource,
	};
	use serde_json::json;

	#[tokio::test]
	async fn test_schema_map_path() {
		let spec = SchemaMapSpec {
			mappings: HashMap::from([
				(
					"title".to_string(),
					FieldSource::Path("$.paper.title".to_string()),
				),
				(
					"author".to_string(),
					FieldSource::Path("$.paper.author".to_string()),
				),
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
				(
					"source".to_string(),
					FieldSource::Literal(LiteralValue::StringValue("arxiv".to_string())),
				),
				(
					"relevance".to_string(),
					FieldSource::Literal(LiteralValue::NumberValue(0.95)),
				),
				(
					"verified".to_string(),
					FieldSource::Literal(LiteralValue::BoolValue(true)),
				),
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
					paths: vec![
						"$.pdf_url".to_string(),
						"$.web_url".to_string(),
						"$.fallback".to_string(),
					],
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
				(
					"name".to_string(),
					FieldSource::Path("$.author.name".to_string()),
				),
				(
					"affiliation".to_string(),
					FieldSource::Path("$.author.org".to_string()),
				),
			]),
		};

		let spec = SchemaMapSpec {
			mappings: HashMap::from([
				(
					"title".to_string(),
					FieldSource::Path("$.title".to_string()),
				),
				(
					"author_info".to_string(),
					FieldSource::Nested(Box::new(inner)),
				),
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

	#[tokio::test]
	async fn test_schema_map_array_map() {
		// Test the arrayMap pattern for iterating over arrays
		let spec = SchemaMapSpec {
			mappings: HashMap::from([(
				"results".to_string(),
				FieldSource::ArrayMap(ArrayMapSource {
					over: "$.papers".to_string(),
					each: HashMap::from([
						("title".to_string(), FieldSource::Path("$.title".to_string())),
						(
							"url".to_string(),
							FieldSource::Coalesce(CoalesceSource {
								paths: vec!["$.pdf_url".to_string(), "$.abs_url".to_string()],
							}),
						),
						(
							"source".to_string(),
							FieldSource::Literal(LiteralValue::StringValue("arxiv".to_string())),
						),
					]),
				}),
			)]),
		};

		let input = json!({
			"papers": [
				{
					"title": "Paper One",
					"pdf_url": "http://example.com/paper1.pdf",
					"abs_url": "http://example.com/paper1",
					"extra_field": "ignored"
				},
				{
					"title": "Paper Two",
					"abs_url": "http://example.com/paper2"
				}
			]
		});

		let result = SchemaMapExecutor::execute(&spec, input).await.unwrap();

		// Verify results is an array
		let results = result["results"].as_array().unwrap();
		assert_eq!(results.len(), 2);

		// First paper: has pdf_url, so coalesce picks it
		assert_eq!(results[0]["title"], "Paper One");
		assert_eq!(results[0]["url"], "http://example.com/paper1.pdf");
		assert_eq!(results[0]["source"], "arxiv");
		// Extra field should not be present
		assert!(results[0].get("extra_field").is_none());

		// Second paper: no pdf_url, so coalesce picks abs_url
		assert_eq!(results[1]["title"], "Paper Two");
		assert_eq!(results[1]["url"], "http://example.com/paper2");
		assert_eq!(results[1]["source"], "arxiv");
	}

	#[tokio::test]
	async fn test_schema_map_array_map_nested() {
		// Test nested arrayMap with nested mappings within each element
		let spec = SchemaMapSpec {
			mappings: HashMap::from([(
				"items".to_string(),
				FieldSource::ArrayMap(ArrayMapSource {
					over: "$.data".to_string(),
					each: HashMap::from([
						("id".to_string(), FieldSource::Path("$.id".to_string())),
						(
							"metadata".to_string(),
							FieldSource::Nested(Box::new(SchemaMapSpec {
								mappings: HashMap::from([
									(
										"created".to_string(),
										FieldSource::Path("$.meta.created_at".to_string()),
									),
									(
										"author".to_string(),
										FieldSource::Path("$.meta.author".to_string()),
									),
								]),
							})),
						),
					]),
				}),
			)]),
		};

		let input = json!({
			"data": [
				{
					"id": "item1",
					"meta": { "created_at": "2024-01-01", "author": "Alice" }
				},
				{
					"id": "item2",
					"meta": { "created_at": "2024-02-01", "author": "Bob" }
				}
			]
		});

		let result = SchemaMapExecutor::execute(&spec, input).await.unwrap();
		let items = result["items"].as_array().unwrap();

		assert_eq!(items[0]["id"], "item1");
		assert_eq!(items[0]["metadata"]["created"], "2024-01-01");
		assert_eq!(items[0]["metadata"]["author"], "Alice");

		assert_eq!(items[1]["id"], "item2");
		assert_eq!(items[1]["metadata"]["created"], "2024-02-01");
		assert_eq!(items[1]["metadata"]["author"], "Bob");
	}
}
