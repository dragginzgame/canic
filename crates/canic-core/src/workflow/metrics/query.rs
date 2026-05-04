use crate::{
    dto::{
        metrics::{MetricEntry, MetricsKind, QueryPerfSample},
        page::{Page, PageRequest},
    },
    ops::runtime::metrics,
    perf,
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
    /// Return one sorted, paginated metrics family snapshot.
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

    /// Wrap a query result with the current same-call local instruction count.
    #[must_use]
    pub fn sample_query<T>(value: T) -> QueryPerfSample<T> {
        QueryPerfSample {
            value,
            local_instructions: perf::perf_counter(),
        }
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::{AccessMetricKind, CanisterRole},
        ops::runtime::metrics::{
            self,
            access::AccessMetrics,
            canister_ops::{
                CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
                CanisterOpsMetrics,
            },
            cascade::{
                CascadeMetricOperation, CascadeMetricOutcome, CascadeMetricReason,
                CascadeMetricSnapshot, CascadeMetrics,
            },
            directory::{
                DirectoryMetricOperation, DirectoryMetricOutcome, DirectoryMetricReason,
                DirectoryMetrics,
            },
            pool::{PoolMetricOperation, PoolMetricOutcome, PoolMetricReason, PoolMetrics},
            scaling::{
                ScalingMetricOperation, ScalingMetricOutcome, ScalingMetricReason, ScalingMetrics,
            },
            wasm_store::{
                WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason,
                WasmStoreMetricSource, WasmStoreMetrics,
            },
        },
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

    #[test]
    fn page_sorts_new_multi_label_metric_families_before_paginating() {
        metrics::reset_for_tests();

        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Upgrade,
            &CanisterRole::new("worker"),
            CanisterOpsMetricOutcome::Completed,
            CanisterOpsMetricReason::Ok,
        );
        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Create,
            &CanisterRole::new("app"),
            CanisterOpsMetricOutcome::Started,
            CanisterOpsMetricReason::Ok,
        );
        WasmStoreMetrics::record(
            WasmStoreMetricOperation::SourceResolve,
            WasmStoreMetricSource::Store,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Ok,
        );
        WasmStoreMetrics::record(
            WasmStoreMetricOperation::ChunkUpload,
            WasmStoreMetricSource::Bootstrap,
            WasmStoreMetricOutcome::Skipped,
            WasmStoreMetricReason::CacheHit,
        );
        CascadeMetrics::record(
            CascadeMetricOperation::RootFanout,
            CascadeMetricSnapshot::Topology,
            CascadeMetricOutcome::Completed,
            CascadeMetricReason::Ok,
        );
        CascadeMetrics::record(
            CascadeMetricOperation::ChildSend,
            CascadeMetricSnapshot::State,
            CascadeMetricOutcome::Failed,
            CascadeMetricReason::SendFailed,
        );
        DirectoryMetrics::record(
            DirectoryMetricOperation::Resolve,
            DirectoryMetricOutcome::Started,
            DirectoryMetricReason::Ok,
        );
        DirectoryMetrics::record(
            DirectoryMetricOperation::Classify,
            DirectoryMetricOutcome::Completed,
            DirectoryMetricReason::PendingFresh,
        );
        PoolMetrics::record(
            PoolMetricOperation::ImportQueued,
            PoolMetricOutcome::Skipped,
            PoolMetricReason::AlreadyPresent,
        );
        PoolMetrics::record(
            PoolMetricOperation::CreateEmpty,
            PoolMetricOutcome::Completed,
            PoolMetricReason::Ok,
        );
        ScalingMetrics::record(
            ScalingMetricOperation::CreateWorker,
            ScalingMetricOutcome::Completed,
            ScalingMetricReason::Ok,
        );
        ScalingMetrics::record(
            ScalingMetricOperation::BootstrapPool,
            ScalingMetricOutcome::Skipped,
            ScalingMetricReason::TargetSatisfied,
        );

        assert_first_metric_labels(MetricsKind::CanisterOps, ["create", "app", "started", "ok"]);
        assert_first_metric_labels(
            MetricsKind::WasmStore,
            ["chunk_upload", "bootstrap", "skipped", "cache_hit"],
        );
        assert_first_metric_labels(
            MetricsKind::Cascade,
            ["child_send", "state", "failed", "send_failed"],
        );
        assert_first_metric_labels(
            MetricsKind::Directory,
            ["classify", "completed", "pending_fresh"],
        );
        assert_first_metric_labels(MetricsKind::Pool, ["create_empty", "completed", "ok"]);
        assert_first_metric_labels(
            MetricsKind::Scaling,
            ["bootstrap_pool", "skipped", "target_satisfied"],
        );
    }

    #[test]
    fn sample_query_returns_value_and_current_counter() {
        let sample = MetricsQuery::sample_query("ok");

        assert_eq!(sample.value, "ok");
        assert_eq!(sample.local_instructions, 0);
    }

    // Assert that pagination sees the sorted first row for one metric family.
    fn assert_first_metric_labels<const N: usize>(kind: MetricsKind, expected: [&str; N]) {
        let page = MetricsQuery::page(
            kind,
            PageRequest {
                limit: 1,
                offset: 0,
            },
        );

        assert_eq!(page.total, 2);
        assert_eq!(page.entries[0].labels, expected);
    }
}
