use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static SYSTEM_METRICS: RefCell<HashMap<SystemMetricKind, u64>> = RefCell::new(HashMap::new());
}

///
/// SystemMetricKind
/// Enumerates the resource-heavy actions we track.
///

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
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
    UpdateSettings,
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
/// SystemMetricsSnapshot
///

pub type SystemMetricsSnapshot = Vec<SystemMetricEntry>;

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
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn system_metrics_increments_and_snapshots() {
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
}
