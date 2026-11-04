//! Module: workflow::fleet_coordinator::transition_timer
//!
//! Responsibility: retain native custody of accepted admission, applied funding and requested Root retirement.
//! Does not own: staged plans, durable policy decisions, or callback execution.
//! Boundary: replay shares one claim and recovery selects only validated pending work.

use crate::{
    ops::continuation_retry::{self, Selection},
    workflow::continuation_retry as retry_workflow,
};
use canic_contracts::dto::operation_retry::Operation;
use canic_core::control_plane_support::ops::ic::IcOps;

use crate::ops::fleet_coordinator::FleetCoordinatorOps;
use canic_core::{api::timer::TimerApi, control_plane_support::error::InternalError};
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, RegisterError, TimerCompletion, TimerError,
    TimerIdentity, reconcile_once,
};
use std::cell::RefCell;

thread_local! {
    static REGISTRATION: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
    static LAST: RefCell<Option<Operation>> = const { RefCell::new(None) };
}

fn identity() -> Result<TimerIdentity, InternalError> {
    TimerIdentity::try_new("canic", "coordinator_transition", "advance")
        .map_err(|_| InternalError::invariant())
}

fn pending() -> Result<Selection, InternalError> {
    let operations = FleetCoordinatorOps::continuation_operations()?
        .into_iter()
        .filter(|operation| !matches!(operation, Operation::CoordinatorProvisioning { .. }))
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return Ok(Selection::Idle);
    }
    continuation_retry::select(
        operations,
        LAST.with(|last| *last.borrow()),
        IcOps::now_nanos(),
    )
}

pub(super) fn schedule() -> Result<(), InternalError> {
    let desired = retry_workflow::schedule(&pending()?);
    if desired.is_some() {
        TimerApi::require_active()?;
        super::recovery::ensure()?;
    }
    REGISTRATION.with(|slot| {
        let mut registration = slot
            .try_borrow_mut()
            .map_err(|_| InternalError::unavailable())?;
        if desired.is_none() && registration.is_none() {
            return Ok(());
        }
        reconcile_once(&mut registration, &identity()?, desired, |_| {
            canic_core::perf::with_async_context(run())
        })
        .map_err(registration_error)
    })
}

pub(super) fn recover() -> bool {
    if TimerApi::require_active().is_err()
        || !matches!(pending(), Ok(Selection::Ready(_) | Selection::WaitUntil(_)))
    {
        return false;
    }
    let Ok(identity) = identity() else {
        return false;
    };
    TimerApi::registration_requires_recovery(&identity, true).unwrap_or(false) && schedule().is_ok()
}

async fn run() -> OnceRunResult {
    if TimerApi::require_active().is_err() {
        return OnceRunResult::new(TimerCompletion::no_work(), OnceDecision::Stop);
    }
    let operation = match pending() {
        Ok(Selection::Ready(operation)) => operation,
        Ok(selection) => {
            return OnceRunResult::new(
                TimerCompletion::no_work(),
                retry_workflow::decision(selection),
            );
        }
        Err(_) => {
            return OnceRunResult::new(TimerCompletion::invariant_failure(0), OnceDecision::Stop);
        }
    };
    LAST.with(|last| *last.borrow_mut() = Some(operation));
    let completion = retry_workflow::run(operation, async {
        match operation {
            Operation::CoordinatorAdmission { operation_id } => {
                Box::pin(super::advance_fleet_admission_once(operation_id)).await
            }
            Operation::CoordinatorFunding { operation_id } => {
                Box::pin(super::advance_funding_policy_rotation_once(operation_id)).await
            }
            Operation::CoordinatorRootRemoval { operation_id } => {
                Box::pin(super::advance_coordinator_root_removal_once(operation_id)).await
            }
            _ => Err(InternalError::invariant()),
        }
    })
    .await;
    match pending() {
        Ok(selection) => OnceRunResult::new(completion, retry_workflow::decision(selection)),
        Err(_) => OnceRunResult::new(TimerCompletion::invariant_failure(0), OnceDecision::Stop),
    }
}

pub(super) fn registration_error(error: TimerError) -> InternalError {
    match error {
        TimerError::Register(RegisterError::CapacityExceeded { .. })
        | TimerError::BackendCapacity => InternalError::resource_exhausted(),
        TimerError::RuntimeBusy => InternalError::unavailable(),
        _ => InternalError::invariant(),
    }
}

pub(super) fn has_pending() -> Result<bool, InternalError> {
    Ok(FleetCoordinatorOps::continuation_operations()?
        .iter()
        .any(|operation| !matches!(operation, Operation::CoordinatorProvisioning { .. })))
}

/// Release only after the Coordinator has proved both operation owners idle.
pub(super) fn unregister_idle() -> Result<(), InternalError> {
    REGISTRATION.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| InternalError::unavailable())?;
        if let Some(registration) = slot.take() {
            match registration.unregister() {
                Ok(()) | Err(TimerError::RegistrationExpired) => {}
                Err(error) => return Err(registration_error(error)),
            }
        }
        Ok(())
    })
}
