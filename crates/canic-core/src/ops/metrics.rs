pub use crate::model::metrics::{
    IccMetricEntry, IccMetrics, IccMetricsSnapshot, MetricEntry, MetricKind, MetricsReport,
    MetricsSnapshot, MetricsState, SystemMetrics,
};

///
/// MetricsOps
/// Thin ops-layer facade over volatile metrics state.
///

pub struct MetricsOps;

impl MetricsOps {
    /// Increment a metric counter.
    pub fn record(kind: MetricKind) {
        SystemMetrics::record(kind);
    }

    /// Export the current metrics snapshot.
    #[must_use]
    pub fn system_snapshot() -> MetricsSnapshot {
        SystemMetrics::snapshot()
    }

    /// Export the current ICC metrics snapshot.
    #[must_use]
    pub fn icc_snapshot() -> IccMetricsSnapshot {
        IccMetrics::snapshot()
    }

    /// Export combined metrics (actions + ICC).
    #[must_use]
    pub fn report() -> MetricsReport {
        MetricsReport {
            system: Self::system_snapshot(),
            icc: Self::icc_snapshot(),
        }
    }
}
