use crate::{
    api::EndpointCall,
    storage::metrics::{
        access::{AccessMetricKey, AccessMetrics as ModelAccessMetrics},
        endpoint::{
            EndpointAttemptCounts, EndpointAttemptMetrics as ModelEndpointAttemptMetrics,
            EndpointResultCounts, EndpointResultMetrics as ModelEndpointResultMetrics,
        },
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
    pub fn increment_attempted(call: EndpointCall) {
        ModelEndpointAttemptMetrics::increment_attempted(call.endpoint.name);
    }

    pub fn increment_completed(call: EndpointCall) {
        ModelEndpointAttemptMetrics::increment_completed(call.endpoint.name);
    }
}

pub struct EndpointResultMetrics;

impl EndpointResultMetrics {
    pub fn increment_ok(call: EndpointCall) {
        ModelEndpointResultMetrics::increment_ok(call.endpoint.name);
    }

    pub fn increment_err(call: EndpointCall) {
        ModelEndpointResultMetrics::increment_err(call.endpoint.name);
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
