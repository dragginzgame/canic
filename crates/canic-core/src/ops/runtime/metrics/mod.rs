use crate::{
    api::EndpointCall,
    cdk::mgmt::HttpMethod,
    storage::metrics::{
        access::{AccessMetricKey, AccessMetricKind, AccessMetrics as ModelAccessMetrics},
        endpoint::{
            EndpointAttemptCounts, EndpointAttemptMetrics as ModelEndpointAttemptMetrics,
            EndpointResultCounts, EndpointResultMetrics as ModelEndpointResultMetrics,
        },
        http::{HttpMethodKind, HttpMetricKey, HttpMetrics},
        icc::{IccMetricKey, IccMetrics},
        system::{SystemMetricKind, SystemMetrics},
        timer::{TimerMetricKey, TimerMetrics},
    },
};

///
/// SystemMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct SystemMetricsSnapshot {
    pub entries: Vec<(SystemMetricKind, u64)>,
}

///
/// HttpMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct HttpMetricsSnapshot {
    pub entries: Vec<(HttpMetricKey, u64)>,
}

///
/// IccMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct IccMetricsSnapshot {
    pub entries: Vec<(IccMetricKey, u64)>,
}

///
/// TimerMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct TimerMetricsSnapshot {
    pub entries: Vec<(TimerMetricKey, u64)>,
}

///
/// AccessMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct AccessMetricsSnapshot {
    pub entries: Vec<(AccessMetricKey, u64)>,
}

///
/// EndpointHealthSnapshot
///

#[derive(Clone)]
pub struct EndpointHealthSnapshot {
    pub attempts: Vec<(&'static str, EndpointAttemptCounts)>,
    pub results: Vec<(&'static str, EndpointResultCounts)>,
    pub access: Vec<(AccessMetricKey, u64)>,
}

///
/// AccessMetrics
///

pub struct AccessMetrics;

impl AccessMetrics {
    pub fn increment(call: EndpointCall, kind: AccessMetricKind) {
        ModelAccessMetrics::increment(call.endpoint.name, kind);
    }
}

///
/// EndpointAttemptMetrics
///

pub struct EndpointAttemptMetrics;

impl EndpointAttemptMetrics {
    pub fn increment_attempted(call: EndpointCall) {
        ModelEndpointAttemptMetrics::increment_attempted(call.endpoint.name);
    }

    pub fn increment_completed(call: EndpointCall) {
        ModelEndpointAttemptMetrics::increment_completed(call.endpoint.name);
    }
}

///
/// EndpointResultMetrics
///

pub struct EndpointResultMetrics;

impl EndpointResultMetrics {
    pub fn increment_ok(call: EndpointCall) {
        ModelEndpointResultMetrics::increment_ok(call.endpoint.name);
    }

    pub fn increment_err(call: EndpointCall) {
        ModelEndpointResultMetrics::increment_err(call.endpoint.name);
    }
}

///
/// MetricsOps
/// Read-side façade over volatile metrics state.
///

pub struct MetricsOps;

impl MetricsOps {
    /// System-level action counters snapshot.
    #[must_use]
    pub fn system_snapshot() -> SystemMetricsSnapshot {
        let entries = SystemMetrics::export_raw().into_iter().collect();
        SystemMetricsSnapshot { entries }
    }

    /// HTTP outcall counters snapshot.
    #[must_use]
    pub fn http_snapshot() -> HttpMetricsSnapshot {
        let entries = HttpMetrics::export_raw().into_iter().collect();
        HttpMetricsSnapshot { entries }
    }

    /// Inter-canister call counters snapshot.
    #[must_use]
    pub fn icc_snapshot() -> IccMetricsSnapshot {
        let entries = IccMetrics::export_raw().into_iter().collect();
        IccMetricsSnapshot { entries }
    }

    /// Timer execution counters snapshot.
    #[must_use]
    pub fn timer_snapshot() -> TimerMetricsSnapshot {
        let entries = TimerMetrics::export_raw().into_iter().collect();
        TimerMetricsSnapshot { entries }
    }

    /// Access-denial counters snapshot.
    #[must_use]
    pub fn access_snapshot() -> AccessMetricsSnapshot {
        let entries = ModelAccessMetrics::export_raw().into_iter().collect();
        AccessMetricsSnapshot { entries }
    }

    /// Endpoint health snapshot (attempts + denials + results).
    #[must_use]
    pub fn endpoint_health_snapshot() -> EndpointHealthSnapshot {
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
}

/// Record a single HTTP outcall for system metrics.
pub fn record_http_outcall() {
    SystemMetrics::increment(SystemMetricKind::HttpOutcall);
}

/// Record an HTTP outcall with label normalization.
pub fn record_http_request(method: HttpMethod, url: &str, label: Option<&str>) {
    let kind = http_method_to_kind(method);
    let label = label.map_or_else(|| normalize_http_label(url, label), str::to_string);

    HttpMetrics::increment(kind, &label);
}

#[must_use]
pub fn normalize_http_label(url: &str, label: Option<&str>) -> String {
    if let Some(label) = label {
        return label.to_string();
    }

    let without_fragment = url.split('#').next().unwrap_or(url);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);

    let trimmed = without_query.trim();
    if trimmed.is_empty() {
        url.to_string()
    } else {
        trimmed.to_string()
    }
}

const fn http_method_to_kind(method: HttpMethod) -> HttpMethodKind {
    match method {
        HttpMethod::GET => HttpMethodKind::Get,
        HttpMethod::POST => HttpMethodKind::Post,
        HttpMethod::HEAD => HttpMethodKind::Head,
    }
}
