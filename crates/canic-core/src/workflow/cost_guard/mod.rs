//! Module: workflow::cost_guard
//!
//! Responsibility: reserve cost permits with expiry scheduling and map public failures.
//! Does not own: quota accounting, cycle reservations, cleanup execution, or command policy.
//! Boundary: costed workflows reserve here before crossing external-effect boundaries.

use crate::{
    InternalError,
    dto::error::Error,
    model::replay::ReplayCostGuardSettlement,
    ops::cost_guard::{
        CostGuardOps, CostGuardPermit, CostGuardRequest, CostGuardReserveError,
        CostGuardReservePublicKind,
    },
    workflow::runtime::intent::IntentCleanupWorkflow,
};

/// Workflow authority that couples cost-intent mutation to finite-expiry scheduling.
pub struct CostGuardWorkflow;

impl CostGuardWorkflow {
    pub fn reserve(request: CostGuardRequest) -> Result<CostGuardPermit, CostGuardReserveError> {
        let permit = CostGuardOps::reserve(request)?;
        let settlement = permit.replay_settlement();
        IntentCleanupWorkflow::schedule_intent(settlement.quota_intent_id)
            .map_err(CostGuardReserveError::Store)?;
        IntentCleanupWorkflow::schedule_intent(settlement.reservation_intent_id)
            .map_err(CostGuardReserveError::Store)?;
        Ok(permit)
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

    pub fn recover(permit: &CostGuardPermit, now_secs: u64) -> Result<(), InternalError> {
        let result = CostGuardOps::recover(permit, now_secs);
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
            Err(recovery_error) => error.with_diagnostic_context(format!(
                "cost guard recovery failed for reservation {}: {recovery_error}",
                permit.reservation_id
            )),
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
    match err.public_kind() {
        Some(CostGuardReservePublicKind::InvalidInput) => {
            InternalError::public(Error::invalid(err.to_string()))
        }
        Some(CostGuardReservePublicKind::ResourceExhausted) => {
            InternalError::public(Error::exhausted(err.to_string()))
        }
        None => err.into(),
    }
}
