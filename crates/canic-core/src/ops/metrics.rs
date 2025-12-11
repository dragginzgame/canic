pub use crate::model::metrics::{
    HttpMetricEntry, HttpMetrics, HttpMetricsSnapshot, IccMetricEntry, IccMetrics,
    IccMetricsSnapshot, MetricsReport, SystemMetricEntry, SystemMetricKind, SystemMetrics,
    SystemMetricsSnapshot,
};

///
/// MetricsOps
/// Thin ops-layer facade over volatile metrics state.
///

pub struct MetricsOps;

impl MetricsOps {
    /// Export the current metrics snapshot.
    #[must_use]
    pub fn system_snapshot() -> SystemMetricsSnapshot {
        SystemMetrics::snapshot()
    }

    /// Export the current HTTP metrics snapshot.
    #[must_use]
    pub fn http_snapshot() -> HttpMetricsSnapshot {
        HttpMetrics::snapshot()
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
            http: Self::http_snapshot(),
            icc: Self::icc_snapshot(),
        }
    }
}
