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
/// MetricsApi
///

pub struct MetricsApi;

impl MetricsApi {
    #[must_use]
    pub fn system() -> Vec<SystemMetricEntry> {
        workflow::metrics::query::MetricsQuery::system_snapshot()
    }

    #[must_use]
    pub fn icc(page: PageRequest) -> Page<IccMetricEntry> {
        workflow::metrics::query::MetricsQuery::icc_page(page)
    }

    #[must_use]
    pub fn http(page: PageRequest) -> Page<HttpMetricEntry> {
        workflow::metrics::query::MetricsQuery::http_page(page)
    }

    #[must_use]
    pub fn timer(page: PageRequest) -> Page<TimerMetricEntry> {
        workflow::metrics::query::MetricsQuery::timer_page(page)
    }

    #[must_use]
    pub fn access(page: PageRequest) -> Page<AccessMetricEntry> {
        workflow::metrics::query::MetricsQuery::access_page(page)
    }

    #[must_use]
    pub fn perf(page: PageRequest) -> Page<PerfEntry> {
        workflow::metrics::query::MetricsQuery::perf_page(page)
    }

    #[must_use]
    pub fn endpoint_health(page: PageRequest) -> Page<EndpointHealthView> {
        workflow::metrics::query::MetricsQuery::endpoint_health_page(
            page,
            Some(protocol::CANIC_METRICS_ENDPOINT_HEALTH),
        )
    }
}
