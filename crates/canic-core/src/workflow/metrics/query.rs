use crate::{
    dto::{
        metrics::{MetricEntry, MetricsKind},
        page::{Page, PageRequest},
    },
    ops::runtime::metrics,
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
    pub fn page(kind: MetricsKind, page: PageRequest) -> Page<MetricEntry> {
        let mut entries = metrics::entries(kind);
        entries.sort_by(|a, b| {
            a.labels
                .cmp(&b.labels)
                .then_with(|| a.principal.cmp(&b.principal))
        });

        paginate_vec(entries, page)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::AccessMetricKind,
        ops::runtime::metrics::{self, access::AccessMetrics},
    };

    #[test]
    fn page_sorts_metric_rows_before_paginating() {
        metrics::reset_for_tests();

        AccessMetrics::increment("zeta", AccessMetricKind::Auth, "caller_is_root");
        AccessMetrics::increment("alpha", AccessMetricKind::Guard, "app_allows_updates");

        let page = MetricsQuery::page(
            MetricsKind::Access,
            PageRequest {
                limit: 1,
                offset: 0,
            },
        );

        assert_eq!(page.total, 2);
        assert_eq!(
            page.entries[0].labels,
            ["alpha", "guard", "app_allows_updates"]
        );

        let page = MetricsQuery::page(
            MetricsKind::Access,
            PageRequest {
                limit: 1,
                offset: 1,
            },
        );

        assert_eq!(page.total, 2);
        assert_eq!(page.entries[0].labels, ["zeta", "auth", "caller_is_root"]);
    }
}
