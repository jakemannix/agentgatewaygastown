//! Integration tests for the Saga pattern executor.
//!
//! These tests simulate real-world saga scenarios with mock tool backends.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use crate::saga::{
	ActionRouter, InputBinding, OutputBinding, Saga, SagaError, SagaExecutor, SagaStep, StepAction,
};

/// A mock tool backend that tracks calls and simulates tool execution.
struct MockToolBackend {
	/// Map of tool names to their mock responses
	responses: Mutex<HashMap<String, Result<serde_json::Value, String>>>,
	/// Record of all tool calls made
	calls: Mutex<Vec<ToolCall>>,
	/// Optional delay to simulate network latency
	latency: Option<Duration>,
}

#[derive(Debug, Clone)]
struct ToolCall {
	tool_name: String,
	input: serde_json::Value,
}

impl MockToolBackend {
	fn new() -> Self {
		Self {
			responses: Mutex::new(HashMap::new()),
			calls: Mutex::new(Vec::new()),
			latency: None,
		}
	}

	#[allow(dead_code)]
	fn with_latency(mut self, latency: Duration) -> Self {
		self.latency = Some(latency);
		self
	}

	fn mock_tool(&self, name: &str, response: Result<serde_json::Value, String>) {
		self
			.responses
			.lock()
			.unwrap()
			.insert(name.to_string(), response);
	}

	fn get_calls(&self) -> Vec<ToolCall> {
		self.calls.lock().unwrap().clone()
	}

	fn call_count(&self, tool_name: &str) -> usize {
		self
			.calls
			.lock()
			.unwrap()
			.iter()
			.filter(|c| c.tool_name == tool_name)
			.count()
	}
}

#[async_trait]
impl ActionRouter for MockToolBackend {
	async fn execute_action(
		&self,
		action: &StepAction,
		input: serde_json::Value,
		_timeout: Option<Duration>,
	) -> Result<serde_json::Value, SagaError> {
		// Simulate network latency if configured
		if let Some(latency) = self.latency {
			tokio::time::sleep(latency).await;
		}

		let tool_name = match action {
			StepAction::Tool { name } => name.clone(),
			StepAction::Http { method, url } => format!("http:{}:{}", method, url),
			StepAction::Backend { name } => format!("backend:{}", name),
		};

		// Record the call
		self.calls.lock().unwrap().push(ToolCall {
			tool_name: tool_name.clone(),
			input: input.clone(),
		});

		// Return mocked response or default
		let responses = self.responses.lock().unwrap();
		match responses.get(&tool_name) {
			Some(Ok(value)) => Ok(value.clone()),
			Some(Err(msg)) => Err(SagaError::StepFailed {
				step_id: "unknown".to_string(),
				message: msg.clone(),
			}),
			None => Ok(json!({"status": "ok", "tool": tool_name})),
		}
	}
}

/// Integration test: Travel booking saga - happy path
#[tokio::test]
async fn test_travel_booking_saga_success() {
	let backend = Arc::new(MockToolBackend::new());

	// Set up mock responses for each service
	backend.mock_tool(
		"airline.book",
		Ok(json!({
				"confirmation_number": "FL123",
				"price": 450.00,
				"route": "SFO -> JFK"
		})),
	);
	backend.mock_tool(
		"hotel.reserve",
		Ok(json!({
				"reservation_id": "HT456",
				"hotel_name": "Grand Hotel NYC",
				"nights": 3,
				"total": 600.00
		})),
	);
	backend.mock_tool(
		"rental.book",
		Ok(json!({
				"booking_ref": "RC789",
				"vehicle": "Economy",
				"daily_rate": 45.00
		})),
	);
	backend.mock_tool(
		"email.send",
		Ok(json!({
				"sent": true,
				"message_id": "MSG001"
		})),
	);

	let executor = SagaExecutor::new(backend.clone());

	// Build the travel booking saga
	let saga = Saga {
		id: Some("travel-booking-001".to_string()),
		name: Some("Travel Booking".to_string()),
		steps: vec![
			SagaStep {
				id: "flight".to_string(),
				name: Some("Book flight".to_string()),
				action: StepAction::Tool {
					name: "airline.book".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "airline.cancel".to_string(),
				}),
				input: Some(InputBinding::Input {
					path: "$.flight".to_string(),
				}),
				timeout: None,
			},
			SagaStep {
				id: "hotel".to_string(),
				name: Some("Reserve hotel".to_string()),
				action: StepAction::Tool {
					name: "hotel.reserve".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "hotel.cancel".to_string(),
				}),
				input: Some(InputBinding::Input {
					path: "$.hotel".to_string(),
				}),
				timeout: None,
			},
			SagaStep {
				id: "car".to_string(),
				name: Some("Book rental car".to_string()),
				action: StepAction::Tool {
					name: "rental.book".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "rental.cancel".to_string(),
				}),
				input: Some(InputBinding::Input {
					path: "$.car".to_string(),
				}),
				timeout: None,
			},
			SagaStep {
				id: "confirmation".to_string(),
				name: Some("Send confirmation email".to_string()),
				action: StepAction::Tool {
					name: "email.send".to_string(),
				},
				compensate: None, // No compensation for email
				input: Some(InputBinding::Merge(vec![
					InputBinding::Step {
						id: "flight".to_string(),
						path: None,
					},
					InputBinding::Step {
						id: "hotel".to_string(),
						path: None,
					},
					InputBinding::Step {
						id: "car".to_string(),
						path: None,
					},
				])),
				timeout: None,
			},
		],
		output: Some(OutputBinding::Object({
			let mut fields = HashMap::new();
			fields.insert(
				"flight".to_string(),
				OutputBinding::Step {
					id: "flight".to_string(),
					path: None,
				},
			);
			fields.insert(
				"hotel".to_string(),
				OutputBinding::Step {
					id: "hotel".to_string(),
					path: None,
				},
			);
			fields.insert(
				"car".to_string(),
				OutputBinding::Step {
					id: "car".to_string(),
					path: None,
				},
			);
			fields
		})),
		timeout: Some(Duration::from_secs(30)),
	};

	let input = json!({
			"flight": {
					"from": "SFO",
					"to": "JFK",
					"date": "2024-03-15",
					"passengers": 2
			},
			"hotel": {
					"city": "NYC",
					"checkin": "2024-03-15",
					"checkout": "2024-03-18",
					"rooms": 1
			},
			"car": {
					"pickup_location": "JFK",
					"pickup_date": "2024-03-15",
					"return_date": "2024-03-18"
			}
	});

	let result = executor.execute(saga, input).await.unwrap();

	// Verify all steps were executed
	assert_eq!(backend.call_count("airline.book"), 1);
	assert_eq!(backend.call_count("hotel.reserve"), 1);
	assert_eq!(backend.call_count("rental.book"), 1);
	assert_eq!(backend.call_count("email.send"), 1);

	// Verify no compensations were called
	assert_eq!(backend.call_count("airline.cancel"), 0);
	assert_eq!(backend.call_count("hotel.cancel"), 0);
	assert_eq!(backend.call_count("rental.cancel"), 0);

	// Verify output structure
	let output = result.output.as_object().unwrap();
	assert!(output.contains_key("flight"));
	assert!(output.contains_key("hotel"));
	assert!(output.contains_key("car"));

	// Verify flight details in output
	assert_eq!(output["flight"]["confirmation_number"], "FL123");
	assert_eq!(output["hotel"]["reservation_id"], "HT456");
	assert_eq!(output["car"]["booking_ref"], "RC789");
}

/// Integration test: Travel booking saga - hotel fails, compensate flight
#[tokio::test]
async fn test_travel_booking_saga_hotel_failure() {
	let backend = Arc::new(MockToolBackend::new());

	backend.mock_tool(
		"airline.book",
		Ok(json!({
				"confirmation_number": "FL123"
		})),
	);
	backend.mock_tool(
		"hotel.reserve",
		Err("No rooms available for selected dates".to_string()),
	);
	backend.mock_tool(
		"airline.cancel",
		Ok(json!({
				"cancelled": true,
				"refund_amount": 450.00
		})),
	);

	let executor = SagaExecutor::new(backend.clone());

	let saga = Saga {
		id: None,
		name: Some("Travel Booking - Hotel Failure".to_string()),
		steps: vec![
			SagaStep {
				id: "flight".to_string(),
				name: None,
				action: StepAction::Tool {
					name: "airline.book".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "airline.cancel".to_string(),
				}),
				input: None,
				timeout: None,
			},
			SagaStep {
				id: "hotel".to_string(),
				name: None,
				action: StepAction::Tool {
					name: "hotel.reserve".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "hotel.cancel".to_string(),
				}),
				input: None,
				timeout: None,
			},
		],
		output: None,
		timeout: None,
	};

	let result = executor.execute(saga, json!({})).await;

	// Should fail
	assert!(result.is_err());
	let err = result.unwrap_err();
	assert!(
		matches!(err, SagaError::StepFailed { ref step_id, .. } if step_id == "hotel"),
		"Expected StepFailed for hotel, got {:?}",
		err
	);

	// Flight was booked and then cancelled
	assert_eq!(backend.call_count("airline.book"), 1);
	assert_eq!(backend.call_count("airline.cancel"), 1);

	// Hotel was attempted but not cancelled (it never succeeded)
	assert_eq!(backend.call_count("hotel.reserve"), 1);
	assert_eq!(backend.call_count("hotel.cancel"), 0);
}

/// Integration test: Supply chain saga - order processing
#[tokio::test]
async fn test_supply_chain_saga() {
	let backend = Arc::new(MockToolBackend::new());

	backend.mock_tool(
		"inventory.reserve",
		Ok(json!({
				"reservation_id": "INV001",
				"items": [
						{"sku": "WIDGET-A", "quantity": 5},
						{"sku": "GADGET-B", "quantity": 2}
				]
		})),
	);
	backend.mock_tool(
		"payment.process",
		Ok(json!({
				"transaction_id": "TXN123",
				"amount": 299.99,
				"status": "approved"
		})),
	);
	backend.mock_tool(
		"shipping.schedule",
		Ok(json!({
				"shipment_id": "SHIP456",
				"carrier": "FedEx",
				"tracking_number": "FX123456789",
				"estimated_delivery": "2024-03-20"
		})),
	);
	backend.mock_tool(
		"notification.send",
		Ok(json!({
				"notified": true
		})),
	);

	let executor = SagaExecutor::new(backend.clone());

	let saga = Saga {
		id: Some("order-001".to_string()),
		name: Some("Order Processing".to_string()),
		steps: vec![
			SagaStep {
				id: "reserve_inventory".to_string(),
				name: Some("Reserve inventory".to_string()),
				action: StepAction::Tool {
					name: "inventory.reserve".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "inventory.release".to_string(),
				}),
				input: Some(InputBinding::Input {
					path: "$.items".to_string(),
				}),
				timeout: None,
			},
			SagaStep {
				id: "process_payment".to_string(),
				name: Some("Process payment".to_string()),
				action: StepAction::Tool {
					name: "payment.process".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "payment.refund".to_string(),
				}),
				input: Some(InputBinding::Input {
					path: "$.payment".to_string(),
				}),
				timeout: None,
			},
			SagaStep {
				id: "schedule_shipping".to_string(),
				name: Some("Schedule shipping".to_string()),
				action: StepAction::Tool {
					name: "shipping.schedule".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "shipping.cancel".to_string(),
				}),
				input: Some(InputBinding::Merge(vec![
					InputBinding::Input {
						path: "$.shipping".to_string(),
					},
					InputBinding::Step {
						id: "reserve_inventory".to_string(),
						path: Some("$.reservation_id".to_string()),
					},
				])),
				timeout: None,
			},
			SagaStep {
				id: "send_notification".to_string(),
				name: Some("Send order confirmation".to_string()),
				action: StepAction::Tool {
					name: "notification.send".to_string(),
				},
				compensate: None, // No need to "un-notify"
				input: Some(InputBinding::Merge(vec![
					InputBinding::Step {
						id: "process_payment".to_string(),
						path: Some("$.transaction_id".to_string()),
					},
					InputBinding::Step {
						id: "schedule_shipping".to_string(),
						path: Some("$.tracking_number".to_string()),
					},
				])),
				timeout: None,
			},
		],
		output: Some(OutputBinding::Object({
			let mut fields = HashMap::new();
			fields.insert(
				"order_id".to_string(),
				OutputBinding::Step {
					id: "reserve_inventory".to_string(),
					path: Some("$.reservation_id".to_string()),
				},
			);
			fields.insert(
				"transaction".to_string(),
				OutputBinding::Step {
					id: "process_payment".to_string(),
					path: None,
				},
			);
			fields.insert(
				"shipment".to_string(),
				OutputBinding::Step {
					id: "schedule_shipping".to_string(),
					path: None,
				},
			);
			fields
		})),
		timeout: Some(Duration::from_secs(60)),
	};

	let input = json!({
			"items": [
					{"sku": "WIDGET-A", "quantity": 5},
					{"sku": "GADGET-B", "quantity": 2}
			],
			"payment": {
					"method": "credit_card",
					"card_last_four": "4242",
					"amount": 299.99
			},
			"shipping": {
					"address": "123 Main St, Anytown, USA",
					"method": "express"
			}
	});

	let result = executor.execute(saga, input).await.unwrap();

	// Verify all steps executed
	assert_eq!(backend.call_count("inventory.reserve"), 1);
	assert_eq!(backend.call_count("payment.process"), 1);
	assert_eq!(backend.call_count("shipping.schedule"), 1);
	assert_eq!(backend.call_count("notification.send"), 1);

	// Verify output structure
	let output = result.output.as_object().unwrap();
	assert_eq!(output["order_id"], "INV001");
	assert_eq!(output["transaction"]["transaction_id"], "TXN123");
	assert_eq!(output["shipment"]["tracking_number"], "FX123456789");
}

/// Integration test: Supply chain saga - payment fails, compensate inventory
#[tokio::test]
async fn test_supply_chain_saga_payment_failure() {
	let backend = Arc::new(MockToolBackend::new());

	backend.mock_tool(
		"inventory.reserve",
		Ok(json!({
				"reservation_id": "INV001"
		})),
	);
	backend.mock_tool("payment.process", Err("Card declined".to_string()));
	backend.mock_tool(
		"inventory.release",
		Ok(json!({
				"released": true
		})),
	);

	let executor = SagaExecutor::new(backend.clone());

	let saga = Saga {
		id: None,
		name: None,
		steps: vec![
			SagaStep {
				id: "inventory".to_string(),
				name: None,
				action: StepAction::Tool {
					name: "inventory.reserve".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "inventory.release".to_string(),
				}),
				input: None,
				timeout: None,
			},
			SagaStep {
				id: "payment".to_string(),
				name: None,
				action: StepAction::Tool {
					name: "payment.process".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "payment.refund".to_string(),
				}),
				input: None,
				timeout: None,
			},
		],
		output: None,
		timeout: None,
	};

	let result = executor.execute(saga, json!({})).await;

	assert!(result.is_err());

	// Inventory was reserved and then released
	assert_eq!(backend.call_count("inventory.reserve"), 1);
	assert_eq!(backend.call_count("inventory.release"), 1);

	// Payment failed, no refund needed
	assert_eq!(backend.call_count("payment.process"), 1);
	assert_eq!(backend.call_count("payment.refund"), 0);
}

/// Integration test: Saga with step-to-step data passing
#[tokio::test]
async fn test_saga_data_flow() {
	let backend = Arc::new(MockToolBackend::new());

	backend.mock_tool(
		"user.create",
		Ok(json!({
				"user_id": "U12345",
				"username": "newuser"
		})),
	);
	backend.mock_tool(
		"profile.create",
		Ok(json!({
				"profile_id": "P67890",
				"user_id": "U12345"
		})),
	);
	backend.mock_tool(
		"welcome.send",
		Ok(json!({
				"email_sent": true
		})),
	);

	let executor = SagaExecutor::new(backend.clone());

	let saga = Saga {
		id: None,
		name: Some("User Onboarding".to_string()),
		steps: vec![
			SagaStep {
				id: "create_user".to_string(),
				name: None,
				action: StepAction::Tool {
					name: "user.create".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "user.delete".to_string(),
				}),
				input: Some(InputBinding::Input {
					path: "$.user".to_string(),
				}),
				timeout: None,
			},
			SagaStep {
				id: "create_profile".to_string(),
				name: None,
				action: StepAction::Tool {
					name: "profile.create".to_string(),
				},
				compensate: Some(StepAction::Tool {
					name: "profile.delete".to_string(),
				}),
				// Pass user_id from previous step
				input: Some(InputBinding::Step {
					id: "create_user".to_string(),
					path: Some("$.user_id".to_string()),
				}),
				timeout: None,
			},
			SagaStep {
				id: "send_welcome".to_string(),
				name: None,
				action: StepAction::Tool {
					name: "welcome.send".to_string(),
				},
				compensate: None,
				// Merge user and profile data
				input: Some(InputBinding::Merge(vec![
					InputBinding::Step {
						id: "create_user".to_string(),
						path: None,
					},
					InputBinding::Step {
						id: "create_profile".to_string(),
						path: None,
					},
				])),
				timeout: None,
			},
		],
		output: Some(OutputBinding::All),
		timeout: None,
	};

	let input = json!({
			"user": {
					"email": "newuser@example.com",
					"name": "New User"
			}
	});

	let result = executor.execute(saga, input).await.unwrap();

	// Verify the data flow
	let calls = backend.get_calls();

	// Second step should have received user_id from first step
	let profile_call = calls
		.iter()
		.find(|c| c.tool_name == "profile.create")
		.unwrap();
	assert_eq!(profile_call.input, json!("U12345"));

	// Third step should have merged data from both previous steps
	let welcome_call = calls
		.iter()
		.find(|c| c.tool_name == "welcome.send")
		.unwrap();
	let welcome_input = welcome_call.input.as_object().unwrap();
	assert!(welcome_input.contains_key("user_id"));
	assert!(welcome_input.contains_key("profile_id"));

	// Verify final output
	assert!(result.step_results.contains_key("create_user"));
	assert!(result.step_results.contains_key("create_profile"));
	assert!(result.step_results.contains_key("send_welcome"));
}
