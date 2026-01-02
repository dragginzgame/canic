use crate::storage::metrics::access::{
    AccessMetricKey, AccessMetricKind, AccessMetrics as ModelAccessMetrics,
};

#[derive(Clone, Debug)]
pub struct AccessMetricsSnapshot {
    pub entries: Vec<(AccessMetricKey, u64)>,
}

pub struct AccessMetrics;

impl AccessMetrics {
    pub fn increment(endpoint: &str, kind: AccessMetricKind) {
        ModelAccessMetrics::increment(endpoint, kind);
    }
}

#[must_use]
pub fn snapshot() -> AccessMetricsSnapshot {
    let entries = ModelAccessMetrics::export_raw().into_iter().collect();
    AccessMetricsSnapshot { entries }
}
