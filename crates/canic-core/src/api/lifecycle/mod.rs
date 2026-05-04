pub mod nonroot;
pub mod root;

pub mod metrics {
    use crate::ops::runtime::metrics::lifecycle::LifecycleMetrics;

    pub use crate::ops::runtime::metrics::lifecycle::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricStage,
    };

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
}
