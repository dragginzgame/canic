pub mod nonroot;
pub mod root;

pub mod metrics {
    use crate::{
        ids::CanisterRole,
        ops::runtime::metrics::{
            canister_ops::CanisterOpsMetrics, lifecycle::LifecycleMetrics,
            wasm_store::WasmStoreMetrics,
        },
    };

    pub use crate::ops::runtime::metrics::canister_ops::{
        CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
    };

    pub use crate::ops::runtime::metrics::lifecycle::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricStage,
    };

    pub use crate::ops::runtime::metrics::wasm_store::{
        WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason,
        WasmStoreMetricSource,
    };

    ///
    /// CanisterOpsMetricsApi
    ///

    pub struct CanisterOpsMetricsApi;

    impl CanisterOpsMetricsApi {
        /// Record one canister operation metric point for a concrete role.
        pub fn record(
            operation: CanisterOpsMetricOperation,
            role: &CanisterRole,
            outcome: CanisterOpsMetricOutcome,
            reason: CanisterOpsMetricReason,
        ) {
            CanisterOpsMetrics::record(operation, role, outcome, reason);
        }

        /// Record one canister operation metric point with no role context.
        pub fn record_unscoped(
            operation: CanisterOpsMetricOperation,
            outcome: CanisterOpsMetricOutcome,
            reason: CanisterOpsMetricReason,
        ) {
            CanisterOpsMetrics::record_unscoped(operation, outcome, reason);
        }

        /// Record one canister operation metric point when role lookup failed.
        pub fn record_unknown_role(
            operation: CanisterOpsMetricOperation,
            outcome: CanisterOpsMetricOutcome,
            reason: CanisterOpsMetricReason,
        ) {
            CanisterOpsMetrics::record_unknown_role(operation, outcome, reason);
        }
    }

    ///
    /// WasmStoreMetricsApi
    ///

    pub struct WasmStoreMetricsApi;

    impl WasmStoreMetricsApi {
        /// Record one wasm-store operation metric point.
        pub fn record(
            operation: WasmStoreMetricOperation,
            source: WasmStoreMetricSource,
            outcome: WasmStoreMetricOutcome,
            reason: WasmStoreMetricReason,
        ) {
            WasmStoreMetrics::record(operation, source, outcome, reason);
        }
    }

    ///
    /// LifecycleMetricsApi
    ///

    pub struct LifecycleMetricsApi;

    impl LifecycleMetricsApi {
        /// Record one lifecycle runtime or bootstrap metric point.
        pub fn record(
            phase: LifecycleMetricPhase,
            role: LifecycleMetricRole,
            stage: LifecycleMetricStage,
            outcome: LifecycleMetricOutcome,
        ) {
            LifecycleMetrics::record(phase, role, stage, outcome);
        }

        /// Record one lifecycle runtime metric point.
        pub fn record_runtime(
            phase: LifecycleMetricPhase,
            role: LifecycleMetricRole,
            outcome: LifecycleMetricOutcome,
        ) {
            Self::record(phase, role, LifecycleMetricStage::Runtime, outcome);
        }

        /// Record one lifecycle bootstrap metric point.
        pub fn record_bootstrap(
            phase: LifecycleMetricPhase,
            role: LifecycleMetricRole,
            outcome: LifecycleMetricOutcome,
        ) {
            Self::record(phase, role, LifecycleMetricStage::Bootstrap, outcome);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{
            CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
            CanisterOpsMetricsApi, LifecycleMetricOutcome, LifecycleMetricPhase,
            LifecycleMetricRole, LifecycleMetricsApi, WasmStoreMetricOperation,
            WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
            WasmStoreMetricsApi,
        };
        use crate::{
            dto::metrics::{MetricValue, MetricsKind},
            ids::CanisterRole,
            ops::runtime::metrics,
        };

        // Verify the facade records canister operation metrics with public labels.
        #[test]
        fn canister_ops_metrics_api_records_rows() {
            metrics::reset_for_tests();

            CanisterOpsMetricsApi::record(
                CanisterOpsMetricOperation::Install,
                &CanisterRole::new("app"),
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::MissingWasm,
            );

            let entries = metrics::entries(MetricsKind::CanisterOps);

            assert_count(&entries, &["install", "app", "failed", "missing_wasm"], 1);
        }

        // Verify the facade records wasm-store metrics with public labels.
        #[test]
        fn wasm_store_metrics_api_records_rows() {
            metrics::reset_for_tests();

            WasmStoreMetricsApi::record(
                WasmStoreMetricOperation::SourceResolve,
                WasmStoreMetricSource::Store,
                WasmStoreMetricOutcome::Failed,
                WasmStoreMetricReason::StoreCall,
            );

            let entries = metrics::entries(MetricsKind::WasmStore);

            assert_count(
                &entries,
                &["source_resolve", "store", "failed", "store_call"],
                1,
            );
        }

        // Verify the facade records both lifecycle metric stages with public labels.
        #[test]
        fn lifecycle_metrics_api_records_runtime_and_bootstrap_rows() {
            metrics::reset_for_tests();

            LifecycleMetricsApi::record_runtime(
                LifecycleMetricPhase::Init,
                LifecycleMetricRole::Root,
                LifecycleMetricOutcome::Completed,
            );
            LifecycleMetricsApi::record_bootstrap(
                LifecycleMetricPhase::PostUpgrade,
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Scheduled,
            );

            let entries = metrics::entries(MetricsKind::Lifecycle);

            assert_count(&entries, &["init", "root", "runtime", "completed"], 1);
            assert_count(
                &entries,
                &["post_upgrade", "nonroot", "bootstrap", "scheduled"],
                1,
            );
        }

        // Assert one metric row count by its stable public labels.
        fn assert_count(entries: &[crate::dto::metrics::MetricEntry], labels: &[&str], count: u64) {
            let entry = entries
                .iter()
                .find(|entry| {
                    entry
                        .labels
                        .iter()
                        .map(String::as_str)
                        .eq(labels.iter().copied())
                })
                .expect("metric entry should exist");

            assert!(matches!(&entry.value, MetricValue::Count(actual) if *actual == count));
        }
    }
}
