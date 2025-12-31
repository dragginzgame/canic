use candid::Principal;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    /// Thread-local storage for inter-canister call counters.
    ///
    /// Keyed by `(target, method)` and holding the number of calls observed.
    static ICC_METRICS: RefCell<HashMap<IccMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// IccMetricKey
///
/// Uniquely identifies an inter-canister call by:
/// - target canister principal
/// - method name
///

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

    /// Export the raw ICC metrics table.
    ///
    /// Returns the internal `(IccMetricKey, count)` map without
    /// sorting or presentation shaping.
    #[must_use]
    pub fn export_raw() -> HashMap<IccMetricKey, u64> {
        ICC_METRICS.with_borrow(std::clone::Clone::clone)
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

    #[test]
    fn icc_metrics_track_target_and_method() {
        IccMetrics::reset();

        let t1 = Principal::from_slice(&[1; 29]);
        let t2 = Principal::from_slice(&[2; 29]);

        IccMetrics::increment(t1, "foo");
        IccMetrics::increment(t1, "foo");
        IccMetrics::increment(t1, "bar");
        IccMetrics::increment(t2, "foo");

        let raw = IccMetrics::export_raw();

        assert_eq!(
            raw.get(&IccMetricKey {
                target: t1,
                method: "foo".to_string()
            }),
            Some(&2)
        );

        assert_eq!(
            raw.get(&IccMetricKey {
                target: t1,
                method: "bar".to_string()
            }),
            Some(&1)
        );

        assert_eq!(
            raw.get(&IccMetricKey {
                target: t2,
                method: "foo".to_string()
            }),
            Some(&1)
        );

        assert_eq!(raw.len(), 3);
    }
}
