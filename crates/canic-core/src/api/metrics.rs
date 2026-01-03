use crate::{
    dto::{
        metrics::{
            AccessMetricEntry, EndpointHealthView, HttpMetricEntry, IccMetricEntry,
            SystemMetricEntry, TimerMetricEntry,
        },
        page::{Page, PageRequest},
    },
    perf::PerfEntry,
    workflow,
};

///
/// Metrics API
///

#[must_use]
pub fn metrics_system() -> Vec<SystemMetricEntry> {
    workflow::metrics::query::metrics_system_snapshot()
}

#[must_use]
pub fn metrics_icc(page: PageRequest) -> Page<IccMetricEntry> {
    workflow::metrics::query::metrics_icc_page(page)
}

#[must_use]
pub fn metrics_http(page: PageRequest) -> Page<HttpMetricEntry> {
    workflow::metrics::query::metrics_http_page(page)
}

#[must_use]
pub fn metrics_timer(page: PageRequest) -> Page<TimerMetricEntry> {
    workflow::metrics::query::metrics_timer_page(page)
}

#[must_use]
pub fn metrics_access(page: PageRequest) -> Page<AccessMetricEntry> {
    workflow::metrics::query::metrics_access_page(page)
}

#[must_use]
pub fn metrics_perf(page: PageRequest) -> Page<PerfEntry> {
    workflow::metrics::query::metrics_perf_page(page)
}

#[must_use]
pub fn metrics_endpoint_health(page: PageRequest) -> Page<EndpointHealthView> {
    workflow::metrics::query::metrics_endpoint_health_page(
        page,
        Some("canic_metrics_endpoint_health"),
    )
}
