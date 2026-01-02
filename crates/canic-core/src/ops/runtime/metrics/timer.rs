use crate::storage::metrics::{
    system::{SystemMetricKind, SystemMetrics},
    timer::{TimerMetricKey, TimerMetrics, TimerMode},
};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct TimerMetricsSnapshot {
    pub entries: Vec<(TimerMetricKey, u64)>,
}

#[must_use]
pub fn snapshot() -> TimerMetricsSnapshot {
    let entries = TimerMetrics::export_raw().into_iter().collect();
    TimerMetricsSnapshot { entries }
}

/// Record a timer schedule event and ensure the metric entry exists.
pub fn record_timer_scheduled(mode: TimerMode, delay: Duration, label: &str) {
    SystemMetrics::increment(SystemMetricKind::TimerScheduled);
    TimerMetrics::ensure(mode, delay, label);
}

/// Record a timer execution event.
pub fn record_timer_tick(mode: TimerMode, delay: Duration, label: &str) {
    TimerMetrics::increment(mode, delay, label);
}
