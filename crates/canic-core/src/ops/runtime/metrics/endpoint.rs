use crate::storage::metrics::{
    access::{AccessMetricKey, AccessMetrics as ModelAccessMetrics},
    endpoint::{
        EndpointAttemptCounts, EndpointAttemptMetrics as ModelEndpointAttemptMetrics,
        EndpointResultCounts, EndpointResultMetrics as ModelEndpointResultMetrics,
    },
};

#[derive(Clone)]
pub struct EndpointHealthSnapshot {
    pub attempts: Vec<(&'static str, EndpointAttemptCounts)>,
    pub results: Vec<(&'static str, EndpointResultCounts)>,
    pub access: Vec<(AccessMetricKey, u64)>,
}

pub struct EndpointAttemptMetrics;

impl EndpointAttemptMetrics {
    pub fn increment_attempted(endpoint: &'static str) {
        ModelEndpointAttemptMetrics::increment_attempted(endpoint);
    }

    pub fn increment_completed(endpoint: &'static str) {
        ModelEndpointAttemptMetrics::increment_completed(endpoint);
    }
}

pub struct EndpointResultMetrics;

impl EndpointResultMetrics {
    pub fn increment_ok(endpoint: &'static str) {
        ModelEndpointResultMetrics::increment_ok(endpoint);
    }

    pub fn increment_err(endpoint: &'static str) {
        ModelEndpointResultMetrics::increment_err(endpoint);
    }
}

#[must_use]
pub fn health_snapshot() -> EndpointHealthSnapshot {
    let attempts = ModelEndpointAttemptMetrics::export_raw()
        .into_iter()
        .collect();
    let results = ModelEndpointResultMetrics::export_raw()
        .into_iter()
        .collect();
    let access = ModelAccessMetrics::export_raw().into_iter().collect();

    EndpointHealthSnapshot {
        attempts,
        results,
        access,
    }
}
