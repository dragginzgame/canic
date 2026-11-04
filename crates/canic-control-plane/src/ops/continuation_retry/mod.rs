//! Deterministic retry accounting; callers commit the replacement with their operation journal.

use crate::{
    policy::continuation_retry::{MAX_ATTEMPTS_WITHOUT_PROGRESS, delay_ns, requires_review},
    storage::stable::continuation_retry::ContinuationRetryRecord,
};
use canic_contracts::dto::operation_retry::{Operation, ResumeRequest, State, Status};
use canic_core::control_plane_support::error::InternalError;

/// Exact observable business progress, excluding retry diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Progress {
    #[cfg(feature = "fleet-coordinator-canister")]
    Admission(Box<canic_contracts::dto::fleet_admission::FleetAdmissionOperationStatusResponse>),
    #[cfg(feature = "fleet-coordinator-canister")]
    Funding(Box<canic_contracts::dto::fleet_coordinator::FleetFundingPolicyRotationStatusResponse>),
    #[cfg(feature = "fleet-coordinator-canister")]
    Provisioning(
        Box<canic_contracts::dto::component_provisioning::FleetComponentProvisioningStatusResponse>,
    ),
    #[cfg(feature = "fleet-coordinator-canister")]
    RootRemoval(
        Box<canic_contracts::dto::fleet_coordinator::CoordinatorRootRemovalOperationStatus>,
    ),
    #[cfg(feature = "root-control-plane")]
    Allocation(Box<crate::view::component_registry::RootComponentAllocationView>),
    #[cfg(feature = "root-control-plane")]
    ComponentRemoval(Box<crate::view::component_registry::RootComponentDrainingView>),
    #[cfg(feature = "root-control-plane")]
    SubtreeRemoval(Box<crate::view::component_registry::RootComponentSubtreeRemovalView>),
}

/// A charged attempt carries only comparison evidence, never new effect authority.
pub struct Attempt {
    operation: Operation,
    generation: u64,
    progress: Progress,
}

/// Select work using retained deadlines; a backed-off operation does not delay ready peers.
pub enum Selection {
    Ready(Operation),
    WaitUntil(u64),
    Idle,
}

pub fn select(
    operations: Vec<Operation>,
    previous: Option<Operation>,
    now_ns: u64,
) -> Result<Selection, InternalError> {
    let mut ready = Vec::new();
    let mut deadline: Option<u64> = None;
    for operation in operations {
        let (record, _) = read(operation)?;
        if suspended(&record) {
            continue;
        }
        if record.retry_at_ns > now_ns {
            deadline = Some(deadline.map_or(record.retry_at_ns, |old| old.min(record.retry_at_ns)));
        } else {
            ready.push(operation);
        }
    }
    ready.sort_unstable();
    Ok(ready
        .iter()
        .copied()
        .find(|key| previous.is_none_or(|last| *key > last))
        .or_else(|| ready.first().copied())
        .map_or_else(
            || deadline.map_or(Selection::Idle, Selection::WaitUntil),
            Selection::Ready,
        ))
}

pub fn begin(operation: Operation, now_ns: u64) -> Result<Attempt, InternalError> {
    let (record, progress) = read(operation)?;
    update(operation, |retry| charge(retry, now_ns))?;
    Ok(Attempt {
        operation,
        generation: record.generation,
        progress,
    })
}

/// Observe completion without refunding interrupted calls or overwriting an explicit resume.
pub fn finish(attempt: Attempt, error: Option<&InternalError>) -> Result<bool, InternalError> {
    let (_, progress) = read(attempt.operation)?;
    let progressed = progress != attempt.progress;
    update(attempt.operation, |record| {
        if record.generation == attempt.generation {
            observe(record, progressed, error);
        }
        Ok(())
    })?;
    Ok(progressed)
}

pub fn status(operation: Operation) -> Result<Status, InternalError> {
    let (record, _) = read(operation)?;
    Ok(Status {
        operation,
        generation: record.generation,
        attempts_without_progress: record.attempts_without_progress,
        maximum_attempts_without_progress: MAX_ATTEMPTS_WITHOUT_PROGRESS,
        retry_at_ns: record.retry_at_ns,
        diagnostic_code: record.diagnostic_code,
        state: if record.review_required {
            State::ReviewRequired
        } else if suspended(&record) {
            State::BudgetExhausted
        } else {
            State::Active
        },
    })
}

pub fn resume_operation(request: ResumeRequest) -> Result<Status, InternalError> {
    read(request.operation)?;
    update(request.operation, |record| {
        resume(record, request.expected_generation)
    })?;
    status(request.operation)
}

fn read(operation: Operation) -> Result<(ContinuationRetryRecord, Progress), InternalError> {
    let (record, progress) = match operation {
        #[cfg(feature = "fleet-coordinator-canister")]
        Operation::CoordinatorAdmission { .. }
        | Operation::CoordinatorFunding { .. }
        | Operation::CoordinatorProvisioning { .. }
        | Operation::CoordinatorRootRemoval { .. } => {
            crate::ops::fleet_coordinator::FleetCoordinatorOps::continuation_retry(operation)
        }
        #[cfg(feature = "root-control-plane")]
        Operation::RootAllocation { .. }
        | Operation::ComponentRemoval { .. }
        | Operation::SubtreeRemoval { .. } => {
            crate::ops::component_registry::ComponentRegistryOps::continuation_retry(operation)
        }
        #[cfg(not(all(feature = "fleet-coordinator-canister", feature = "root-control-plane")))]
        _ => Err(InternalError::invalid_input()),
    }?;
    validate(&record)?;
    Ok((record, progress))
}

pub const fn validate(record: &ContinuationRetryRecord) -> Result<(), InternalError> {
    if record.attempts_without_progress > MAX_ATTEMPTS_WITHOUT_PROGRESS
        || (record.review_required && record.diagnostic_code.is_none())
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn update(
    operation: Operation,
    mutate: impl FnOnce(&mut ContinuationRetryRecord) -> Result<(), InternalError>,
) -> Result<(), InternalError> {
    match operation {
        #[cfg(feature = "fleet-coordinator-canister")]
        Operation::CoordinatorAdmission { .. }
        | Operation::CoordinatorFunding { .. }
        | Operation::CoordinatorProvisioning { .. }
        | Operation::CoordinatorRootRemoval { .. } => {
            crate::ops::fleet_coordinator::FleetCoordinatorOps::update_continuation_retry(
                operation, mutate,
            )
        }
        #[cfg(feature = "root-control-plane")]
        Operation::RootAllocation { .. }
        | Operation::ComponentRemoval { .. }
        | Operation::SubtreeRemoval { .. } => {
            crate::ops::component_registry::ComponentRegistryOps::update_continuation_retry(
                operation, mutate,
            )
        }
        #[cfg(not(all(feature = "fleet-coordinator-canister", feature = "root-control-plane")))]
        _ => Err(InternalError::invalid_input()),
    }
}

/// A bounded retry window cannot be reopened by scheduling, replay or watchdog recovery.
pub const fn suspended(record: &ContinuationRetryRecord) -> bool {
    record.review_required || record.attempts_without_progress >= MAX_ATTEMPTS_WITHOUT_PROGRESS
}

/// Charge before dispatch, so an abandoned callback or upgrade cannot refund an attempt.
pub fn charge(record: &mut ContinuationRetryRecord, now_ns: u64) -> Result<(), InternalError> {
    if suspended(record) || now_ns < record.retry_at_ns {
        return Err(InternalError::unavailable());
    }
    record.attempts_without_progress += 1;
    record.retry_at_ns = now_ns
        .checked_add(delay_ns(record.attempts_without_progress))
        .ok_or_else(InternalError::invariant)?;
    Ok(())
}

/// Only proved work progress replenishes automatic attempts; changing errors does not.
pub fn observe(
    record: &mut ContinuationRetryRecord,
    progressed: bool,
    error: Option<&InternalError>,
) {
    if progressed {
        record.attempts_without_progress = 0;
        record.retry_at_ns = 0;
        record.diagnostic_code = None;
        record.review_required = false;
    }
    if let Some(error) = error {
        let code = error.public_error().raw_code();
        record.diagnostic_code = Some(code);
        record.review_required = requires_review(code) || error.provisioning_failure().is_some_and(|failure|
            failure.retry_category == canic_contracts::dto::component_provisioning::ProvisioningRetryCategory::ReviewRequired);
    }
}

/// Exact explicit operator resumption is idempotent and never changes effect authority.
pub fn resume(record: &mut ContinuationRetryRecord, generation: u64) -> Result<(), InternalError> {
    if generation.checked_add(1) == Some(record.generation) {
        return Ok(());
    }
    if generation != record.generation || !suspended(record) {
        return Err(InternalError::conflict());
    }
    record.generation = generation
        .checked_add(1)
        .ok_or_else(InternalError::invariant)?;
    record.attempts_without_progress = 0;
    record.retry_at_ns = 0;
    record.diagnostic_code = None;
    record.review_required = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_failures_and_restart_preserve_the_budget() {
        let mut record = ContinuationRetryRecord::default();
        for attempt in 1..=MAX_ATTEMPTS_WITHOUT_PROGRESS {
            let now = record.retry_at_ns;
            charge(&mut record, now).unwrap();
            let error = if attempt % 2 == 0 {
                InternalError::unavailable()
            } else {
                InternalError::resource_exhausted()
            };
            observe(&mut record, false, Some(&error));
            assert_eq!(record.attempts_without_progress, attempt);
        }
        assert!(suspended(&record));
        let before = record.clone();
        assert!(charge(&mut record, u64::MAX).is_err());
        assert_eq!(record, before);
        resume(&mut record, 0).unwrap();
        charge(&mut record, 0).unwrap();
        let resumed = record.clone();
        resume(&mut record, 0).unwrap();
        assert_eq!(record, resumed);
    }

    #[test]
    fn waiting_backoff_and_authority_review_do_not_discard_progress() {
        let mut record = ContinuationRetryRecord::default();
        charge(&mut record, 10).unwrap();
        assert!(charge(&mut record, 11).is_err());
        observe(&mut record, false, None);
        assert_eq!(record.diagnostic_code, None);
        observe(&mut record, false, Some(&InternalError::conflict()));
        assert!(suspended(&record));
        assert_eq!(record.attempts_without_progress, 1);
        observe(&mut record, true, None);
        assert!(!suspended(&record));
        assert_eq!(record.retry_at_ns, 0);
    }
}
