use crate::{api::EndpointCall, ops};

///
/// EndpointResultMetrics
/// Endpoint result metrics exposed to user code and macros
///

pub struct EndpointResultMetrics;

impl EndpointResultMetrics {
    pub fn increment_ok(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointResultMetrics::increment_ok(call);
    }

    pub fn increment_err(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointResultMetrics::increment_err(call);
    }
}

///
/// EndpointAttemptMetrics
///

pub struct EndpointAttemptMetrics;

impl EndpointAttemptMetrics {
    pub fn increment_attempted(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointAttemptMetrics::increment_attempted(call);
    }

    pub fn increment_completed(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointAttemptMetrics::increment_completed(call);
    }
}

///
/// AccessMetrics
/// (access / denial metrics)
///

pub struct AccessMetrics;

impl AccessMetrics {
    pub fn increment(call: EndpointCall, kind: crate::dto::metrics::AccessMetricKind) {
        let kind = match kind {
            crate::dto::metrics::AccessMetricKind::Auth => {
                crate::storage::metrics::access::AccessMetricKind::Auth
            }
            crate::dto::metrics::AccessMetricKind::Guard => {
                crate::storage::metrics::access::AccessMetricKind::Guard
            }
            crate::dto::metrics::AccessMetricKind::Rule => {
                crate::storage::metrics::access::AccessMetricKind::Rule
            }
        };
        ops::runtime::metrics::access::AccessMetrics::increment(call, kind);
    }
}
