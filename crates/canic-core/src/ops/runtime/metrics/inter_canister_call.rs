use crate::{
    ids::SystemMetricKind,
    ops::{prelude::*, runtime::metrics::system::SystemMetrics},
};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    /// Thread-local storage for inter-canister call counters.
    ///
    /// Keyed by `(target, method)` and holding the number of calls observed.
    static INTER_CANISTER_CALL_METRICS: RefCell<HashMap<InterCanisterCallMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// InterCanisterCallMetricsSnapshot
///

#[derive(Clone)]
pub struct InterCanisterCallMetricsSnapshot {
    pub entries: Vec<(InterCanisterCallMetricKey, u64)>,
}

///
/// InterCanisterCallMetricKey
/// Cardinality is bounded by observed canister targets and static method names.
///

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct InterCanisterCallMetricKey {
    pub target: Principal,
    pub method: String,
}

///
/// InterCanisterCallMetrics
/// Volatile counters for inter-canister calls keyed by target + method.
/// Targets may grow with topology size; methods must remain low-cardinality.
///

pub struct InterCanisterCallMetrics;

impl InterCanisterCallMetrics {
    /// Increment the inter-canister call counter for a target/method pair.
    fn increment(target: Principal, method: &str) {
        INTER_CANISTER_CALL_METRICS.with_borrow_mut(|counts| {
            let key = InterCanisterCallMetricKey {
                target,
                method: method.to_string(),
            };

            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Record an inter-canister call in system + ICC metrics.
    ///
    /// This is the preferred integration point for call instrumentation,
    /// even if not all call paths currently route through it.
    pub fn record_call(target: impl Into<Principal>, method: &str) {
        let target: Principal = target.into();

        SystemMetrics::increment(SystemMetricKind::CanisterCall);
        Self::increment(target, method);
    }

    /// Snapshot the current inter-canister call metrics as a stable vector.
    #[must_use]
    pub fn snapshot() -> InterCanisterCallMetricsSnapshot {
        let entries = INTER_CANISTER_CALL_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect();

        InterCanisterCallMetricsSnapshot { entries }
    }

    /// Test-only helper: clear all inter-canister call metrics.
    #[cfg(test)]
    pub fn reset() {
        INTER_CANISTER_CALL_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::runtime::metrics::system::SystemMetrics;

    fn snapshot_map() -> HashMap<InterCanisterCallMetricKey, u64> {
        InterCanisterCallMetrics::snapshot()
            .entries
            .into_iter()
            .collect()
    }

    #[test]
    fn inter_canister_call_metrics_track_target_and_method() {
        InterCanisterCallMetrics::reset();

        let t1 = Principal::from_slice(&[1; 29]);
        let t2 = Principal::from_slice(&[2; 29]);

        InterCanisterCallMetrics::increment(t1, "foo");
        InterCanisterCallMetrics::increment(t1, "foo");
        InterCanisterCallMetrics::increment(t1, "bar");
        InterCanisterCallMetrics::increment(t2, "foo");

        let map = snapshot_map();

        assert_eq!(
            map.get(&InterCanisterCallMetricKey {
                target: t1,
                method: "foo".to_string()
            }),
            Some(&2)
        );

        assert_eq!(
            map.get(&InterCanisterCallMetricKey {
                target: t1,
                method: "bar".to_string()
            }),
            Some(&1)
        );

        assert_eq!(
            map.get(&InterCanisterCallMetricKey {
                target: t2,
                method: "foo".to_string()
            }),
            Some(&1)
        );

        assert_eq!(map.len(), 3);
    }

    #[test]
    fn record_call_updates_inter_canister_call_and_system_metrics() {
        InterCanisterCallMetrics::reset();
        SystemMetrics::reset();

        let target = Principal::from_slice(&[3; 29]);
        InterCanisterCallMetrics::record_call(target, "canic_sync");
        InterCanisterCallMetrics::record_call(target, "canic_sync");

        let map = snapshot_map();
        assert_eq!(
            map.get(&InterCanisterCallMetricKey {
                target,
                method: "canic_sync".to_string()
            }),
            Some(&2)
        );

        let system: HashMap<_, _> = SystemMetrics::snapshot().into_iter().collect();
        assert_eq!(system.get(&SystemMetricKind::CanisterCall), Some(&2));
    }
}
