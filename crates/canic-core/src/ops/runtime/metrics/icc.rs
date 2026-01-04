use crate::ops::{
    prelude::*,
    runtime::metrics::system::{SystemMetricKind, SystemMetrics},
};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    /// Thread-local storage for inter-canister call counters.
    ///
    /// Keyed by `(target, method)` and holding the number of calls observed.
    static ICC_METRICS: RefCell<HashMap<IccMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// IccMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct IccMetricsSnapshot {
    pub entries: Vec<(IccMetricKey, u64)>,
}

///
/// IccMetricKey
///

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IccMetricKey {
    pub target: Principal,
    pub method: String,
}

///
/// IccMetrics
/// Volatile counters for inter-canister calls keyed by target + method.
///

pub struct IccMetrics;

impl IccMetrics {
    /// Increment the ICC counter for a target/method pair.
    fn increment(target: Principal, method: &str) {
        ICC_METRICS.with_borrow_mut(|counts| {
            let key = IccMetricKey {
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

    /// Snapshot the current ICC metrics as a stable vector.
    #[must_use]
    pub fn snapshot() -> IccMetricsSnapshot {
        let entries = ICC_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect();

        IccMetricsSnapshot { entries }
    }

    /// Test-only helper: clear all ICC metrics.
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

    fn snapshot_map() -> HashMap<IccMetricKey, u64> {
        IccMetrics::snapshot().entries.into_iter().collect()
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

        let map = snapshot_map();

        assert_eq!(
            map.get(&IccMetricKey {
                target: t1,
                method: "foo".to_string()
            }),
            Some(&2)
        );

        assert_eq!(
            map.get(&IccMetricKey {
                target: t1,
                method: "bar".to_string()
            }),
            Some(&1)
        );

        assert_eq!(
            map.get(&IccMetricKey {
                target: t2,
                method: "foo".to_string()
            }),
            Some(&1)
        );

        assert_eq!(map.len(), 3);
    }
}
