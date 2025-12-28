use crate::{
    dto::{
        metrics::{
            AccessMetricEntry, EndpointHealthView, http::HttpMetricEntry, icc::IccMetricEntry,
            system::SystemMetricEntry, timer::TimerMetricEntry,
        },
        page::{Page, PageRequest},
    },
    model::metrics::{
        access::AccessMetrics,
        endpoint::{EndpointAttemptMetrics, EndpointResultMetrics},
        http::HttpMetrics,
        icc::IccMetrics,
        system::SystemMetrics,
        timer::TimerMetrics,
    },
    ops::adapter::metrics::{
        access::access_metrics_to_view, endpoint::endpoint_health_to_view,
        http::http_metrics_to_view, icc::icc_metrics_to_view, system::system_metrics_to_view,
        timer::timer_metrics_to_view,
    },
};

/// MetricsOps
/// Read-side façade over volatile metrics state.
pub struct MetricsOps;

impl MetricsOps {
    #[must_use]
    pub fn system_page(request: PageRequest) -> Page<SystemMetricEntry> {
        let raw = SystemMetrics::export_raw();
        let entries = system_metrics_to_view(raw);
        paginate_sorted(entries, request, |a, b| a.kind.cmp(&b.kind))
    }

    #[must_use]
    pub fn http_page(request: PageRequest) -> Page<HttpMetricEntry> {
        let raw = HttpMetrics::export_raw();
        let entries = http_metrics_to_view(raw);
        paginate_sorted(entries, request, |a, b| {
            a.method.cmp(&b.method).then_with(|| a.label.cmp(&b.label))
        })
    }

    #[must_use]
    pub fn icc_page(request: PageRequest) -> Page<IccMetricEntry> {
        let raw = IccMetrics::export_raw();
        let entries = icc_metrics_to_view(raw);
        paginate_sorted(entries, request, |a, b| {
            a.target
                .as_slice()
                .cmp(b.target.as_slice())
                .then_with(|| a.method.cmp(&b.method))
        })
    }

    #[must_use]
    pub fn timer_page(request: PageRequest) -> Page<TimerMetricEntry> {
        let raw = TimerMetrics::export_raw();
        let entries = timer_metrics_to_view(raw);
        paginate_sorted(entries, request, |a, b| {
            a.mode
                .cmp(&b.mode)
                .then_with(|| a.delay_ms.cmp(&b.delay_ms))
                .then_with(|| a.label.cmp(&b.label))
        })
    }

    #[must_use]
    pub fn access_page(request: PageRequest) -> Page<AccessMetricEntry> {
        let raw = AccessMetrics::export_raw();
        let entries = access_metrics_to_view(raw);
        paginate_sorted(entries, request, |a, b| {
            a.endpoint
                .cmp(&b.endpoint)
                .then_with(|| a.kind.cmp(&b.kind))
        })
    }

    /// Derived endpoint health view.
    #[must_use]
    pub fn endpoint_health_page(
        request: PageRequest,
        exclude_endpoint: Option<&str>,
    ) -> Page<EndpointHealthView> {
        let attempts = EndpointAttemptMetrics::export_raw();
        let results = EndpointResultMetrics::export_raw();
        let denied = AccessMetrics::export_raw();

        let entries = endpoint_health_to_view(attempts, results, denied, exclude_endpoint);

        paginate(entries, request)
    }
}
