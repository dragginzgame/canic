use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static CYCLES_TOPUP_METRICS: RefCell<HashMap<CyclesTopupMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// CyclesTopupMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[remain::sorted]
pub enum CyclesTopupMetricKey {
    AboveThreshold,
    ConfigError,
    PolicyMissing,
    RequestErr,
    RequestInFlight,
    RequestOk,
    RequestScheduled,
}

impl CyclesTopupMetricKey {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::AboveThreshold => "above_threshold",
            Self::ConfigError => "config_error",
            Self::PolicyMissing => "policy_missing",
            Self::RequestErr => "request_err",
            Self::RequestInFlight => "request_in_flight",
            Self::RequestOk => "request_ok",
            Self::RequestScheduled => "request_scheduled",
        }
    }
}

///
/// CyclesTopupMetrics
///

pub struct CyclesTopupMetrics;

impl CyclesTopupMetrics {
    // Record one auto-top-up decision or outcome.
    fn increment(key: CyclesTopupMetricKey) {
        CYCLES_TOPUP_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Record that current canister config could not be read.
    pub fn record_config_error() {
        Self::increment(CyclesTopupMetricKey::ConfigError);
    }

    /// Record that no auto-top-up policy is configured.
    pub fn record_policy_missing() {
        Self::increment(CyclesTopupMetricKey::PolicyMissing);
    }

    /// Record that balance is still above the configured threshold.
    pub fn record_above_threshold() {
        Self::increment(CyclesTopupMetricKey::AboveThreshold);
    }

    /// Record that a request was skipped because one is already running.
    pub fn record_request_in_flight() {
        Self::increment(CyclesTopupMetricKey::RequestInFlight);
    }

    /// Record that a top-up request was scheduled.
    pub fn record_request_scheduled() {
        Self::increment(CyclesTopupMetricKey::RequestScheduled);
    }

    /// Record that a top-up request succeeded.
    pub fn record_request_ok() {
        Self::increment(CyclesTopupMetricKey::RequestOk);
    }

    /// Record that a top-up request failed.
    pub fn record_request_err() {
        Self::increment(CyclesTopupMetricKey::RequestErr);
    }

    #[must_use]
    pub fn snapshot() -> Vec<(CyclesTopupMetricKey, u64)> {
        CYCLES_TOPUP_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    #[cfg(test)]
    pub fn reset() {
        CYCLES_TOPUP_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_map() -> HashMap<CyclesTopupMetricKey, u64> {
        CyclesTopupMetrics::snapshot().into_iter().collect()
    }

    #[test]
    fn topup_metrics_increment_by_decision() {
        CyclesTopupMetrics::reset();

        CyclesTopupMetrics::record_policy_missing();
        CyclesTopupMetrics::record_policy_missing();
        CyclesTopupMetrics::record_request_scheduled();
        CyclesTopupMetrics::record_request_ok();

        let map = snapshot_map();

        assert_eq!(map.get(&CyclesTopupMetricKey::PolicyMissing), Some(&2));
        assert_eq!(map.get(&CyclesTopupMetricKey::RequestScheduled), Some(&1));
        assert_eq!(map.get(&CyclesTopupMetricKey::RequestOk), Some(&1));
        assert_eq!(map.len(), 3);
    }
}
