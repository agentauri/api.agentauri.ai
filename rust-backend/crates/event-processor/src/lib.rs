//! Event Processor library
//!
//! This library provides the core event processing functionality for stateful triggers.
//! It exports evaluators and state management for use in integration tests.

pub mod cached_state_manager;
pub mod circuit_breaker;
pub mod evaluators;
pub mod polling_fallback;
pub mod processor;
pub mod queue;
pub mod state_manager;
pub mod trigger_engine;

// Re-export commonly used types
pub use cached_state_manager::CachedStateManager;
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState, CircuitState,
};
pub use evaluators::ema::EmaEvaluator;
pub use evaluators::rate_counter::RateCounterEvaluator;
pub use polling_fallback::PollingFallback;
pub use state_manager::TriggerStateManager;
