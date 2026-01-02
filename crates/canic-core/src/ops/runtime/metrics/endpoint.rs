use crate::ops::runtime::metrics::access::AccessMetricKey;
use crate::storage::metrics::{
    access::AccessMetrics as ModelAccessMetrics,
    endpoint::{
        EndpointAttemptCounts as ModelEndpointAttemptCounts,
        EndpointAttemptMetrics as ModelEndpointAttemptMetrics,
        EndpointResultCounts as ModelEndpointResultCounts,
        EndpointResultMetrics as ModelEndpointResultMetrics,
    },
};

#[derive(Clone)]
pub struct EndpointHealthSnapshot {
    pub attempts: Vec<(&'static str, EndpointAttemptCounts)>,
    pub results: Vec<(&'static str, EndpointResultCounts)>,
    pub access: Vec<(AccessMetricKey, u64)>,
}

#[derive(Clone, Default)]
pub struct EndpointAttemptCounts {
    pub attempted: u64,
    pub completed: u64,
}

#[derive(Clone, Default)]
pub struct EndpointResultCounts {
    pub ok: u64,
    pub err: u64,
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
        .map(|(endpoint, counts)| (endpoint, counts.into()))
        .collect();
    let results = ModelEndpointResultMetrics::export_raw()
        .into_iter()
        .map(|(endpoint, counts)| (endpoint, counts.into()))
        .collect();
    let access = ModelAccessMetrics::export_raw()
        .into_iter()
        .map(|(key, count)| (key.into(), count))
        .collect();

    EndpointHealthSnapshot {
        attempts,
        results,
        access,
    }
}

impl From<ModelEndpointAttemptCounts> for EndpointAttemptCounts {
    fn from(counts: ModelEndpointAttemptCounts) -> Self {
        Self {
            attempted: counts.attempted,
            completed: counts.completed,
        }
    }
}

impl From<ModelEndpointResultCounts> for EndpointResultCounts {
    fn from(counts: ModelEndpointResultCounts) -> Self {
        Self {
            ok: counts.ok,
            err: counts.err,
        }
    }
}
