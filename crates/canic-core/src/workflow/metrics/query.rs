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
    #[cfg(feature = "sharding")]
    use crate::ops::runtime::metrics::sharding::{
        ShardingMetricOperation, ShardingMetricOutcome, ShardingMetricReason, ShardingMetrics,
    };
    use crate::{
        ids::{AccessMetricKind, CanisterRole},
        ops::runtime::metrics::{
            self,
            access::AccessMetrics,
            auth::{
                AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface,
                AuthMetrics,
            },
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
            intent::{
                IntentMetricOperation, IntentMetricOutcome, IntentMetricReason,
                IntentMetricSurface, IntentMetrics,
            },
            platform_call::{
                PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
                PlatformCallMetricSurface, PlatformCallMetrics,
            },
            pool::{PoolMetricOperation, PoolMetricOutcome, PoolMetricReason, PoolMetrics},
            provisioning::{
                ProvisioningMetricOperation, ProvisioningMetricOutcome, ProvisioningMetricReason,
                ProvisioningMetrics,
            },
            replay::{
                ReplayMetricOperation, ReplayMetricOutcome, ReplayMetricReason, ReplayMetrics,
            },
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
    fn page_sorts_auth_metric_family_before_paginating() {
        metrics::reset_for_tests();

        AuthMetrics::record(
            AuthMetricSurface::Session,
            AuthMetricOperation::Session,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Created,
        );
        AuthMetrics::record(
            AuthMetricSurface::Attestation,
            AuthMetricOperation::Verify,
            AuthMetricOutcome::Failed,
            AuthMetricReason::UnknownKeyId,
        );

        assert_first_metric_labels(
            MetricsKind::Auth,
            ["attestation", "verify", "failed", "unknown_key_id"],
        );
    }

    #[test]
    fn page_sorts_new_multi_label_metric_families_before_paginating() {
        metrics::reset_for_tests();

        record_multi_label_sort_metrics();

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
        assert_first_metric_labels(MetricsKind::Replay, ["check", "completed", "fresh"]);
        assert_first_metric_labels(
            MetricsKind::Intent,
            ["call", "capacity_check", "failed", "capacity"],
        );
        assert_first_metric_labels(
            MetricsKind::PlatformCall,
            ["generic", "bounded_wait", "started", "ok"],
        );
        assert_first_metric_labels(
            MetricsKind::Provisioning,
            ["allocate", "app", "completed", "new_allocation"],
        );
        assert_first_metric_labels(
            MetricsKind::Scaling,
            ["bootstrap_pool", "skipped", "target_satisfied"],
        );
    }

    // Seed multi-label families used by sorting and pagination coverage.
    fn record_multi_label_sort_metrics() {
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
        record_replay_sort_metrics();
        record_intent_sort_metrics();
        record_platform_call_sort_metrics();
        record_provisioning_sort_metrics();
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
    }

    #[cfg(feature = "sharding")]
    #[test]
    fn page_sorts_sharding_metric_family_before_paginating() {
        metrics::reset_for_tests();

        ShardingMetrics::record(
            ShardingMetricOperation::PlanAssign,
            ShardingMetricOutcome::Completed,
            ShardingMetricReason::ExistingCapacity,
        );
        ShardingMetrics::record(
            ShardingMetricOperation::BootstrapPool,
            ShardingMetricOutcome::Skipped,
            ShardingMetricReason::TargetSatisfied,
        );

        assert_first_metric_labels(
            MetricsKind::Sharding,
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

    // Seed intent rows used by multi-family sorting coverage.
    fn record_intent_sort_metrics() {
        IntentMetrics::record(
            IntentMetricSurface::Pool,
            IntentMetricOperation::Reserve,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
        IntentMetrics::record(
            IntentMetricSurface::Call,
            IntentMetricOperation::CapacityCheck,
            IntentMetricOutcome::Failed,
            IntentMetricReason::Capacity,
        );
    }

    // Seed platform call rows used by multi-family sorting coverage.
    fn record_platform_call_sort_metrics() {
        PlatformCallMetrics::record(
            PlatformCallMetricSurface::Management,
            PlatformCallMetricMode::Update,
            PlatformCallMetricOutcome::Failed,
            PlatformCallMetricReason::Infra,
        );
        PlatformCallMetrics::record(
            PlatformCallMetricSurface::Generic,
            PlatformCallMetricMode::BoundedWait,
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
    }

    // Seed provisioning rows used by multi-family sorting coverage.
    fn record_provisioning_sort_metrics() {
        ProvisioningMetrics::record(
            ProvisioningMetricOperation::Upgrade,
            &CanisterRole::new("worker"),
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::ManagementCall,
        );
        ProvisioningMetrics::record(
            ProvisioningMetricOperation::Allocate,
            &CanisterRole::new("app"),
            ProvisioningMetricOutcome::Completed,
            ProvisioningMetricReason::NewAllocation,
        );
    }

    // Seed replay rows used by multi-family sorting coverage.
    fn record_replay_sort_metrics() {
        ReplayMetrics::record(
            ReplayMetricOperation::Reserve,
            ReplayMetricOutcome::Failed,
            ReplayMetricReason::Capacity,
        );
        ReplayMetrics::record(
            ReplayMetricOperation::Check,
            ReplayMetricOutcome::Completed,
            ReplayMetricReason::Fresh,
        );
    }
}
