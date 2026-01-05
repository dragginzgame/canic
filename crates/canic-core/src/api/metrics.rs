use crate::{
    dto::{
        metrics::{
            AccessMetricEntry, EndpointHealthView, HttpMetricEntry, IccMetricEntry,
            SystemMetricEntry, TimerMetricEntry,
        },
        page::{Page, PageRequest},
    },
    perf::PerfEntry,
    protocol, workflow,
};

///
/// Metrics API
///

#[must_use]
pub fn metrics_system() -> Vec<SystemMetricEntry> {
    workflow::metrics::query::MetricsQuery::metrics_system_snapshot()
}

#[must_use]
pub fn metrics_icc(page: PageRequest) -> Page<IccMetricEntry> {
    workflow::metrics::query::MetricsQuery::metrics_icc_page(page)
}

#[must_use]
pub fn metrics_http(page: PageRequest) -> Page<HttpMetricEntry> {
    workflow::metrics::query::MetricsQuery::metrics_http_page(page)
}

#[must_use]
pub fn metrics_timer(page: PageRequest) -> Page<TimerMetricEntry> {
    workflow::metrics::query::MetricsQuery::metrics_timer_page(page)
}

#[must_use]
pub fn metrics_access(page: PageRequest) -> Page<AccessMetricEntry> {
    workflow::metrics::query::MetricsQuery::metrics_access_page(page)
}

#[must_use]
pub fn metrics_perf(page: PageRequest) -> Page<PerfEntry> {
    workflow::metrics::query::MetricsQuery::metrics_perf_page(page)
}

#[must_use]
pub fn metrics_endpoint_health(page: PageRequest) -> Page<EndpointHealthView> {
    workflow::metrics::query::MetricsQuery::metrics_endpoint_health_page(
        page,
        Some(protocol::CANIC_METRICS_ENDPOINT_HEALTH),
    )
}
