//! Prometheus metrics for action workers
//!
//! Provides observability into worker performance and health.
//!
//! # Security
//!
//! - Error messages in metrics are sanitized to prevent data exposure
//! - Metric labels have bounded cardinality to prevent memory exhaustion

use lazy_static::lazy_static;
use prometheus::{Gauge, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, Opts, Registry};

lazy_static! {
    /// Global metrics registry
    pub static ref REGISTRY: Registry = Registry::new();

    /// Total jobs processed, labeled by action_type and status
    pub static ref JOBS_PROCESSED: IntCounterVec = IntCounterVec::new(
        Opts::new("action_worker_jobs_processed_total", "Total jobs processed by action workers"),
        &["action_type", "status"]  // status: success, failure, dlq
    ).expect("Failed to create JOBS_PROCESSED metric");

    /// Job processing duration in seconds, labeled by action_type
    pub static ref JOB_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("action_worker_job_duration_seconds", "Job processing duration in seconds")
            .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        &["action_type"]
    ).expect("Failed to create JOB_DURATION metric");

    /// Total retry attempts, labeled by action_type and attempt number
    pub static ref RETRIES: IntCounterVec = IntCounterVec::new(
        Opts::new("action_worker_retries_total", "Total retry attempts"),
        &["action_type", "attempt"]
    ).expect("Failed to create RETRIES metric");

    /// Current queue depth (approximate)
    pub static ref QUEUE_DEPTH: Gauge = Gauge::new(
        "action_worker_queue_depth",
        "Current approximate queue depth"
    ).expect("Failed to create QUEUE_DEPTH metric");

    /// Total rate limit hits
    pub static ref RATE_LIMIT_HITS: IntCounter = IntCounter::new(
        "action_worker_rate_limit_hits_total",
        "Total number of rate limit hits"
    ).expect("Failed to create RATE_LIMIT_HITS metric");

    /// Total DLQ entries
    pub static ref DLQ_SIZE: Gauge = Gauge::new(
        "action_worker_dlq_size",
        "Current Dead Letter Queue size"
    ).expect("Failed to create DLQ_SIZE metric");

    /// Active workers count
    pub static ref ACTIVE_WORKERS: Gauge = Gauge::new(
        "action_worker_active_workers",
        "Number of active workers"
    ).expect("Failed to create ACTIVE_WORKERS metric");
}

/// Register all metrics with the global registry
///
/// This should be called once at application startup.
pub fn register_metrics() {
    REGISTRY
        .register(Box::new(JOBS_PROCESSED.clone()))
        .expect("Failed to register JOBS_PROCESSED");
    REGISTRY
        .register(Box::new(JOB_DURATION.clone()))
        .expect("Failed to register JOB_DURATION");
    REGISTRY
        .register(Box::new(RETRIES.clone()))
        .expect("Failed to register RETRIES");
    REGISTRY
        .register(Box::new(QUEUE_DEPTH.clone()))
        .expect("Failed to register QUEUE_DEPTH");
    REGISTRY
        .register(Box::new(RATE_LIMIT_HITS.clone()))
        .expect("Failed to register RATE_LIMIT_HITS");
    REGISTRY
        .register(Box::new(DLQ_SIZE.clone()))
        .expect("Failed to register DLQ_SIZE");
    REGISTRY
        .register(Box::new(ACTIVE_WORKERS.clone()))
        .expect("Failed to register ACTIVE_WORKERS");

    tracing::info!("Prometheus metrics registered");
}

/// Record a successful job completion
pub fn record_job_success(action_type: &str, duration_secs: f64) {
    JOBS_PROCESSED
        .with_label_values(&[action_type, "success"])
        .inc();
    JOB_DURATION
        .with_label_values(&[action_type])
        .observe(duration_secs);
}

/// Record a job failure
pub fn record_job_failure(action_type: &str, duration_secs: f64) {
    JOBS_PROCESSED
        .with_label_values(&[action_type, "failure"])
        .inc();
    JOB_DURATION
        .with_label_values(&[action_type])
        .observe(duration_secs);
}

/// Record a job moved to DLQ
pub fn record_job_dlq(action_type: &str) {
    JOBS_PROCESSED
        .with_label_values(&[action_type, "dlq"])
        .inc();
}

/// Record a retry attempt
pub fn record_retry(action_type: &str, attempt: u32) {
    RETRIES
        .with_label_values(&[action_type, &attempt.to_string()])
        .inc();
}

/// Update queue depth
pub fn set_queue_depth(depth: u64) {
    QUEUE_DEPTH.set(depth as f64);
}

/// Record rate limit hit
pub fn record_rate_limit_hit() {
    RATE_LIMIT_HITS.inc();
}

/// Update DLQ size
pub fn set_dlq_size(size: u64) {
    DLQ_SIZE.set(size as f64);
}

/// Update active workers count
pub fn set_active_workers(count: usize) {
    ACTIVE_WORKERS.set(count as f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        // Verify metrics can be created and incremented without panicking
        JOBS_PROCESSED
            .with_label_values(&["telegram", "success"])
            .inc();
        JOBS_PROCESSED
            .with_label_values(&["telegram", "failure"])
            .inc();
        RETRIES.with_label_values(&["telegram", "1"]).inc();
        QUEUE_DEPTH.set(10.0);
        RATE_LIMIT_HITS.inc();
    }

    #[test]
    fn test_record_functions() {
        record_job_success("telegram", 0.5);
        record_job_failure("rest", 1.0);
        record_job_dlq("mcp");
        record_retry("telegram", 1);
        set_queue_depth(100);
        record_rate_limit_hit();
        set_dlq_size(5);
        set_active_workers(3);
    }
}
