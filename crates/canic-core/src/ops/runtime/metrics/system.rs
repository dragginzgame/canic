use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static SYSTEM_METRICS: RefCell<HashMap<SystemMetricKind, u64>> = RefCell::new(HashMap::new());
}

///
/// SystemMetricKind
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum SystemMetricKind {
    CanisterCall,
    CanisterStatus,
    CreateCanister,
    DeleteCanister,
    DepositCycles,
    HttpOutcall,
    InstallCode,
    RawRand,
    ReinstallCode,
    TimerScheduled,
    UninstallCode,
    UpdateSettings,
    UpgradeCode,
}

///
/// SystemMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct SystemMetricsSnapshot {
    pub entries: Vec<(SystemMetricKind, u64)>,
}

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

    #[must_use]
    pub fn snapshot() -> SystemMetricsSnapshot {
        let entries = SYSTEM_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect();

        SystemMetricsSnapshot { entries }
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

    fn snapshot_map() -> HashMap<SystemMetricKind, u64> {
        SystemMetrics::snapshot().entries.into_iter().collect()
    }

    #[test]
    fn system_metrics_start_empty() {
        SystemMetrics::reset();

        let snapshot = SystemMetrics::snapshot();
        assert!(snapshot.entries.is_empty());
    }

    #[test]
    fn increment_increases_counter() {
        SystemMetrics::reset();

        SystemMetrics::increment(SystemMetricKind::CanisterCall);

        let map = snapshot_map();
        assert_eq!(map.get(&SystemMetricKind::CanisterCall), Some(&1));
    }

    #[test]
    fn increment_accumulates() {
        SystemMetrics::reset();

        SystemMetrics::increment(SystemMetricKind::HttpOutcall);
        SystemMetrics::increment(SystemMetricKind::HttpOutcall);
        SystemMetrics::increment(SystemMetricKind::HttpOutcall);

        let map = snapshot_map();
        assert_eq!(map.get(&SystemMetricKind::HttpOutcall), Some(&3));
    }

    #[test]
    fn metrics_are_isolated_per_kind() {
        SystemMetrics::reset();

        SystemMetrics::increment(SystemMetricKind::CreateCanister);
        SystemMetrics::increment(SystemMetricKind::DeleteCanister);
        SystemMetrics::increment(SystemMetricKind::DeleteCanister);

        let map = snapshot_map();

        assert_eq!(map.get(&SystemMetricKind::CreateCanister), Some(&1));
        assert_eq!(map.get(&SystemMetricKind::DeleteCanister), Some(&2));
    }

    #[test]
    fn reset_clears_all_metrics() {
        SystemMetrics::reset();

        SystemMetrics::increment(SystemMetricKind::UpgradeCode);
        SystemMetrics::increment(SystemMetricKind::UpdateSettings);

        SystemMetrics::reset();

        let snapshot = SystemMetrics::snapshot();
        assert!(snapshot.entries.is_empty());
    }

    #[test]
    fn increment_saturates_at_u64_max() {
        SystemMetrics::reset();

        // Force near-overflow state
        SYSTEM_METRICS.with_borrow_mut(|counts| {
            counts.insert(SystemMetricKind::TimerScheduled, u64::MAX);
        });

        SystemMetrics::increment(SystemMetricKind::TimerScheduled);

        let map = snapshot_map();
        assert_eq!(map.get(&SystemMetricKind::TimerScheduled), Some(&u64::MAX));
    }
}
