use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static ROOT_CAPABILITY_METRICS: RefCell<HashMap<(RootCapabilityMetricKey, RootCapabilityMetricEvent), u64>> =
        RefCell::new(HashMap::new());
}

///
/// RootCapabilityMetricKey
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RootCapabilityMetricKey {
    Provision,
    Upgrade,
    MintCycles,
    IssueDelegation,
}

impl RootCapabilityMetricKey {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Provision => "Provision",
            Self::Upgrade => "Upgrade",
            Self::MintCycles => "MintCycles",
            Self::IssueDelegation => "IssueDelegation",
        }
    }
}

///
/// RootCapabilityMetricEvent
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RootCapabilityMetricEvent {
    Authorized,
    Denied,
    ReplayAccepted,
    ReplayDuplicateSame,
    ReplayDuplicateConflict,
    ReplayExpired,
    ReplayTtlExceeded,
    ExecutionSuccess,
    ExecutionError,
}

impl RootCapabilityMetricEvent {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Authorized => "Authorized",
            Self::Denied => "Denied",
            Self::ReplayAccepted => "ReplayAccepted",
            Self::ReplayDuplicateSame => "ReplayDuplicateSame",
            Self::ReplayDuplicateConflict => "ReplayDuplicateConflict",
            Self::ReplayExpired => "ReplayExpired",
            Self::ReplayTtlExceeded => "ReplayTtlExceeded",
            Self::ExecutionSuccess => "ExecutionSuccess",
            Self::ExecutionError => "ExecutionError",
        }
    }
}

///
/// RootCapabilityMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct RootCapabilityMetricsSnapshot {
    pub entries: Vec<(RootCapabilityMetricKey, RootCapabilityMetricEvent, u64)>,
}

///
/// RootCapabilityMetrics
///

pub struct RootCapabilityMetrics;

impl RootCapabilityMetrics {
    pub fn record(capability: RootCapabilityMetricKey, event: RootCapabilityMetricEvent) {
        ROOT_CAPABILITY_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry((capability, event)).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    #[must_use]
    pub fn snapshot() -> RootCapabilityMetricsSnapshot {
        let entries = ROOT_CAPABILITY_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .map(|((capability, event), count)| (capability, event, count))
            .collect();

        RootCapabilityMetricsSnapshot { entries }
    }

    #[cfg(test)]
    pub fn reset() {
        ROOT_CAPABILITY_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn snapshot_map() -> HashMap<(RootCapabilityMetricKey, RootCapabilityMetricEvent), u64> {
        RootCapabilityMetrics::snapshot()
            .entries
            .into_iter()
            .map(|(capability, event, count)| ((capability, event), count))
            .collect()
    }

    #[test]
    fn root_capability_metrics_start_empty() {
        RootCapabilityMetrics::reset();

        let snapshot = RootCapabilityMetrics::snapshot();
        assert!(snapshot.entries.is_empty());
    }

    #[test]
    fn record_increments_for_same_key_and_event() {
        RootCapabilityMetrics::reset();

        RootCapabilityMetrics::record(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricEvent::Authorized,
        );
        RootCapabilityMetrics::record(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricEvent::Authorized,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&(
                RootCapabilityMetricKey::Provision,
                RootCapabilityMetricEvent::Authorized
            )),
            Some(&2)
        );
    }

    #[test]
    fn metrics_are_partitioned_by_capability_and_event() {
        RootCapabilityMetrics::reset();

        RootCapabilityMetrics::record(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricEvent::Authorized,
        );
        RootCapabilityMetrics::record(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricEvent::Denied,
        );
        RootCapabilityMetrics::record(
            RootCapabilityMetricKey::IssueDelegation,
            RootCapabilityMetricEvent::Denied,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&(
                RootCapabilityMetricKey::Provision,
                RootCapabilityMetricEvent::Authorized
            )),
            Some(&1)
        );
        assert_eq!(
            map.get(&(
                RootCapabilityMetricKey::Provision,
                RootCapabilityMetricEvent::Denied
            )),
            Some(&1)
        );
        assert_eq!(
            map.get(&(
                RootCapabilityMetricKey::IssueDelegation,
                RootCapabilityMetricEvent::Denied
            )),
            Some(&1)
        );
    }
}
