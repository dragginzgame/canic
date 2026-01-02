use crate::storage::metrics::system::{SystemMetricKind, SystemMetrics};

#[derive(Clone, Debug)]
pub struct SystemMetricsSnapshot {
    pub entries: Vec<(SystemMetricKind, u64)>,
}

#[must_use]
pub fn snapshot() -> SystemMetricsSnapshot {
    let entries = SystemMetrics::export_raw().into_iter().collect();
    SystemMetricsSnapshot { entries }
}

/// Record a single system metric.
pub fn record_system_metric(kind: SystemMetricKind) {
    SystemMetrics::increment(kind);
}

/// Record a single HTTP outcall for system metrics.
pub fn record_http_outcall() {
    record_system_metric(SystemMetricKind::HttpOutcall);
}
