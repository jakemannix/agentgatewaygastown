//! Tests for the ContentRouter pattern.

use serde_json::json;

use super::RouterExecutor;
use super::types::{
	ExecutionError, FieldPredicate, PredicateOperator, RouteCase, RouterSpec, StepOperation,
};

/// Helper to create a simple field predicate.
fn field_eq(field: &str, value: serde_json::Value) -> FieldPredicate {
	FieldPredicate {
		field: field.to_string(),
		operator: PredicateOperator::Eq,
		value,
	}
}

/// Helper to create a transform operation that marks which route was taken.
fn marker(name: &str) -> StepOperation {
	StepOperation::Transform {
		expression: format!("route:{}", name),
	}
}

#[test]
fn test_router_first_match() {
	// When multiple routes match, the first one should win
	let spec = RouterSpec {
		routes: vec![
			RouteCase {
				when: field_eq("type", json!("a")),
				then: marker("first"),
			},
			RouteCase {
				when: field_eq("type", json!("a")), // Also matches, but should not be chosen
				then: marker("second"),
			},
		],
		otherwise: None,
	};

	let input = json!({ "type": "a", "data": "test" });
	let result = RouterExecutor::execute(&spec, input.clone());

	assert!(result.is_ok());
	// The input should be returned (passthrough behavior for Transform stub)
	assert_eq!(result.unwrap(), input);
}

#[test]
fn test_router_second_match() {
	// When first route doesn't match, second route should be evaluated and match
	let spec = RouterSpec {
		routes: vec![
			RouteCase {
				when: field_eq("type", json!("a")),
				then: marker("first"),
			},
			RouteCase {
				when: field_eq("type", json!("b")),
				then: marker("second"),
			},
		],
		otherwise: None,
	};

	let input = json!({ "type": "b", "data": "test" });
	let result = RouterExecutor::execute(&spec, input.clone());

	assert!(result.is_ok());
	// The second route should be taken
	assert_eq!(result.unwrap(), input);
}

#[test]
fn test_router_otherwise() {
	// When no route matches, the otherwise clause should be executed
	let spec = RouterSpec {
		routes: vec![
			RouteCase {
				when: field_eq("type", json!("a")),
				then: marker("route_a"),
			},
			RouteCase {
				when: field_eq("type", json!("b")),
				then: marker("route_b"),
			},
		],
		otherwise: Some(Box::new(marker("default"))),
	};

	let input = json!({ "type": "c", "data": "test" });
	let result = RouterExecutor::execute(&spec, input.clone());

	assert!(result.is_ok());
	// The otherwise clause should be taken
	assert_eq!(result.unwrap(), input);
}

#[test]
fn test_router_no_match_no_otherwise() {
	// When no route matches and there's no otherwise, error should be returned
	let spec = RouterSpec {
		routes: vec![RouteCase {
			when: field_eq("type", json!("a")),
			then: marker("route_a"),
		}],
		otherwise: None,
	};

	let input = json!({ "type": "b", "data": "test" });
	let result = RouterExecutor::execute(&spec, input);

	assert!(result.is_err());
	match result.unwrap_err() {
		ExecutionError::NoRouteMatch => {}, // Expected
		e => panic!("unexpected error: {:?}", e),
	}
}

#[test]
fn test_router_complex_predicate() {
	// Test nested predicates with various operators
	let spec = RouterSpec {
		routes: vec![
			RouteCase {
				when: FieldPredicate {
					field: "status.code".to_string(),
					operator: PredicateOperator::Gte,
					value: json!(400),
				},
				then: marker("error"),
			},
			RouteCase {
				when: FieldPredicate {
					field: "status.code".to_string(),
					operator: PredicateOperator::Gte,
					value: json!(200),
				},
				then: marker("success"),
			},
		],
		otherwise: Some(Box::new(marker("unknown"))),
	};

	// Test error case (code >= 400)
	let error_input = json!({ "status": { "code": 500 }, "data": "error" });
	let result = RouterExecutor::execute(&spec, error_input.clone());
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), error_input);

	// Test success case (code >= 200 but < 400)
	let success_input = json!({ "status": { "code": 200 }, "data": "ok" });
	let result = RouterExecutor::execute(&spec, success_input.clone());
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), success_input);

	// Test otherwise case (code < 200)
	let unknown_input = json!({ "status": { "code": 100 }, "data": "info" });
	let result = RouterExecutor::execute(&spec, unknown_input.clone());
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), unknown_input);
}

#[test]
fn test_predicate_operators() {
	use super::evaluate_predicate;

	// Test Eq
	let pred = field_eq("name", json!("test"));
	assert!(evaluate_predicate(&pred, &json!({"name": "test"})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"name": "other"})).unwrap());

	// Test Neq
	let pred = FieldPredicate {
		field: "name".to_string(),
		operator: PredicateOperator::Neq,
		value: json!("test"),
	};
	assert!(!evaluate_predicate(&pred, &json!({"name": "test"})).unwrap());
	assert!(evaluate_predicate(&pred, &json!({"name": "other"})).unwrap());

	// Test numeric comparisons
	let pred = FieldPredicate {
		field: "count".to_string(),
		operator: PredicateOperator::Gt,
		value: json!(5),
	};
	assert!(evaluate_predicate(&pred, &json!({"count": 10})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"count": 5})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"count": 3})).unwrap());

	// Test Gte
	let pred = FieldPredicate {
		field: "count".to_string(),
		operator: PredicateOperator::Gte,
		value: json!(5),
	};
	assert!(evaluate_predicate(&pred, &json!({"count": 10})).unwrap());
	assert!(evaluate_predicate(&pred, &json!({"count": 5})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"count": 3})).unwrap());

	// Test Lt
	let pred = FieldPredicate {
		field: "count".to_string(),
		operator: PredicateOperator::Lt,
		value: json!(5),
	};
	assert!(!evaluate_predicate(&pred, &json!({"count": 10})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"count": 5})).unwrap());
	assert!(evaluate_predicate(&pred, &json!({"count": 3})).unwrap());

	// Test Lte
	let pred = FieldPredicate {
		field: "count".to_string(),
		operator: PredicateOperator::Lte,
		value: json!(5),
	};
	assert!(!evaluate_predicate(&pred, &json!({"count": 10})).unwrap());
	assert!(evaluate_predicate(&pred, &json!({"count": 5})).unwrap());
	assert!(evaluate_predicate(&pred, &json!({"count": 3})).unwrap());
}

#[test]
fn test_string_operators() {
	use super::evaluate_predicate;

	// Test Contains
	let pred = FieldPredicate {
		field: "message".to_string(),
		operator: PredicateOperator::Contains,
		value: json!("error"),
	};
	assert!(evaluate_predicate(&pred, &json!({"message": "an error occurred"})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"message": "all good"})).unwrap());

	// Test StartsWith
	let pred = FieldPredicate {
		field: "path".to_string(),
		operator: PredicateOperator::StartsWith,
		value: json!("/api/"),
	};
	assert!(evaluate_predicate(&pred, &json!({"path": "/api/users"})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"path": "/web/users"})).unwrap());

	// Test EndsWith
	let pred = FieldPredicate {
		field: "filename".to_string(),
		operator: PredicateOperator::EndsWith,
		value: json!(".json"),
	};
	assert!(evaluate_predicate(&pred, &json!({"filename": "data.json"})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"filename": "data.xml"})).unwrap());
}

#[test]
fn test_matches_operator() {
	use super::evaluate_predicate;

	let pred = FieldPredicate {
		field: "email".to_string(),
		operator: PredicateOperator::Matches,
		value: json!(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"),
	};
	assert!(evaluate_predicate(&pred, &json!({"email": "test@example.com"})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"email": "invalid-email"})).unwrap());
}

#[test]
fn test_exists_operator() {
	use super::evaluate_predicate;

	let pred = FieldPredicate {
		field: "optional".to_string(),
		operator: PredicateOperator::Exists,
		value: json!(null), // Value is ignored for Exists
	};
	assert!(evaluate_predicate(&pred, &json!({"optional": "value"})).unwrap());
	assert!(evaluate_predicate(&pred, &json!({"optional": 0})).unwrap());
	assert!(evaluate_predicate(&pred, &json!({"optional": false})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"optional": null})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"other": "value"})).unwrap());
}

#[test]
fn test_in_operator() {
	use super::evaluate_predicate;

	let pred = FieldPredicate {
		field: "status".to_string(),
		operator: PredicateOperator::In,
		value: json!(["active", "pending", "review"]),
	};
	assert!(evaluate_predicate(&pred, &json!({"status": "active"})).unwrap());
	assert!(evaluate_predicate(&pred, &json!({"status": "pending"})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"status": "deleted"})).unwrap());
}

#[test]
fn test_nested_field_access() {
	use super::evaluate_predicate;

	let pred = FieldPredicate {
		field: "user.profile.name".to_string(),
		operator: PredicateOperator::Eq,
		value: json!("Alice"),
	};
	let input = json!({
			"user": {
					"profile": {
							"name": "Alice",
							"age": 30
					}
			}
	});
	assert!(evaluate_predicate(&pred, &input).unwrap());

	let pred = FieldPredicate {
		field: "user.profile.age".to_string(),
		operator: PredicateOperator::Gt,
		value: json!(25),
	};
	assert!(evaluate_predicate(&pred, &input).unwrap());
}

#[test]
fn test_array_contains() {
	use super::evaluate_predicate;

	let pred = FieldPredicate {
		field: "tags".to_string(),
		operator: PredicateOperator::Contains,
		value: json!("important"),
	};
	assert!(evaluate_predicate(&pred, &json!({"tags": ["urgent", "important", "bug"]})).unwrap());
	assert!(!evaluate_predicate(&pred, &json!({"tags": ["minor", "bug"]})).unwrap());
}

#[test]
fn test_nested_router() {
	// Test a router inside a router (nested routing)
	let inner_router = RouterSpec {
		routes: vec![RouteCase {
			when: FieldPredicate {
				field: "priority".to_string(),
				operator: PredicateOperator::Eq,
				value: json!("high"),
			},
			then: marker("urgent_error"),
		}],
		otherwise: Some(Box::new(marker("normal_error"))),
	};

	let outer_router = RouterSpec {
		routes: vec![RouteCase {
			when: FieldPredicate {
				field: "type".to_string(),
				operator: PredicateOperator::Eq,
				value: json!("error"),
			},
			then: StepOperation::Router(Box::new(inner_router)),
		}],
		otherwise: Some(Box::new(marker("not_error"))),
	};

	// High priority error - should go through both routers
	let input = json!({ "type": "error", "priority": "high" });
	let result = RouterExecutor::execute(&outer_router, input.clone());
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), input);

	// Low priority error - should go to inner router's otherwise
	let input = json!({ "type": "error", "priority": "low" });
	let result = RouterExecutor::execute(&outer_router, input.clone());
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), input);

	// Not an error - should go to outer router's otherwise
	let input = json!({ "type": "success", "priority": "high" });
	let result = RouterExecutor::execute(&outer_router, input.clone());
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), input);
}

#[test]
fn test_serde_roundtrip() {
	// Test that types can be serialized and deserialized
	let spec = RouterSpec {
		routes: vec![RouteCase {
			when: FieldPredicate {
				field: "type".to_string(),
				operator: PredicateOperator::Eq,
				value: json!("test"),
			},
			then: StepOperation::Passthrough,
		}],
		otherwise: Some(Box::new(StepOperation::Transform {
			expression: "fallback".to_string(),
		})),
	};

	let json = serde_json::to_string(&spec).unwrap();
	let parsed: RouterSpec = serde_json::from_str(&json).unwrap();
	assert_eq!(spec, parsed);
}
