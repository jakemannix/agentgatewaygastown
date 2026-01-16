//! Saga pattern implementation for distributed transactions.
//!
//! The Saga pattern provides a way to manage distributed transactions through
//! a sequence of steps with compensating actions. If a step fails, previously
//! completed steps are compensated in reverse order to maintain consistency.
//!
//! # Example
//!
//! ```json
//! {
//!   "saga": {
//!     "name": "Travel Booking",
//!     "steps": [
//!       {
//!         "id": "flight",
//!         "action": { "tool": { "name": "airline.book" } },
//!         "compensate": { "tool": { "name": "airline.cancel" } }
//!       },
//!       {
//!         "id": "hotel",
//!         "action": { "tool": { "name": "hotel.reserve" } },
//!         "compensate": { "tool": { "name": "hotel.cancel" } }
//!       }
//!     ]
//!   }
//! }
//! ```

mod executor;
#[cfg(test)]
mod integration_tests;
mod types;

pub use executor::{ActionRouter, SagaError, SagaExecutor, SagaResult, SagaStatus, StepResult};
pub use types::{InputBinding, OutputBinding, Saga, SagaStep, StepAction};
