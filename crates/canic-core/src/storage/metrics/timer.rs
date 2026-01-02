use std::{cell::RefCell, collections::HashMap, time::Duration};

thread_local! {
    /// Thread-local storage for timer execution counters.
    ///
    /// Keyed by `(mode, delay_ms, label)` and holding the number of times
    /// the timer has fired.
    static TIMER_METRICS: RefCell<HashMap<TimerMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// TimerMode
///
/// Identifies whether a timer is a one-shot or an interval timer.
///
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TimerMode {
    Interval,
    Once,
}

///
/// TimerMetricKey
///
/// Uniquely identifies a timer metric by:
/// - scheduling mode
/// - delay in milliseconds
/// - a stable, low-cardinality label
///
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TimerMetricKey {
    pub mode: TimerMode,
    pub delay_ms: u64,
    pub label: String,
}

///
/// TimerMetrics
///
/// Volatile counters for timer executions keyed by `(mode, delay_ms, label)`.
///
/// ## What this measures
///
/// `TimerMetrics` answers two related questions:
///
/// 1) **Which timers have been scheduled?**
///    - Use [`ensure`] at scheduling time to guarantee the timer appears in
///      raw exports, even if it has not fired yet (important for interval timers).
///
/// 2) **How many times has a given timer fired?**
///    - Use [`increment`] when a timer fires (one-shot completion or interval tick).
///
/// Interval timers are counted once per tick. Scheduling counts are tracked
/// separately (e.g. via `SystemMetricKind::TimerScheduled`), and instruction
/// costs are tracked via perf counters.
///
/// ## Cardinality and labels
///
/// Labels are used as metric keys. They should be:
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
    #[allow(clippy::cast_possible_truncation)]
    fn delay_ms(delay: Duration) -> u64 {
        delay.as_millis().min(u128::from(u64::MAX)) as u64
    }

    /// Ensure a timer key exists in the metrics table with an initial count of `0`.
    ///
    /// Intended to be called at **schedule time** so that timers are visible
    /// in raw exports before their first execution.
    ///
    /// Idempotent: repeated calls with the same key do not change the count.
    pub fn ensure(mode: TimerMode, delay: Duration, label: &str) {
        let delay_ms = Self::delay_ms(delay);

        TIMER_METRICS.with_borrow_mut(|counts| {
            let key = TimerMetricKey {
                mode,
                delay_ms,
                label: label.to_string(),
            };

            counts.entry(key).or_insert(0);
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
                delay_ms,
                label: label.to_string(),
            };

            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Export the raw timer metrics table.
    ///
    /// This returns the internal `(TimerMetricKey, count)` map and performs
    /// no sorting, aggregation, or presentation shaping.
    #[must_use]
    pub fn export_raw() -> HashMap<TimerMetricKey, u64> {
        TIMER_METRICS.with_borrow(std::clone::Clone::clone)
    }

    /// Test-only helper: clear all timer metrics.
    #[cfg(test)]
    pub fn reset() {
        TIMER_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

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

        let raw = TimerMetrics::export_raw();

        let key_once = TimerMetricKey {
            mode: TimerMode::Once,
            delay_ms: 1_000,
            label: "once:a".to_string(),
        };

        let key_interval = TimerMetricKey {
            mode: TimerMode::Interval,
            delay_ms: 500,
            label: "interval:b".to_string(),
        };

        assert_eq!(raw.get(&key_once), Some(&2));
        assert_eq!(raw.get(&key_interval), Some(&1));
        assert_eq!(raw.len(), 2);
    }

    #[test]
    fn ensure_creates_zero_count_entry() {
        TimerMetrics::reset();

        TimerMetrics::ensure(TimerMode::Interval, Duration::from_secs(2), "heartbeat");

        let raw = TimerMetrics::export_raw();

        let key = TimerMetricKey {
            mode: TimerMode::Interval,
            delay_ms: 2_000,
            label: "heartbeat".to_string(),
        };

        assert_eq!(raw.get(&key), Some(&0));
        assert_eq!(raw.len(), 1);
    }

    #[test]
    fn ensure_is_idempotent() {
        TimerMetrics::reset();

        TimerMetrics::ensure(TimerMode::Once, Duration::from_secs(1), "once:x");
        TimerMetrics::ensure(TimerMode::Once, Duration::from_secs(1), "once:x");

        let raw = TimerMetrics::export_raw();

        let key = TimerMetricKey {
            mode: TimerMode::Once,
            delay_ms: 1_000,
            label: "once:x".to_string(),
        };

        assert_eq!(raw.get(&key), Some(&0));
        assert_eq!(raw.len(), 1);
    }
}
