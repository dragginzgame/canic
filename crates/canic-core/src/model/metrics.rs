use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

use crate::types::Principal;

thread_local! {
    static METRIC_COUNTS: RefCell<HashMap<MetricKind, u64>> = RefCell::new(HashMap::new());
    static ICC_METRICS: RefCell<HashMap<IccMetricKey, u64>> = RefCell::new(HashMap::new());
}

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

///
/// MetricsSnapshot
///

pub type MetricsSnapshot = Vec<MetricEntry>;

///
/// MetricKind
/// Enumerates the resource-heavy actions we track.
///
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MetricKind {
    CreateCanister,
    InstallCode,
    ReinstallCode,
    UpgradeCode,
    UninstallCode,
    DeleteCanister,
    DepositCycles,
    CanisterStatus,
    CanisterCall,
}

///
/// MetricEntry
/// Snapshot entry pairing a metric kind with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MetricEntry {
    pub kind: MetricKind,
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
/// IccMetricsSnapshot
///
pub type IccMetricsSnapshot = Vec<IccMetricEntry>;

///
/// MetricsReport
/// Composite metrics view bundling action and ICC counters.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MetricsReport {
    pub system: MetricsSnapshot,
    pub icc: IccMetricsSnapshot,
}

// -----------------------------------------------------------------------------
// State
// -----------------------------------------------------------------------------

///
/// MetricsState
/// Volatile counters for resource-using IC actions.
///
pub struct MetricsState;

impl MetricsState {
    /// Increment a counter and return the new value.
    pub fn increment(kind: MetricKind) {
        METRIC_COUNTS.with_borrow_mut(|counts| {
            let entry = counts.entry(kind).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Return a snapshot of all counters.
    #[must_use]
    pub fn snapshot() -> Vec<MetricEntry> {
        METRIC_COUNTS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(kind, count)| MetricEntry {
                    kind: *kind,
                    count: *count,
                })
                .collect()
        })
    }

    #[cfg(test)]
    pub fn reset() {
        METRIC_COUNTS.with_borrow_mut(HashMap::clear);
    }
}

///
/// SystemMetrics
/// Thin facade over the action metrics counters.
///
pub struct SystemMetrics;

impl SystemMetrics {
    pub fn record(kind: MetricKind) {
        MetricsState::increment(kind);
    }

    #[must_use]
    pub fn snapshot() -> MetricsSnapshot {
        MetricsState::snapshot()
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
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn increments_and_snapshots() {
        MetricsState::reset();

        MetricsState::increment(MetricKind::CreateCanister);
        MetricsState::increment(MetricKind::CreateCanister);
        MetricsState::increment(MetricKind::InstallCode);

        let snapshot = MetricsState::snapshot();
        let as_map: HashMap<MetricKind, u64> = snapshot
            .into_iter()
            .map(|entry| (entry.kind, entry.count))
            .collect();

        assert_eq!(as_map.get(&MetricKind::CreateCanister), Some(&2));
        assert_eq!(as_map.get(&MetricKind::InstallCode), Some(&1));
        assert!(!as_map.contains_key(&MetricKind::CanisterCall));
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
}
