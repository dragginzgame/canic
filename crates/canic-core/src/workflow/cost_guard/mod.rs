//! Module: workflow::cost_guard
//!
//! Responsibility: reserve cost permits with expiry scheduling and map public failures.
//! Does not own: quota accounting, cycle reservations, cleanup execution, or command policy.
//! Boundary: costed workflows reserve here before crossing external-effect boundaries.

use crate::{
    InternalError,
    model::replay::ReplayCostGuardSettlement,
    ops::cost_guard::{CostGuardOps, CostGuardPermit, CostGuardRequest, CostGuardReserveError},
    workflow::runtime::intent::IntentCleanupWorkflow,
};

/// Workflow authority that couples cost-intent mutation to finite-expiry scheduling.
pub struct CostGuardWorkflow;

impl CostGuardWorkflow {
    pub fn reserve(request: CostGuardRequest) -> Result<CostGuardPermit, CostGuardReserveError> {
        let deadline = CostGuardOps::cleanup_due_at_secs(&request)?;
        // The pair shares one deadline and one native owner. Admit that owner
        // before either reservation; there is no second fallible scheduling point.
        IntentCleanupWorkflow::schedule_at(deadline).map_err(CostGuardReserveError::Store)?;
        CostGuardOps::reserve(request)
    }

    pub fn complete(permit: &CostGuardPermit, now_secs: u64) -> Result<(), InternalError> {
        let result = CostGuardOps::complete(permit, now_secs);
        Self::reconcile_after_success(&result);
        result
    }

    pub fn complete_replay_settlement(
        settlement: &ReplayCostGuardSettlement,
        now_secs: u64,
    ) -> Result<(), InternalError> {
        let result = CostGuardOps::complete_replay_settlement(settlement, now_secs);
        Self::reconcile_after_success(&result);
        result
    }

    #[must_use]
    pub fn complete_after_failure(
        permit: &CostGuardPermit,
        now_secs: u64,
        error: InternalError,
    ) -> InternalError {
        match Self::complete(permit, now_secs) {
            Ok(()) => error,
            Err(_completion_error) => error,
        }
    }

    pub fn recover(permit: &CostGuardPermit, now_secs: u64) -> Result<(), InternalError> {
        let result = CostGuardOps::recover(permit, now_secs);
        Self::reconcile_after_success(&result);
        result
    }

    pub fn recover_replay_settlement(
        settlement: &ReplayCostGuardSettlement,
        now_secs: u64,
    ) -> Result<(), InternalError> {
        let result = CostGuardOps::recover_replay_settlement(settlement, now_secs);
        Self::reconcile_after_success(&result);
        result
    }

    #[must_use]
    pub fn recover_after_failure(
        permit: &CostGuardPermit,
        now_secs: u64,
        error: InternalError,
    ) -> InternalError {
        match Self::recover(permit, now_secs) {
            Ok(()) => error,
            Err(_recovery_error) => error,
        }
    }

    fn reconcile_after_success(result: &Result<(), InternalError>) {
        if result.is_ok() {
            IntentCleanupWorkflow::reconcile_after_terminal();
        }
    }
}

#[must_use]
pub fn map_cost_guard_reserve_error(err: CostGuardReserveError) -> InternalError {
    err.into()
}
