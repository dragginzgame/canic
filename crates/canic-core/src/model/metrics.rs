use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, time::Duration};

thread_local! {
    static SYSTEM_METRICS: RefCell<HashMap<SystemMetricKind, u64>> = RefCell::new(HashMap::new());
    static ICC_METRICS: RefCell<HashMap<IccMetricKey, u64>> = RefCell::new(HashMap::new());
    static HTTP_METRICS: RefCell<HashMap<HttpMetricKey, u64>> = RefCell::new(HashMap::new());
    static TIMER_METRICS: RefCell<HashMap<TimerMetricKey, u64>> = RefCell::new(HashMap::new());
}

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

///
/// SystemMetricsSnapshot
///

pub type SystemMetricsSnapshot = Vec<SystemMetricEntry>;

///
/// SystemMetricKind
/// Enumerates the resource-heavy actions we track.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[remain::sorted]
pub enum SystemMetricKind {
    CanisterCall,
    CanisterStatus,
    CreateCanister,
    DeleteCanister,
    DepositCycles,
    HttpOutcall,
    InstallCode,
    ReinstallCode,
    TimerScheduled,
    UninstallCode,
    UpgradeCode,
}

///
/// SystemMetricEntry
/// Snapshot entry pairing a metric kind with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SystemMetricEntry {
    pub kind: SystemMetricKind,
    pub count: u64,
}

///
/// IccMetricKey
/// Uniquely identifies an inter-canister call by target + method.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct IccMetricKey {
    pub target: Principal,
    pub method: String,
}

///
/// IccMetricEntry
/// Snapshot entry pairing a target/method with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct IccMetricEntry {
    pub target: Principal,
    pub method: String,
    pub count: u64,
}

///
/// HttpMetricKey
/// Uniquely identifies an HTTP outcall by method + URL.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct HttpMetricKey {
    pub method: String,
    pub url: String,
}

///
/// HttpMetricEntry
/// Snapshot entry pairing a method/url with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct HttpMetricEntry {
    pub method: String,
    pub url: String,
    pub count: u64,
}

///
/// HttpMetricsSnapshot
///

pub type HttpMetricsSnapshot = Vec<HttpMetricEntry>;

///
/// TimerMode
///
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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
/// IccMetricsSnapshot
///

pub type IccMetricsSnapshot = Vec<IccMetricEntry>;

///
/// MetricsReport
/// Composite metrics view bundling action, ICC, HTTP, and timer counters.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MetricsReport {
    pub system: SystemMetricsSnapshot,
    pub icc: IccMetricsSnapshot,
    pub http: HttpMetricsSnapshot,
    pub timer: TimerMetricsSnapshot,
}

// -----------------------------------------------------------------------------
// State
// -----------------------------------------------------------------------------

///
/// SystemMetrics
/// Thin facade over the action metrics counters.
///

pub struct SystemMetrics;

impl SystemMetrics {
    /// Increment a counter and return the new value.
    pub fn increment(kind: SystemMetricKind) {
        SYSTEM_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(kind).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Return a snapshot of all counters.
    #[must_use]
    pub fn snapshot() -> Vec<SystemMetricEntry> {
        SYSTEM_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(kind, count)| SystemMetricEntry {
                    kind: *kind,
                    count: *count,
                })
                .collect()
        })
    }

    #[cfg(test)]
    pub fn reset() {
        SYSTEM_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// IccMetrics
/// Volatile counters for inter-canister calls keyed by target + method.
///

pub struct IccMetrics;

impl IccMetrics {
    /// Increment the ICC counter for a target/method pair.
    pub fn increment(target: Principal, method: &str) {
        ICC_METRICS.with_borrow_mut(|counts| {
            let key = IccMetricKey {
                target,
                method: method.to_string(),
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot all ICC counters.
    #[must_use]
    pub fn snapshot() -> IccMetricsSnapshot {
        ICC_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(key, count)| IccMetricEntry {
                    target: key.target,
                    method: key.method.clone(),
                    count: *count,
                })
                .collect()
        })
    }

    #[cfg(test)]
    pub fn reset() {
        ICC_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// HttpMetrics
/// Volatile counters for HTTP outcalls keyed by method + URL.
///

pub struct HttpMetrics;

impl HttpMetrics {
    pub fn increment(method: &str, url: &str) {
        HTTP_METRICS.with_borrow_mut(|counts| {
            let key = HttpMetricKey {
                method: method.to_string(),
                url: url.to_string(),
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    #[must_use]
    pub fn snapshot() -> HttpMetricsSnapshot {
        HTTP_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(key, count)| HttpMetricEntry {
                    method: key.method.clone(),
                    url: key.url.clone(),
                    count: *count,
                })
                .collect()
        })
    }

    #[cfg(test)]
    pub fn reset() {
        HTTP_METRICS.with_borrow_mut(HashMap::clear);
    }
}

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
    use std::collections::HashMap;

    #[test]
    fn increments_and_snapshots() {
        SystemMetrics::reset();

        SystemMetrics::increment(SystemMetricKind::CreateCanister);
        SystemMetrics::increment(SystemMetricKind::CreateCanister);
        SystemMetrics::increment(SystemMetricKind::InstallCode);

        let snapshot = SystemMetrics::snapshot();
        let as_map: HashMap<SystemMetricKind, u64> = snapshot
            .into_iter()
            .map(|entry| (entry.kind, entry.count))
            .collect();

        assert_eq!(as_map.get(&SystemMetricKind::CreateCanister), Some(&2));
        assert_eq!(as_map.get(&SystemMetricKind::InstallCode), Some(&1));
        assert!(!as_map.contains_key(&SystemMetricKind::CanisterCall));
    }

    #[test]
    fn icc_metrics_track_target_and_method() {
        IccMetrics::reset();

        let t1 = Principal::from_slice(&[1; 29]);
        let t2 = Principal::from_slice(&[2; 29]);

        IccMetrics::increment(t1, "foo");
        IccMetrics::increment(t1, "foo");
        IccMetrics::increment(t1, "bar");
        IccMetrics::increment(t2, "foo");

        let snapshot = IccMetrics::snapshot();
        let mut map: HashMap<(Principal, String), u64> = snapshot
            .into_iter()
            .map(|entry| ((entry.target, entry.method), entry.count))
            .collect();

        assert_eq!(map.remove(&(t1, "foo".to_string())), Some(2));
        assert_eq!(map.remove(&(t1, "bar".to_string())), Some(1));
        assert_eq!(map.remove(&(t2, "foo".to_string())), Some(1));
        assert!(map.is_empty());
    }

    #[test]
    fn http_metrics_track_method_and_url() {
        HttpMetrics::reset();

        HttpMetrics::increment("GET", "https://example.com/a");
        HttpMetrics::increment("GET", "https://example.com/a");
        HttpMetrics::increment("POST", "https://example.com/a");
        HttpMetrics::increment("GET", "https://example.com/b");

        let snapshot = HttpMetrics::snapshot();
        let mut map: HashMap<(String, String), u64> = snapshot
            .into_iter()
            .map(|entry| ((entry.method, entry.url), entry.count))
            .collect();

        assert_eq!(
            map.remove(&("GET".to_string(), "https://example.com/a".to_string())),
            Some(2)
        );
        assert_eq!(
            map.remove(&("POST".to_string(), "https://example.com/a".to_string())),
            Some(1)
        );
        assert_eq!(
            map.remove(&("GET".to_string(), "https://example.com/b".to_string())),
            Some(1)
        );
        assert!(map.is_empty());
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
