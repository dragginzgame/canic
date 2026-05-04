use crate::{InternalError, InternalErrorClass, InternalErrorOrigin};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static CASCADE_METRICS: RefCell<HashMap<CascadeMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// CascadeMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum CascadeMetricOperation {
    ChildSend,
    LocalApply,
    NonrootFanout,
    RootFanout,
    RouteResolve,
}

impl CascadeMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::ChildSend => "child_send",
            Self::LocalApply => "local_apply",
            Self::NonrootFanout => "nonroot_fanout",
            Self::RootFanout => "root_fanout",
            Self::RouteResolve => "route_resolve",
        }
    }
}

///
/// CascadeMetricSnapshot
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum CascadeMetricSnapshot {
    State,
    Topology,
}

impl CascadeMetricSnapshot {
    /// Return the stable public metrics label for this snapshot kind.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Topology => "topology",
        }
    }
}

///
/// CascadeMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum CascadeMetricOutcome {
    Completed,
    Failed,
    Skipped,
    Started,
}

impl CascadeMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Started => "started",
        }
    }
}

///
/// CascadeMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum CascadeMetricReason {
    EmptySnapshot,
    InvalidState,
    ManagementCall,
    NoRoute,
    Ok,
    PartialFailure,
    PolicyDenied,
    SendFailed,
    Unknown,
}

impl CascadeMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::EmptySnapshot => "empty_snapshot",
            Self::InvalidState => "invalid_state",
            Self::ManagementCall => "management_call",
            Self::NoRoute => "no_route",
            Self::Ok => "ok",
            Self::PartialFailure => "partial_failure",
            Self::PolicyDenied => "policy_denied",
            Self::SendFailed => "send_failed",
            Self::Unknown => "unknown",
        }
    }

    /// Classify one internal error into a bounded metric reason.
    #[must_use]
    pub(crate) const fn from_error(err: &InternalError) -> Self {
        match (err.class(), err.origin()) {
            (InternalErrorClass::Infra, InternalErrorOrigin::Infra) => Self::ManagementCall,
            (InternalErrorClass::Domain, InternalErrorOrigin::Domain)
            | (InternalErrorClass::Access, _) => Self::PolicyDenied,
            (InternalErrorClass::Domain, InternalErrorOrigin::Config)
            | (
                InternalErrorClass::Invariant
                | InternalErrorClass::Ops
                | InternalErrorClass::Workflow,
                _,
            ) => Self::InvalidState,
            _ => Self::Unknown,
        }
    }
}

///
/// CascadeMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct CascadeMetricKey {
    pub operation: CascadeMetricOperation,
    pub snapshot: CascadeMetricSnapshot,
    pub outcome: CascadeMetricOutcome,
    pub reason: CascadeMetricReason,
}

///
/// CascadeMetrics
///

pub struct CascadeMetrics;

impl CascadeMetrics {
    /// Record one cascade operation event.
    pub fn record(
        operation: CascadeMetricOperation,
        snapshot: CascadeMetricSnapshot,
        outcome: CascadeMetricOutcome,
        reason: CascadeMetricReason,
    ) {
        CASCADE_METRICS.with_borrow_mut(|counts| {
            let key = CascadeMetricKey {
                operation,
                snapshot,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current cascade metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(CascadeMetricKey, u64)> {
        CASCADE_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all cascade metrics.
    #[cfg(test)]
    pub fn reset() {
        CASCADE_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<CascadeMetricKey, u64> {
        CascadeMetrics::snapshot().into_iter().collect()
    }

    // Verify cascade metrics accumulate by operation, snapshot, outcome, and reason.
    #[test]
    fn cascade_metrics_accumulate_by_operation_snapshot_outcome_and_reason() {
        CascadeMetrics::reset();

        CascadeMetrics::record(
            CascadeMetricOperation::RootFanout,
            CascadeMetricSnapshot::State,
            CascadeMetricOutcome::Started,
            CascadeMetricReason::Ok,
        );
        CascadeMetrics::record(
            CascadeMetricOperation::ChildSend,
            CascadeMetricSnapshot::Topology,
            CascadeMetricOutcome::Failed,
            CascadeMetricReason::SendFailed,
        );
        CascadeMetrics::record(
            CascadeMetricOperation::ChildSend,
            CascadeMetricSnapshot::Topology,
            CascadeMetricOutcome::Failed,
            CascadeMetricReason::SendFailed,
        );

        let map = snapshot_map();

        assert_eq!(
            map.get(&CascadeMetricKey {
                operation: CascadeMetricOperation::RootFanout,
                snapshot: CascadeMetricSnapshot::State,
                outcome: CascadeMetricOutcome::Started,
                reason: CascadeMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&CascadeMetricKey {
                operation: CascadeMetricOperation::ChildSend,
                snapshot: CascadeMetricSnapshot::Topology,
                outcome: CascadeMetricOutcome::Failed,
                reason: CascadeMetricReason::SendFailed,
            }),
            Some(&2)
        );
    }
}
