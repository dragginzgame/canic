use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static REPLAY_METRICS: RefCell<HashMap<ReplayMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// ReplayMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ReplayMetricOperation {
    Abort,
    Check,
    Commit,
    Decode,
    Reserve,
}

impl ReplayMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Abort => "abort",
            Self::Check => "check",
            Self::Commit => "commit",
            Self::Decode => "decode",
            Self::Reserve => "reserve",
        }
    }
}

///
/// ReplayMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ReplayMetricOutcome {
    Completed,
    Failed,
}

impl ReplayMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

///
/// ReplayMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ReplayMetricReason {
    Capacity,
    Conflict,
    DecodeFailed,
    Duplicate,
    EncodeFailed,
    Expired,
    Fresh,
    InFlight,
    InvalidTtl,
    MissingMetadata,
    Ok,
}

impl ReplayMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Capacity => "capacity",
            Self::Conflict => "conflict",
            Self::DecodeFailed => "decode_failed",
            Self::Duplicate => "duplicate",
            Self::EncodeFailed => "encode_failed",
            Self::Expired => "expired",
            Self::Fresh => "fresh",
            Self::InFlight => "in_flight",
            Self::InvalidTtl => "invalid_ttl",
            Self::MissingMetadata => "missing_metadata",
            Self::Ok => "ok",
        }
    }
}

///
/// ReplayMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ReplayMetricKey {
    pub operation: ReplayMetricOperation,
    pub outcome: ReplayMetricOutcome,
    pub reason: ReplayMetricReason,
}

///
/// ReplayMetrics
///

pub struct ReplayMetrics;

impl ReplayMetrics {
    /// Record one root replay event.
    pub fn record(
        operation: ReplayMetricOperation,
        outcome: ReplayMetricOutcome,
        reason: ReplayMetricReason,
    ) {
        REPLAY_METRICS.with_borrow_mut(|counts| {
            let key = ReplayMetricKey {
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current replay metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(ReplayMetricKey, u64)> {
        REPLAY_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all replay metrics.
    #[cfg(test)]
    pub fn reset() {
        REPLAY_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<ReplayMetricKey, u64> {
        ReplayMetrics::snapshot().into_iter().collect()
    }

    #[test]
    fn replay_metrics_accumulate_by_operation_outcome_and_reason() {
        ReplayMetrics::reset();

        ReplayMetrics::record(
            ReplayMetricOperation::Check,
            ReplayMetricOutcome::Completed,
            ReplayMetricReason::Fresh,
        );
        ReplayMetrics::record(
            ReplayMetricOperation::Check,
            ReplayMetricOutcome::Completed,
            ReplayMetricReason::Fresh,
        );
        ReplayMetrics::record(
            ReplayMetricOperation::Reserve,
            ReplayMetricOutcome::Failed,
            ReplayMetricReason::Capacity,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&ReplayMetricKey {
                operation: ReplayMetricOperation::Check,
                outcome: ReplayMetricOutcome::Completed,
                reason: ReplayMetricReason::Fresh,
            }),
            Some(&2)
        );
        assert_eq!(
            map.get(&ReplayMetricKey {
                operation: ReplayMetricOperation::Reserve,
                outcome: ReplayMetricOutcome::Failed,
                reason: ReplayMetricReason::Capacity,
            }),
            Some(&1)
        );
    }
}
