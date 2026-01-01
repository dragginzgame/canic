use crate::{
    dto::{
        metrics::{
            AccessMetricEntry, EndpointHealthView, HttpMetricEntry, IccMetricEntry,
            SystemMetricEntry, TimerMetricEntry,
        },
        page::{Page, PageRequest},
    },
    ops::{perf::PerfOps, runtime::metrics::MetricsOps},
    perf::PerfEntry,
};

pub(crate) fn metrics_system_snapshot() -> Vec<SystemMetricEntry> {
    MetricsOps::system_snapshot()
}

pub(crate) fn metrics_icc_page(page: PageRequest) -> Page<IccMetricEntry> {
    MetricsOps::icc_page(page)
}

pub(crate) fn metrics_http_page(page: PageRequest) -> Page<HttpMetricEntry> {
    MetricsOps::http_page(page)
}

pub(crate) fn metrics_timer_page(page: PageRequest) -> Page<TimerMetricEntry> {
    MetricsOps::timer_page(page)
}

pub(crate) fn metrics_access_page(page: PageRequest) -> Page<AccessMetricEntry> {
    MetricsOps::access_page(page)
}

pub(crate) fn metrics_perf_page(page: PageRequest) -> Page<PerfEntry> {
    PerfOps::snapshot(page)
}

pub(crate) fn metrics_endpoint_health_page(
    page: PageRequest,
    exclude_endpoint: Option<&str>,
) -> Page<EndpointHealthView> {
    MetricsOps::endpoint_health_page_excluding(page, exclude_endpoint)
}
