pub use crate::model::metrics::{MetricEntry, MetricKind, MetricsSnapshot, MetricsState};

///
/// MetricsOps
/// Thin ops-layer facade over volatile metrics state.
///

pub struct MetricsOps;

impl MetricsOps {
    /// Increment a metric counter.
    pub fn record(kind: MetricKind) {
        MetricsState::increment(kind);
    }

    /// Export the current metrics snapshot.
    #[must_use]
    pub fn snapshot() -> MetricsSnapshot {
        MetricsState::snapshot()
    }
}
