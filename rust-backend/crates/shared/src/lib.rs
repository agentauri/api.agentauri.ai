//! Shared library for api.8004.dev backend services
//!
//! This crate provides common functionality used across all backend services:
//! - Database connection pooling and utilities
//! - Common data models matching the PostgreSQL schema
//! - Error handling types
//! - Configuration management
//! - Logging infrastructure
//! - Job definitions for event processor and action workers

pub mod config;
pub mod db;
pub mod error;
pub mod jobs;
pub mod models;

// Re-export commonly used types
pub use config::Config;
pub use db::DbPool;
pub use error::{Error, Result};
pub use jobs::{ActionJob, ActionType, ACTION_JOBS_DLQ, ACTION_JOBS_QUEUE};

/// Initialize tracing subscriber for structured logging
pub fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "shared=debug,api_gateway=debug,event_processor=debug,action_workers=debug,info"
                    .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
