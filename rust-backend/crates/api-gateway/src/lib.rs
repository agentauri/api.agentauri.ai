//! API Gateway Library
//!
//! This library module exposes the core functionality of the API Gateway
//! for use in integration tests and potential future library consumers.

// TODO: Fix Clippy warnings in follow-up PR
#![allow(clippy::all)]
// Allow dead code for OAuth and wallet services (not yet fully implemented)
#![allow(dead_code)]

pub mod background_tasks;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod validators;
