//! Event Processor library
//!
//! This library provides the core event processing functionality for stateful triggers.
//! It exports evaluators and state management for use in integration tests.

pub mod evaluators;
pub mod state_manager;
pub mod cached_state_manager;
pub mod trigger_engine;

// Re-export commonly used types
pub use evaluators::ema::EmaEvaluator;
pub use evaluators::rate_counter::RateCounterEvaluator;
pub use state_manager::TriggerStateManager;
pub use cached_state_manager::CachedStateManager;
