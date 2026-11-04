//! Module: workflow::fleet_coordinator::provisioning_timer
//!
//! Responsibility: retain one native claim for Coordinator's durable provisioning demand.
//! Does not own: operation records, paid-effect reconciliation, or callback execution.
//! Boundary: replay requests a successor; recovery never replaces a live callback.

use crate::ops::fleet_coordinator::FleetCoordinatorOps;
use crate::{
    ops::continuation_retry::{self, Selection},
    workflow::continuation_retry as retry_workflow,
};
use canic_contracts::dto::component_provisioning::FleetComponentProvisioningStatusResponse;
use canic_contracts::dto::operation_retry::{Operation, State};
use canic_core::{
    api::timer::TimerApi,
    control_plane_support::{error::InternalError, ops::ic::IcOps},
};
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, RegisterError, TimerCompletion, TimerError,
    TimerIdentity, TimerStart, register_once,
};
use std::cell::RefCell;

thread_local! {
    static PROVISIONING: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
}

fn identity() -> Result<TimerIdentity, InternalError> {
    TimerIdentity::try_new("canic", "coordinator_component_provisioning", "advance")
        .map_err(|_| InternalError::invariant())
}

fn pending_operation() -> Result<Option<FleetComponentProvisioningStatusResponse>, InternalError> {
    FleetCoordinatorOps::pending_component_provisioning()
}

fn pending() -> Result<Selection, InternalError> {
    let Some(operation) = pending_operation()? else {
        return Ok(Selection::Idle);
    };
    let key = Operation::CoordinatorProvisioning {
        operation_id: operation.operation_id,
    };
    let retry = continuation_retry::status(key)?;
    if retry.state != State::Active {
        return Ok(Selection::Idle);
    }
    let now = IcOps::now_nanos();
    let domain_delay = super::component_provisioning_pending_delay(&operation, now);
    let deadline = now
        .saturating_add(u64::try_from(domain_delay.as_nanos()).unwrap_or(u64::MAX))
        .max(retry.retry_at_ns);
    Ok(if deadline > now {
        Selection::WaitUntil(deadline)
    } else {
        Selection::Ready(key)
    })
}

/// Reuse the exact claim, including a running callback's successor, for current durable work.
pub(super) fn schedule() -> Result<(), InternalError> {
    let Some(schedule) = retry_workflow::schedule(&pending()?) else {
        return cancel_pending();
    };
    TimerApi::require_active()?;
    super::recovery::ensure()?;
    PROVISIONING.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| InternalError::unavailable())?;
        if let Some(registration) = slot.as_ref() {
            match registration.ensure_scheduled(schedule) {
                Ok(()) => return Ok(()),
                Err(TimerError::RegistrationExpired) => *slot = None,
                Err(error) => return Err(registration_error(error)),
            }
        }
        let registration = register_once(
            identity()?,
            TimerStart::RemoveWhenStopped { initial: schedule },
            |_| canic_core::perf::with_async_context(run()),
        )
        .map_err(registration_error)?;
        *slot = Some(registration);
        Ok(())
    })
}

fn cancel_pending() -> Result<(), InternalError> {
    PROVISIONING.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| InternalError::unavailable())?;
        if let Some(registration) = slot.as_ref() {
            match registration.cancel() {
                Ok(()) => {}
                Err(TimerError::RegistrationExpired) => *slot = None,
                Err(error) => return Err(registration_error(error)),
            }
        }
        Ok(())
    })
}

/// Reconstruct missing, cancelled or abandoned work without stealing native running authority.
pub(super) fn recover() -> bool {
    if TimerApi::require_active().is_err() {
        return false;
    }
    let Ok(Selection::Ready(_) | Selection::WaitUntil(_)) = pending() else {
        return false;
    };
    let Ok(identity) = identity() else {
        return false;
    };
    if !TimerApi::registration_requires_recovery(&identity, true).unwrap_or(false) {
        return false;
    }
    schedule().is_ok()
}

async fn run() -> OnceRunResult {
    if TimerApi::require_active().is_err() {
        return OnceRunResult::new(TimerCompletion::no_work(), OnceDecision::Stop);
    }
    match pending() {
        Ok(Selection::Ready(_)) => {}
        Ok(selection) => {
            return OnceRunResult::new(
                TimerCompletion::no_work(),
                retry_workflow::decision(selection),
            );
        }
        Err(_) => {
            return OnceRunResult::new(TimerCompletion::invariant_failure(0), OnceDecision::Stop);
        }
    }
    let operation = match pending_operation() {
        Ok(Some(operation)) => operation,
        Ok(None) => return OnceRunResult::new(TimerCompletion::no_work(), OnceDecision::Stop),
        Err(_) => {
            return OnceRunResult::new(TimerCompletion::invariant_failure(0), OnceDecision::Stop);
        }
    };
    let result = Box::pin(super::advance_scheduled_component_provisioning(
        operation.operation_id,
        operation.plan_hash,
    ))
    .await;
    match pending() {
        Ok(selection) => {
            OnceRunResult::new(result.completion(), retry_workflow::decision(selection))
        }
        Err(_) => OnceRunResult::new(TimerCompletion::invariant_failure(0), OnceDecision::Stop),
    }
}

fn registration_error(error: TimerError) -> InternalError {
    match error {
        TimerError::Register(RegisterError::CapacityExceeded { .. })
        | TimerError::BackendCapacity => InternalError::resource_exhausted(),
        TimerError::RuntimeBusy => InternalError::unavailable(),
        _ => InternalError::invariant(),
    }
}

pub(super) fn has_pending() -> Result<bool, InternalError> {
    Ok(pending_operation()?.is_some())
}

/// Release only after the Coordinator has proved both operation owners idle.
pub(super) fn unregister_idle() -> Result<(), InternalError> {
    PROVISIONING.with(|slot| {
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
