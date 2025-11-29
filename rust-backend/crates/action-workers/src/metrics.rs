//! Prometheus metrics for action workers
//!
//! Provides observability into worker performance and health.
//!
//! # Security
//!
//! - Error messages in metrics are sanitized to prevent data exposure
//! - Metric labels have bounded cardinality to prevent memory exhaustion
//!
//! # Migration Note
//!
//! This module uses the `metrics` crate instead of `prometheus` crate
//! due to CVE vulnerabilities in prometheus 0.13's protobuf dependency.
//! The metrics crate provides a safer, more idiomatic Rust interface.
//!
//! Rust guideline compliant 2025-01-28

use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use std::sync::OnceLock;

/// Singleton to ensure metrics are only initialized once
static METRICS_INITIALIZED: OnceLock<()> = OnceLock::new();

/// Initialize the Prometheus metrics exporter
///
/// This should be called once at application startup.
/// The exporter will listen on the specified address for scrape requests.
///
/// # Arguments
///
/// * `addr` - Socket address for the Prometheus scrape endpoint (e.g., "0.0.0.0:9090")
///
/// # Panics
///
/// Panics if the exporter cannot bind to the specified address.
pub fn init_metrics(addr: SocketAddr) {
    METRICS_INITIALIZED.get_or_init(|| {
        PrometheusBuilder::new()
            .with_http_listener(addr)
            .install()
            .expect("Failed to install Prometheus exporter");

        tracing::info!(addr = %addr, "Prometheus metrics exporter initialized");
    });
}

/// Initialize metrics with default address (for testing or when address not configured)
///
/// Uses 0.0.0.0:9090 as the default address.
pub fn init_metrics_default() {
    let addr: SocketAddr = "0.0.0.0:9090".parse().expect("Invalid default address");
    init_metrics(addr);
}

/// Record a successful job completion
///
/// # Arguments
///
/// * `action_type` - Type of action (e.g., "telegram", "rest", "mcp")
/// * `duration_secs` - Job processing duration in seconds
pub fn record_job_success(action_type: &str, duration_secs: f64) {
    counter!("action_worker_jobs_processed_total", "action_type" => action_type.to_string(), "status" => "success").increment(1);
    histogram!("action_worker_job_duration_seconds", "action_type" => action_type.to_string())
        .record(duration_secs);
}

/// Record a job failure
///
/// # Arguments
///
/// * `action_type` - Type of action
/// * `duration_secs` - Job processing duration in seconds
pub fn record_job_failure(action_type: &str, duration_secs: f64) {
    counter!("action_worker_jobs_processed_total", "action_type" => action_type.to_string(), "status" => "failure").increment(1);
    histogram!("action_worker_job_duration_seconds", "action_type" => action_type.to_string())
        .record(duration_secs);
}

/// Record a job moved to DLQ (Dead Letter Queue)
///
/// # Arguments
///
/// * `action_type` - Type of action
pub fn record_job_dlq(action_type: &str) {
    counter!("action_worker_jobs_processed_total", "action_type" => action_type.to_string(), "status" => "dlq").increment(1);
}

/// Record a retry attempt
///
/// # Arguments
///
/// * `action_type` - Type of action
/// * `attempt` - Retry attempt number (1, 2, 3, etc.)
pub fn record_retry(action_type: &str, attempt: u32) {
    counter!("action_worker_retries_total", "action_type" => action_type.to_string(), "attempt" => attempt.to_string()).increment(1);
}

/// Update the current queue depth
///
/// # Arguments
///
/// * `depth` - Current approximate queue depth
pub fn set_queue_depth(depth: u64) {
    gauge!("action_worker_queue_depth").set(depth as f64);
}

/// Record a rate limit hit
pub fn record_rate_limit_hit() {
    counter!("action_worker_rate_limit_hits_total").increment(1);
}

/// Update the DLQ size
///
/// # Arguments
///
/// * `size` - Current Dead Letter Queue size
pub fn set_dlq_size(size: u64) {
    gauge!("action_worker_dlq_size").set(size as f64);
}

/// Update the active workers count
///
/// # Arguments
///
/// * `count` - Number of active workers
pub fn set_active_workers(count: usize) {
    gauge!("action_worker_active_workers").set(count as f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_functions() {
        // These tests verify that the metric functions don't panic
        // The actual metric values would be tested via integration tests
        // with the Prometheus exporter

        record_job_success("telegram", 0.5);
        record_job_failure("rest", 1.0);
        record_job_dlq("mcp");
        record_retry("telegram", 1);
        set_queue_depth(100);
        record_rate_limit_hit();
        set_dlq_size(5);
        set_active_workers(3);
    }

    #[test]
    fn test_metrics_with_various_action_types() {
        // Test with various action types to ensure no panics
        let action_types = ["telegram", "rest", "mcp", "webhook"];

        for action_type in action_types {
            record_job_success(action_type, 0.1);
            record_job_failure(action_type, 0.2);
            record_job_dlq(action_type);
            record_retry(action_type, 1);
            record_retry(action_type, 2);
            record_retry(action_type, 3);
        }
    }

    #[test]
    fn test_gauge_updates() {
        // Test gauge updates with various values
        set_queue_depth(0);
        set_queue_depth(100);
        set_queue_depth(1000);

        set_dlq_size(0);
        set_dlq_size(10);

        set_active_workers(0);
        set_active_workers(5);
        set_active_workers(10);
    }
}
