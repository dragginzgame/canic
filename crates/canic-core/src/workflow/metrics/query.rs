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
    workflow::{metrics::mapper::MetricsMapper, view::paginate::paginate_vec},
};

///
/// MetricsQuery
///

pub struct MetricsQuery;

impl MetricsQuery {
    pub fn system_snapshot() -> Vec<SystemMetricEntry> {
        let snapshot = MetricsOps::system_snapshot();
        let mut entries = MetricsMapper::system_metrics_to_view(snapshot.entries);

        entries.sort_by(|a, b| a.kind.cmp(&b.kind));

        entries
    }

    pub fn icc_page(page: PageRequest) -> Page<IccMetricEntry> {
        let snapshot = MetricsOps::icc_snapshot();
        let mut entries = MetricsMapper::icc_metrics_to_view(snapshot.entries);

        entries.sort_by(|a, b| {
            a.target
                .as_slice()
                .cmp(b.target.as_slice())
                .then_with(|| a.method.cmp(&b.method))
        });

        paginate_vec(entries, page)
    }

    pub fn http_page(page: PageRequest) -> Page<HttpMetricEntry> {
        let snapshot = MetricsOps::http_snapshot();
        let mut entries = MetricsMapper::http_metrics_to_view(snapshot.entries);

        entries.sort_by(|a, b| a.method.cmp(&b.method).then_with(|| a.label.cmp(&b.label)));

        paginate_vec(entries, page)
    }

    pub fn timer_page(page: PageRequest) -> Page<TimerMetricEntry> {
        let snapshot = MetricsOps::timer_snapshot();
        let mut entries = MetricsMapper::timer_metrics_to_view(snapshot.entries);

        entries.sort_by(|a, b| {
            a.mode
                .cmp(&b.mode)
                .then_with(|| a.delay_ms.cmp(&b.delay_ms))
                .then_with(|| a.label.cmp(&b.label))
        });

        paginate_vec(entries, page)
    }

    pub fn access_page(page: PageRequest) -> Page<AccessMetricEntry> {
        let snapshot = MetricsOps::access_snapshot();
        let mut entries = MetricsMapper::access_metrics_to_view(snapshot.entries);

        entries.sort_by(|a, b| {
            a.endpoint
                .cmp(&b.endpoint)
                .then_with(|| a.kind.cmp(&b.kind))
        });

        paginate_vec(entries, page)
    }

    pub fn perf_page(page: PageRequest) -> Page<PerfEntry> {
        let snapshot = PerfOps::snapshot();
        paginate_vec(snapshot.entries, page)
    }

    pub fn endpoint_health_page(
        page: PageRequest,
        exclude_endpoint: Option<&str>,
    ) -> Page<EndpointHealthView> {
        let snapshot = MetricsOps::endpoint_health_snapshot();
        let mut entries = MetricsMapper::endpoint_health_to_view(
            snapshot.attempts,
            snapshot.results,
            snapshot.access,
            exclude_endpoint,
        );

        entries.sort_by(|a, b| a.endpoint.cmp(&b.endpoint));

        paginate_vec(entries, page)
    }
}
