//! Stateful condition evaluators
//!
//! This module provides evaluators for stateful trigger conditions:
//! - EMA (Exponential Moving Average): Smooth score trends
//! - Rate Counter: Count events in sliding time window

pub mod ema;
pub mod rate_counter;

pub use ema::{EmaEvaluator, EmaState};
pub use rate_counter::{RateCounterEvaluator, RateCounterState};
