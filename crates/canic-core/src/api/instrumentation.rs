use crate::{api::EndpointCall, ops};

///
/// AccessMetrics
/// Access/denial metrics exposed to macro-expanded endpoints.
///

pub struct AccessMetrics;

impl AccessMetrics {
    pub fn increment(call: EndpointCall, kind: crate::dto::metrics::AccessMetricKind) {
        let kind = match kind {
            crate::dto::metrics::AccessMetricKind::Auth => {
                ops::runtime::metrics::access::AccessMetricKind::Auth
            }
            crate::dto::metrics::AccessMetricKind::Guard => {
                ops::runtime::metrics::access::AccessMetricKind::Guard
            }
            crate::dto::metrics::AccessMetricKind::Rule => {
                ops::runtime::metrics::access::AccessMetricKind::Rule
            }
        };
        ops::runtime::metrics::access::AccessMetrics::increment(call.endpoint.name, kind);
    }
}

///
/// EndpointAttemptMetrics
///

pub struct EndpointAttemptMetrics;

impl EndpointAttemptMetrics {
    pub fn increment_attempted(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointAttemptMetrics::increment_attempted(
            call.endpoint.name,
        );
    }

    pub fn increment_completed(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointAttemptMetrics::increment_completed(
            call.endpoint.name,
        );
    }
}

///
/// EndpointResultMetrics
/// Endpoint result metrics exposed to macro-expanded endpoints.
///

pub struct EndpointResultMetrics;

impl EndpointResultMetrics {
    pub fn increment_ok(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointResultMetrics::increment_ok(call.endpoint.name);
    }

    pub fn increment_err(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointResultMetrics::increment_err(call.endpoint.name);
    }
}
