//! Metrics endpoint surface for macro-generated entrypoints.

use crate::{
    PublicError,
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

pub fn canic_metrics_system() -> Result<Vec<SystemMetricEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_system_snapshot())
}

pub fn canic_metrics_icc(page: PageRequest) -> Result<Page<IccMetricEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_icc_page(page))
}

pub fn canic_metrics_http(page: PageRequest) -> Result<Page<HttpMetricEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_http_page(page))
}

pub fn canic_metrics_timer(page: PageRequest) -> Result<Page<TimerMetricEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_timer_page(page))
}

pub fn canic_metrics_access(page: PageRequest) -> Result<Page<AccessMetricEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_access_page(page))
}

pub fn canic_metrics_perf(page: PageRequest) -> Result<Page<PerfEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_perf_page(page))
}

pub fn canic_metrics_endpoint_health(
    page: PageRequest,
) -> Result<Page<EndpointHealthView>, PublicError> {
    Ok(workflow::metrics::query::metrics_endpoint_health_page(
        page,
        Some("canic_metrics_endpoint_health"),
    ))
}
