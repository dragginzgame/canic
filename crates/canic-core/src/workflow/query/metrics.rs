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
    workflow::query::{
        metrics_adapter::{
            access_metrics_to_view, endpoint_health_to_view, http_metrics_to_view,
            icc_metrics_to_view, system_metrics_to_view, timer_metrics_to_view,
        },
        paginate::paginate_vec,
    },
};

pub(crate) fn metrics_system_snapshot() -> Vec<SystemMetricEntry> {
    let snapshot = MetricsOps::system_snapshot();
    let mut entries = system_metrics_to_view(snapshot.entries);

    entries.sort_by(|a, b| a.kind.cmp(&b.kind));

    entries
}

pub(crate) fn metrics_icc_page(page: PageRequest) -> Page<IccMetricEntry> {
    let snapshot = MetricsOps::icc_snapshot();
    let mut entries = icc_metrics_to_view(snapshot.entries);

    entries.sort_by(|a, b| {
        a.target
            .as_slice()
            .cmp(b.target.as_slice())
            .then_with(|| a.method.cmp(&b.method))
    });

    paginate_vec(entries, page)
}

pub(crate) fn metrics_http_page(page: PageRequest) -> Page<HttpMetricEntry> {
    let snapshot = MetricsOps::http_snapshot();
    let mut entries = http_metrics_to_view(snapshot.entries);

    entries.sort_by(|a, b| a.method.cmp(&b.method).then_with(|| a.label.cmp(&b.label)));

    paginate_vec(entries, page)
}

pub(crate) fn metrics_timer_page(page: PageRequest) -> Page<TimerMetricEntry> {
    let snapshot = MetricsOps::timer_snapshot();
    let mut entries = timer_metrics_to_view(snapshot.entries);

    entries.sort_by(|a, b| {
        a.mode
            .cmp(&b.mode)
            .then_with(|| a.delay_ms.cmp(&b.delay_ms))
            .then_with(|| a.label.cmp(&b.label))
    });

    paginate_vec(entries, page)
}

pub(crate) fn metrics_access_page(page: PageRequest) -> Page<AccessMetricEntry> {
    let snapshot = MetricsOps::access_snapshot();
    let mut entries = access_metrics_to_view(snapshot.entries);

    entries.sort_by(|a, b| {
        a.endpoint
            .cmp(&b.endpoint)
            .then_with(|| a.kind.cmp(&b.kind))
    });

    paginate_vec(entries, page)
}

pub(crate) fn metrics_perf_page(page: PageRequest) -> Page<PerfEntry> {
    let snapshot = PerfOps::snapshot();
    paginate_vec(snapshot.entries, page)
}

pub(crate) fn metrics_endpoint_health_page(
    page: PageRequest,
    exclude_endpoint: Option<&str>,
) -> Page<EndpointHealthView> {
    let snapshot = MetricsOps::endpoint_health_snapshot();
    let mut entries = endpoint_health_to_view(
        snapshot.attempts,
        snapshot.results,
        snapshot.access,
        exclude_endpoint,
    );

    entries.sort_by(|a, b| a.endpoint.cmp(&b.endpoint));

    paginate_vec(entries, page)
}
