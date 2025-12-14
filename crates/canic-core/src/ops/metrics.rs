pub use crate::model::metrics::{
    AccessMetricEntry, AccessMetricKind, AccessMetrics, AccessMetricsSnapshot, HttpMetricEntry,
    HttpMetrics, HttpMetricsSnapshot, IccMetricEntry, IccMetrics, IccMetricsSnapshot,
    SystemMetricEntry, SystemMetricKind, SystemMetrics, SystemMetricsSnapshot, TimerMetricEntry,
    TimerMetrics, TimerMetricsSnapshot,
};
use crate::types::PageRequest;
use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// MetricsOps
/// Thin ops-layer facade over volatile metrics state.
///

pub struct MetricsOps;

///
/// MetricsPageDto
/// Generic pagination envelope for metrics endpoints.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MetricsPageDto<T> {
    pub entries: Vec<T>,
    pub total: u64,
}

impl MetricsOps {
    /// Export the current metrics snapshot.
    #[must_use]
    pub fn system_snapshot() -> SystemMetricsSnapshot {
        let mut entries = SystemMetrics::snapshot();
        entries.sort_by(|a, b| a.kind.cmp(&b.kind));
        entries
    }

    /// Export the current HTTP metrics snapshot.
    #[must_use]
    pub fn http_snapshot() -> HttpMetricsSnapshot {
        HttpMetrics::snapshot()
    }

    /// Export the current HTTP metrics snapshot as a stable, paged view.
    #[must_use]
    pub fn http_page(request: PageRequest) -> MetricsPageDto<HttpMetricEntry> {
        let mut entries = Self::http_snapshot();
        entries.sort_by(|a, b| a.method.cmp(&b.method).then_with(|| a.url.cmp(&b.url)));
        paginate(entries, request)
    }

    /// Export the current ICC metrics snapshot.
    #[must_use]
    pub fn icc_snapshot() -> IccMetricsSnapshot {
        IccMetrics::snapshot()
    }

    /// Export the current ICC metrics snapshot as a stable, paged view.
    #[must_use]
    pub fn icc_page(request: PageRequest) -> MetricsPageDto<IccMetricEntry> {
        let mut entries = Self::icc_snapshot();
        entries.sort_by(|a, b| {
            a.target
                .as_slice()
                .cmp(b.target.as_slice())
                .then_with(|| a.method.cmp(&b.method))
        });
        paginate(entries, request)
    }

    /// Export the current timer metrics snapshot.
    #[must_use]
    pub fn timer_snapshot() -> TimerMetricsSnapshot {
        TimerMetrics::snapshot()
    }

    /// Export the current timer metrics snapshot as a stable, paged view.
    #[must_use]
    pub fn timer_page(request: PageRequest) -> MetricsPageDto<TimerMetricEntry> {
        let mut entries = Self::timer_snapshot();
        entries.sort_by(|a, b| {
            a.mode
                .cmp(&b.mode)
                .then_with(|| a.delay_ms.cmp(&b.delay_ms))
                .then_with(|| a.label.cmp(&b.label))
        });
        paginate(entries, request)
    }

    /// Export the current access metrics snapshot.
    #[must_use]
    pub fn access_snapshot() -> AccessMetricsSnapshot {
        AccessMetrics::snapshot()
    }

    /// Export the current access metrics snapshot as a stable, paged view.
    #[must_use]
    pub fn access_page(request: PageRequest) -> MetricsPageDto<AccessMetricEntry> {
        let mut entries = Self::access_snapshot();
        entries.sort_by(|a, b| {
            a.endpoint
                .cmp(&b.endpoint)
                .then_with(|| a.kind.cmp(&b.kind))
        });
        paginate(entries, request)
    }
}

// -----------------------------------------------------------------------------
// Pagination
// -----------------------------------------------------------------------------

#[must_use]
fn paginate<T>(entries: Vec<T>, request: PageRequest) -> MetricsPageDto<T> {
    let request = request.clamped();
    let total = entries.len() as u64;
    let (start, end) = pagination_bounds(total, request);

    let entries = entries.into_iter().skip(start).take(end - start).collect();

    MetricsPageDto { entries, total }
}

#[allow(clippy::cast_possible_truncation)]
fn pagination_bounds(total: u64, request: PageRequest) -> (usize, usize) {
    let start = request.offset.min(total);
    let end = request.offset.saturating_add(request.limit).min(total);

    let start = start as usize;
    let end = end as usize;

    (start, end)
}
