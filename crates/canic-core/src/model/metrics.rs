use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static METRIC_COUNTS: RefCell<HashMap<MetricKind, u64>> = RefCell::new(HashMap::new());
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
}
