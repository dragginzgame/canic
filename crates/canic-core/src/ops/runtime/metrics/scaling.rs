use crate::{
    InternalError, InternalErrorClass, InternalErrorOrigin,
    domain::policy::placement::scaling::ScalingPlanReason,
};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static SCALING_METRICS: RefCell<HashMap<ScalingMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// ScalingMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ScalingMetricOperation {
    BootstrapConfig,
    BootstrapPool,
    CreateWorker,
    PlanCreate,
    RegisterWorker,
}

impl ScalingMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::BootstrapConfig => "bootstrap_config",
            Self::BootstrapPool => "bootstrap_pool",
            Self::CreateWorker => "create_worker",
            Self::PlanCreate => "plan_create",
            Self::RegisterWorker => "register_worker",
        }
    }
}

///
/// ScalingMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ScalingMetricOutcome {
    Completed,
    Failed,
    Skipped,
    Started,
}

impl ScalingMetricOutcome {
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
/// ScalingMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ScalingMetricReason {
    AtMaxWorkers,
    BelowMinWorkers,
    InvalidState,
    ManagementCall,
    MissingWorkerEntry,
    NoInitialWorkers,
    Ok,
    PolicyDenied,
    ScalingDisabled,
    TargetSatisfied,
    Unknown,
    WithinBounds,
}

impl ScalingMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::AtMaxWorkers => "at_max_workers",
            Self::BelowMinWorkers => "below_min_workers",
            Self::InvalidState => "invalid_state",
            Self::ManagementCall => "management_call",
            Self::MissingWorkerEntry => "missing_worker_entry",
            Self::NoInitialWorkers => "no_initial_workers",
            Self::Ok => "ok",
            Self::PolicyDenied => "policy_denied",
            Self::ScalingDisabled => "scaling_disabled",
            Self::TargetSatisfied => "target_satisfied",
            Self::Unknown => "unknown",
            Self::WithinBounds => "within_bounds",
        }
    }

    /// Classify one internal error into a bounded metric reason.
    #[must_use]
    pub(crate) const fn from_error(err: &InternalError) -> Self {
        match (err.class(), err.origin()) {
            (InternalErrorClass::Infra, InternalErrorOrigin::Infra) => Self::ManagementCall,
            (InternalErrorClass::Access | InternalErrorClass::Domain, _) => Self::PolicyDenied,
            (
                InternalErrorClass::Invariant
                | InternalErrorClass::Ops
                | InternalErrorClass::Workflow,
                _,
            ) => Self::InvalidState,
            _ => Self::Unknown,
        }
    }

    /// Classify one scaling policy decision into a bounded metric reason.
    #[must_use]
    pub(crate) const fn from_plan_reason(reason: ScalingPlanReason) -> Self {
        match reason {
            ScalingPlanReason::AtMaxWorkers => Self::AtMaxWorkers,
            ScalingPlanReason::BelowMinWorkers => Self::BelowMinWorkers,
            ScalingPlanReason::WithinBounds => Self::WithinBounds,
        }
    }
}

///
/// ScalingMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ScalingMetricKey {
    pub operation: ScalingMetricOperation,
    pub outcome: ScalingMetricOutcome,
    pub reason: ScalingMetricReason,
}

///
/// ScalingMetrics
///

pub struct ScalingMetrics;

impl ScalingMetrics {
    /// Record one scaling workflow event.
    pub fn record(
        operation: ScalingMetricOperation,
        outcome: ScalingMetricOutcome,
        reason: ScalingMetricReason,
    ) {
        SCALING_METRICS.with_borrow_mut(|counts| {
            let key = ScalingMetricKey {
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current scaling metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(ScalingMetricKey, u64)> {
        SCALING_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all scaling metrics.
    #[cfg(test)]
    pub fn reset() {
        SCALING_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<ScalingMetricKey, u64> {
        ScalingMetrics::snapshot().into_iter().collect()
    }

    // Verify scaling metrics accumulate by operation, outcome, and reason.
    #[test]
    fn scaling_metrics_accumulate_by_operation_outcome_and_reason() {
        ScalingMetrics::reset();

        ScalingMetrics::record(
            ScalingMetricOperation::PlanCreate,
            ScalingMetricOutcome::Started,
            ScalingMetricReason::Ok,
        );
        ScalingMetrics::record(
            ScalingMetricOperation::BootstrapPool,
            ScalingMetricOutcome::Skipped,
            ScalingMetricReason::TargetSatisfied,
        );
        ScalingMetrics::record(
            ScalingMetricOperation::BootstrapPool,
            ScalingMetricOutcome::Skipped,
            ScalingMetricReason::TargetSatisfied,
        );

        let map = snapshot_map();

        assert_eq!(
            map.get(&ScalingMetricKey {
                operation: ScalingMetricOperation::PlanCreate,
                outcome: ScalingMetricOutcome::Started,
                reason: ScalingMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&ScalingMetricKey {
                operation: ScalingMetricOperation::BootstrapPool,
                outcome: ScalingMetricOutcome::Skipped,
                reason: ScalingMetricReason::TargetSatisfied,
            }),
            Some(&2)
        );
    }
}
