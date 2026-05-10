use canic::{
    api::metrics::MetricsQuery,
    dto::metrics::{MetricEntry, MetricsKind, QueryPerfSample},
    dto::page::PageRequest,
};

// Verify the public facade exposes query perf sampling without internal paths.
#[test]
fn metrics_query_sample_query_is_public_facade_usable() {
    let sample: QueryPerfSample<&str> = MetricsQuery::sample_query("ok");

    assert_eq!(sample.value, "ok");
    assert_eq!(sample.local_instructions, 0);
}

// Verify the public facade can still page metric rows through re-exported DTOs.
#[test]
fn metrics_query_page_is_public_facade_usable() {
    let page = MetricsQuery::page(
        MetricsKind::Security,
        PageRequest {
            limit: 10,
            offset: 0,
        },
    );

    let entries: Vec<MetricEntry> = page.entries;
    assert!(entries.is_empty());
}

// Verify all metric families are reachable through the public facade.
#[test]
fn all_metric_families_are_public_facade_usable() {
    for kind in [
        MetricsKind::Core,
        MetricsKind::Placement,
        MetricsKind::Platform,
        MetricsKind::Runtime,
        MetricsKind::Security,
        MetricsKind::Storage,
    ] {
        let page = MetricsQuery::page(
            kind,
            PageRequest {
                limit: 10,
                offset: 0,
            },
        );

        assert_eq!(page.total, 0);
        assert!(page.entries.is_empty());
    }
}
