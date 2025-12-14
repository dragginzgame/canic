use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static ICC_METRICS: RefCell<HashMap<IccMetricKey, u64>> = RefCell::new(HashMap::new());
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

#[cfg(test)]
mod test {
    use super::*;

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
