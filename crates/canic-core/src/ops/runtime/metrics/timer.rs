use crate::ops::runtime::metrics::store::timer::{
    TimerMetricKey as ModelTimerMetricKey, TimerMetrics, TimerMode as ModelTimerMode,
};
use crate::ops::runtime::metrics::system::{SystemMetricKind, record_system_metric};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct TimerMetricsSnapshot {
    pub entries: Vec<(TimerMetricKey, u64)>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TimerMode {
    Interval,
    Once,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TimerMetricKey {
    pub mode: TimerMode,
    pub delay_ms: u64,
    pub label: String,
}

#[must_use]
pub fn snapshot() -> TimerMetricsSnapshot {
    let entries = TimerMetrics::export_raw()
        .into_iter()
        .map(|(key, count)| (key.into(), count))
        .collect();
    TimerMetricsSnapshot { entries }
}

/// Record a timer schedule event and ensure the metric entry exists.
pub fn record_timer_scheduled(mode: TimerMode, delay: Duration, label: &str) {
    record_system_metric(SystemMetricKind::TimerScheduled);
    TimerMetrics::ensure(mode_to_model(mode), delay, label);
}

/// Record a timer execution event.
pub fn record_timer_tick(mode: TimerMode, delay: Duration, label: &str) {
    TimerMetrics::increment(mode_to_model(mode), delay, label);
}

const fn mode_to_model(mode: TimerMode) -> ModelTimerMode {
    match mode {
        TimerMode::Interval => ModelTimerMode::Interval,
        TimerMode::Once => ModelTimerMode::Once,
    }
}

const fn mode_from_model(mode: ModelTimerMode) -> TimerMode {
    match mode {
        ModelTimerMode::Interval => TimerMode::Interval,
        ModelTimerMode::Once => TimerMode::Once,
    }
}

impl From<ModelTimerMetricKey> for TimerMetricKey {
    fn from(key: ModelTimerMetricKey) -> Self {
        Self {
            mode: mode_from_model(key.mode),
            delay_ms: key.delay_ms,
            label: key.label,
        }
    }
}
