use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, time::Duration};

thread_local! {
    static TIMER_METRICS: RefCell<HashMap<TimerMetricKey, u64>> = RefCell::new(HashMap::new());
}

///
/// TimerMode
///

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum TimerMode {
    Interval,
    Once,
}

///
/// TimerMetricKey
/// Uniquely identifies a timer by mode + delay (ms) + label.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TimerMetricKey {
    pub mode: TimerMode,
    pub delay_ms: u64,
    pub label: String,
}

///
/// TimerMetricEntry
/// Snapshot entry pairing a timer mode/delay with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct TimerMetricEntry {
    pub mode: TimerMode,
    pub delay_ms: u64,
    pub label: String,
    pub count: u64,
}

///
/// TimerMetricsSnapshot
///

pub type TimerMetricsSnapshot = Vec<TimerMetricEntry>;

///
/// TimerMetrics
///
/// Volatile counters for timers keyed by `(mode, delay_ms, label)`.
///
/// ## What this measures
///
/// `TimerMetrics` is intended to answer two related questions:
///
/// 1) **Which timers have been scheduled?**
///    - Use [`ensure`] at scheduling time to guarantee the timer appears in snapshots,
///      even if it has not fired yet (e.g., newly-created interval timers).
///
/// 2) **How many scheduling events have occurred for a given timer key?**
///    - Use [`increment`] when you explicitly want to count scheduling operations.
///
/// Note that this type does **not** count executions/ticks of interval timers.
/// Execution counts should be tracked separately (e.g., via perf records or a
/// dedicated “timer runs” metric), because scheduling and execution are different signals.
///
/// ## Cardinality and labels
///
/// Labels are used as metric keys. Keep labels stable and low-cardinality (avoid
/// embedding principals, IDs, or other high-variance values).
///
/// ## Thread safety / runtime model
///
/// This uses `thread_local!` storage. On the IC, this is the standard way to maintain
/// mutable global state without `unsafe`.
///

pub struct TimerMetrics;

impl TimerMetrics {
    #[allow(clippy::cast_possible_truncation)]
    fn delay_ms(delay: Duration) -> u64 {
        delay.as_millis().min(u128::from(u64::MAX)) as u64
    }

    /// Ensure a timer key exists in the metrics table with an initial count of `0`.
    ///
    /// This is used at **schedule time** to make timers visible in snapshots before they
    /// have fired (particularly important for interval timers).
    ///
    /// Idempotent: calling `ensure` repeatedly for the same key does not change the count.
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

    /// Increment the scheduling counter for a timer key.
    ///
    /// Use this when you want to count how many times a given timer (identified by
    /// `(mode, delay_ms, label)`) has been scheduled.
    ///
    /// This uses saturating arithmetic to avoid overflow.
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

    /// Snapshot all timer scheduling metrics.
    ///
    /// Returns the current contents of the metrics table as a vector of entries.
    /// Callers may sort or page the results as needed at the API layer.
    #[must_use]
    pub fn snapshot() -> TimerMetricsSnapshot {
        TIMER_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(key, count)| TimerMetricEntry {
                    mode: key.mode,
                    delay_ms: key.delay_ms,
                    label: key.label.clone(),
                    count: *count,
                })
                .collect()
        })
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

        let snapshot = TimerMetrics::snapshot();
        let mut map: HashMap<(TimerMode, u64, String), u64> = snapshot
            .into_iter()
            .map(|entry| ((entry.mode, entry.delay_ms, entry.label), entry.count))
            .collect();

        assert_eq!(
            map.remove(&(TimerMode::Once, 1_000, "once:a".to_string())),
            Some(2)
        );
        assert_eq!(
            map.remove(&(TimerMode::Interval, 500, "interval:b".to_string())),
            Some(1)
        );
        assert!(map.is_empty());
    }
}
