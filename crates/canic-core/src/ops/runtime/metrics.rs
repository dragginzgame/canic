pub use crate::dto::metrics::{
    access::{AccessMetricEntry, AccessMetricKind},
    endpoint::EndpointHealthView,
    http::HttpMetricEntry,
    icc::IccMetricEntry,
    system::SystemMetricEntry,
    timer::TimerMetricEntry,
};
use crate::{
    api::EndpointCall,
    dto::page::{Page, PageRequest},
    model::metrics::{
        access::AccessMetrics as ModelAccessMetrics,
        endpoint::{
            EndpointAttemptMetrics as ModelEndpointAttemptMetrics,
            EndpointResultMetrics as ModelEndpointResultMetrics,
        },
        http::HttpMetrics,
        icc::IccMetrics,
        system::{SystemMetricKind, SystemMetrics},
        timer::TimerMetrics,
    },
    ops::{
        adapter::metrics::{
            access::{access_metric_kind_from_view, access_metrics_to_view},
            endpoint::endpoint_health_to_view,
            http::http_metrics_to_view,
            icc::icc_metrics_to_view,
            system::system_metrics_to_view,
            timer::timer_metrics_to_view,
        },
        view::paginate_vec,
    },
};

pub type SystemMetricsSnapshot = Vec<SystemMetricEntry>;

pub struct AccessMetrics;

impl AccessMetrics {
    pub fn increment(call: EndpointCall, kind: AccessMetricKind) {
        let model_kind = access_metric_kind_from_view(kind);
        ModelAccessMetrics::increment(call.endpoint.name, model_kind);
    }
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

///
/// MetricsOps
/// Read-side façade over volatile metrics state.
///
pub struct MetricsOps;

impl MetricsOps {
    /// System-level action counters.
    #[must_use]
    pub fn system_page(request: PageRequest) -> Page<SystemMetricEntry> {
        let raw = SystemMetrics::export_raw();
        let mut entries = system_metrics_to_view(raw);

        entries.sort_by(|a, b| a.kind.cmp(&b.kind));

        paginate_vec(entries, request)
    }

    /// System-level action counters without pagination.
    #[must_use]
    pub fn system_snapshot() -> SystemMetricsSnapshot {
        let raw = SystemMetrics::export_raw();
        let mut entries = system_metrics_to_view(raw);

        entries.sort_by(|a, b| a.kind.cmp(&b.kind));

        entries
    }

    /// HTTP outcall counters.
    #[must_use]
    pub fn http_page(request: PageRequest) -> Page<HttpMetricEntry> {
        let raw = HttpMetrics::export_raw();
        let mut entries = http_metrics_to_view(raw);

        entries.sort_by(|a, b| a.method.cmp(&b.method).then_with(|| a.label.cmp(&b.label)));

        paginate_vec(entries, request)
    }

    /// Inter-canister call counters.
    #[must_use]
    pub fn icc_page(request: PageRequest) -> Page<IccMetricEntry> {
        let raw = IccMetrics::export_raw();
        let mut entries = icc_metrics_to_view(raw);

        entries.sort_by(|a, b| {
            a.target
                .as_slice()
                .cmp(b.target.as_slice())
                .then_with(|| a.method.cmp(&b.method))
        });

        paginate_vec(entries, request)
    }

    /// Timer execution counters.
    #[must_use]
    pub fn timer_page(request: PageRequest) -> Page<TimerMetricEntry> {
        let raw = TimerMetrics::export_raw();
        let mut entries = timer_metrics_to_view(raw);

        entries.sort_by(|a, b| {
            a.mode
                .cmp(&b.mode)
                .then_with(|| a.delay_ms.cmp(&b.delay_ms))
                .then_with(|| a.label.cmp(&b.label))
        });

        paginate_vec(entries, request)
    }

    /// Access-denial counters.
    #[must_use]
    pub fn access_page(request: PageRequest) -> Page<AccessMetricEntry> {
        let raw = ModelAccessMetrics::export_raw();
        let mut entries = access_metrics_to_view(raw);

        entries.sort_by(|a, b| {
            a.endpoint
                .cmp(&b.endpoint)
                .then_with(|| a.kind.cmp(&b.kind))
        });

        paginate_vec(entries, request)
    }

    /// Derived endpoint health view (attempts + denials + results).
    #[must_use]
    pub fn endpoint_health_page(
        request: PageRequest,
        exclude_endpoint: Option<&str>,
    ) -> Page<EndpointHealthView> {
        let attempts = ModelEndpointAttemptMetrics::export_raw();
        let results = ModelEndpointResultMetrics::export_raw();
        let access = ModelAccessMetrics::export_raw();

        let mut entries = endpoint_health_to_view(attempts, results, access, exclude_endpoint);

        entries.sort_by(|a, b| a.endpoint.cmp(&b.endpoint));

        paginate_vec(entries, request)
    }

    #[must_use]
    pub fn endpoint_health_page_excluding(
        request: PageRequest,
        exclude_endpoint: Option<&str>,
    ) -> Page<EndpointHealthView> {
        Self::endpoint_health_page(request, exclude_endpoint)
    }
}

/// Record a single HTTP outcall for system metrics.
pub fn record_http_outcall() {
    SystemMetrics::increment(SystemMetricKind::HttpOutcall);
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
