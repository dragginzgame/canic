use crate::{
    dto::{
        metrics::{MetricEntry, MetricsKind, MetricsRequest, MetricsResponse},
        page::{Page, PageRequest},
    },
    ops::runtime::metrics::MetricsOps,
    workflow::view::paginate::paginate_vec,
};

///
/// MetricsQuery
///
/// Read-only query façade over metric snapshots.
/// Responsible for mapping, sorting, and pagination only.
///

pub struct MetricsQuery;

impl MetricsQuery {
    #[must_use]
    pub fn dispatch(req: MetricsRequest) -> MetricsResponse {
        let entries = Self::page(req.kind, req.page);

        MetricsResponse { entries }
    }

    #[must_use]
    pub fn page(kind: MetricsKind, page: PageRequest) -> Page<MetricEntry> {
        let mut entries = MetricsOps::entries(kind);
        entries.sort_by(|a, b| {
            a.labels
                .cmp(&b.labels)
                .then_with(|| a.principal.cmp(&b.principal))
                .then_with(|| a.count.cmp(&b.count))
                .then_with(|| a.value_u64.cmp(&b.value_u64))
                .then_with(|| a.value_u128.cmp(&b.value_u128))
        });

        paginate_vec(entries, page)
    }
}
