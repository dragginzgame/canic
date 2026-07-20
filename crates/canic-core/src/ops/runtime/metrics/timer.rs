//! Module: ops::runtime::metrics::timer
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the timer family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use crate::{ids::SystemMetricKind, ops::runtime::metrics::system::SystemMetrics};
use std::{cell::RefCell, collections::HashMap, time::Duration};

pub use crate::domain::runtime::TimerMode;

thread_local! {
    /// Thread-local storage for timer execution counters.
    ///
    /// Keyed by `(mode, delay_ms, label)` and holding the number of times
    /// the timer has fired.
    static TIMER_METRICS: RefCell<HashMap<TimerMetricKey, TimerMetricValue>> =
        RefCell::new(HashMap::new());
}

///
/// TimerMetricsSnapshot
///
/// Point-in-time timer metric rows collected from the runtime counter table.
///

#[derive(Clone)]
pub struct TimerMetricsSnapshot {
    pub entries: Vec<(TimerMetricKey, TimerMetricValue)>,
}

///
/// TimerMetricKey
///
/// Composite key for one low-cardinality timer execution counter.
///

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct TimerMetricKey {
    pub mode: TimerMode,
    pub label: String,
}

/// Bounded values retained for one logical timer label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerMetricValue {
    pub executions: u64,
    pub latest_delay_ms: u64,
}

///
/// TimerMetrics
///
/// Operations-layer recorder for timer execution counters.
///
/// ## What this measures
///
/// `TimerMetrics` answers two related questions:
///
/// 1) **Which timers have been scheduled?**
///    - Use [`ensure`](Self::ensure) at scheduling time to guarantee the timer
///      appears in snapshots before its first execution.
///
/// 2) **How many times has a given timer fired?**
///    - Use [`increment`](Self::increment) when a timer fires.
///
/// Interval timers are counted once per tick. Scheduling counts are tracked
/// separately (e.g. via `SystemMetricKind::TimerScheduled`), and instruction
/// costs are tracked via perf counters.
///
/// ## Cardinality and labels
///
/// Mode and label are the complete metric key; delays remain values so exact
/// deadline scheduling cannot create a new row per duration. Labels should be:
/// - stable
/// - low-cardinality
/// - free of principals, IDs, or other high-variance data
///
/// ## Runtime model
///
/// Uses `thread_local!` storage. On the IC, this is the standard pattern
/// for maintaining mutable global state without `unsafe`.
///

pub struct TimerMetrics;

impl TimerMetrics {
    /// Convert a `Duration` to milliseconds, saturating at `u64::MAX`.
    #[expect(clippy::cast_possible_truncation)]
    fn delay_ms(delay: Duration) -> u64 {
        delay.as_millis().min(u128::from(u64::MAX)) as u64
    }

    /// Ensure a timer key exists in the metrics table with an initial count of `0`.
    ///
    /// Intended to be called at **schedule time** so that timers are visible
    /// in snapshots before their first execution.
    ///
    /// Repeated calls preserve the execution count and update the latest delay.
    pub fn ensure(mode: TimerMode, delay: Duration, label: &str) {
        let delay_ms = Self::delay_ms(delay);
        TIMER_METRICS.with_borrow_mut(|counts| {
            let key = TimerMetricKey {
                mode,
                label: label.to_string(),
            };
            counts
                .entry(key)
                .and_modify(|value| value.latest_delay_ms = delay_ms)
                .or_insert(TimerMetricValue {
                    executions: 0,
                    latest_delay_ms: delay_ms,
                });
        });
    }

    /// Increment the execution counter for a timer key.
    ///
    /// Call this when a timer fires (once for one-shot completion,
    /// once per tick for interval timers).
    ///
    /// Uses saturating arithmetic to avoid overflow.
    pub fn increment(mode: TimerMode, delay: Duration, label: &str) {
        let delay_ms = Self::delay_ms(delay);
        TIMER_METRICS.with_borrow_mut(|counts| {
            let key = TimerMetricKey {
                mode,
                label: label.to_string(),
            };
            let entry = counts.entry(key).or_insert(TimerMetricValue {
                executions: 0,
                latest_delay_ms: delay_ms,
            });
            entry.executions = entry.executions.saturating_add(1);
            entry.latest_delay_ms = delay_ms;
        });
    }

    /// Record a timer schedule event and ensure the metric entry exists.
    pub fn record_timer_scheduled(mode: TimerMode, delay: Duration, label: &str) {
        SystemMetrics::increment(SystemMetricKind::TimerScheduled);
        Self::ensure(mode, delay, label);
    }

    /// Record a timer execution event.
    pub fn record_timer_tick(mode: TimerMode, delay: Duration, label: &str) {
        Self::increment(mode, delay, label);
    }

    #[must_use]
    pub fn snapshot() -> TimerMetricsSnapshot {
        let entries = TIMER_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect();

        TimerMetricsSnapshot { entries }
    }

    /// Test-only helper: clear all timer metrics.
    #[cfg(test)]
    pub fn reset() {
        TIMER_METRICS.with_borrow_mut(HashMap::clear);
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_map() -> HashMap<TimerMetricKey, TimerMetricValue> {
        TimerMetrics::snapshot().entries.into_iter().collect()
    }

    #[test]
    fn timer_metrics_track_mode_delay_and_label() {
        TimerMetrics::reset();

        TimerMetrics::increment(TimerMode::Once, Duration::from_secs(1), "once:a");
        TimerMetrics::increment(TimerMode::Once, Duration::from_secs(1), "once:a");
        TimerMetrics::increment(
            TimerMode::Interval,
            Duration::from_millis(500),
            "interval:b",
        );

        let map = snapshot_map();

        let key_once = TimerMetricKey {
            mode: TimerMode::Once,
            label: "once:a".to_string(),
        };

        let key_interval = TimerMetricKey {
            mode: TimerMode::Interval,
            label: "interval:b".to_string(),
        };

        assert_eq!(
            map.get(&key_once),
            Some(&TimerMetricValue {
                executions: 2,
                latest_delay_ms: 1_000
            })
        );
        assert_eq!(
            map.get(&key_interval),
            Some(&TimerMetricValue {
                executions: 1,
                latest_delay_ms: 500
            })
        );
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn ensure_creates_zero_count_entry() {
        TimerMetrics::reset();

        TimerMetrics::ensure(TimerMode::Interval, Duration::from_secs(2), "heartbeat");

        let map = snapshot_map();

        let key = TimerMetricKey {
            mode: TimerMode::Interval,
            label: "heartbeat".to_string(),
        };

        assert_eq!(
            map.get(&key),
            Some(&TimerMetricValue {
                executions: 0,
                latest_delay_ms: 2_000
            })
        );
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn ensure_is_idempotent() {
        TimerMetrics::reset();

        TimerMetrics::ensure(TimerMode::Once, Duration::from_secs(1), "once:x");
        TimerMetrics::ensure(TimerMode::Once, Duration::from_secs(1), "once:x");

        let map = snapshot_map();

        let key = TimerMetricKey {
            mode: TimerMode::Once,
            label: "once:x".to_string(),
        };

        assert_eq!(
            map.get(&key),
            Some(&TimerMetricValue {
                executions: 0,
                latest_delay_ms: 1_000
            })
        );
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn rescheduling_a_deadline_updates_value_without_adding_cardinality() {
        TimerMetrics::reset();

        TimerMetrics::ensure(TimerMode::Once, Duration::from_secs(2), "deadline");
        TimerMetrics::ensure(TimerMode::Once, Duration::from_secs(1), "deadline");

        let map = snapshot_map();
        assert_eq!(map.len(), 1);
        assert_eq!(
            map.values().next(),
            Some(&TimerMetricValue {
                executions: 0,
                latest_delay_ms: 1_000
            })
        );
    }
}
