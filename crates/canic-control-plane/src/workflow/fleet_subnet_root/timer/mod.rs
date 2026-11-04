//! Module: workflow::fleet_subnet_root::timer
//!
//! Responsibility: retain the native continuation for Root's durable removal journal.
//! Does not own: paid effects, removal authority, or callback execution.
//! Boundary: replay and reconstruction share one claim; recovery never steals a running callback.

use crate::{
    ops::component_registry::ComponentRegistryOps,
    view::component_registry::RootFleetSubnetDrainingView,
};
use canic_core::{api::timer::TimerApi, control_plane_support::error::InternalError};
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, RegisterError, TimerCompletion, TimerError,
    TimerIdentity, TimerSchedule, TimerStart, register_once,
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static REMOVAL: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
}

fn identity() -> Result<TimerIdentity, InternalError> {
    TimerIdentity::try_new("canic", "root_removal", "advance")
        .map_err(|_| InternalError::invariant())
}

pub(super) fn retained_removal() -> Result<Option<RootFleetSubnetDrainingView>, InternalError> {
    let Some(draining) = ComponentRegistryOps::current().and_then(|state| state.root_draining)
    else {
        return Ok(None);
    };
    ComponentRegistryOps::root_draining_if_present(draining.operation_id)
}

fn pending_operation() -> Result<Option<[u8; 32]>, InternalError> {
    Ok(retained_removal()?
        .filter(|removal| removal.root_deletion_preparation.is_none())
        .map(|removal| removal.operation_id))
}

/// Reuse one native claim for the exact accepted operation, including an in-flight successor.
pub(super) fn schedule(operation_id: [u8; 32]) -> Result<(), InternalError> {
    let Some(pending) = pending_operation()? else {
        return Ok(());
    };
    if pending != operation_id {
        return Err(InternalError::conflict());
    }
    TimerApi::require_active()?;
    crate::workflow::canister_pool::ensure_recovery_watchdog()?;
    let schedule = TimerSchedule::After(Duration::ZERO);
    REMOVAL.with(|slot| {
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
        *slot = Some(
            register_once(
                identity()?,
                TimerStart::RemoveWhenStopped { initial: schedule },
                |_| canic_core::perf::with_async_context(run()),
            )
            .map_err(registration_error)?,
        );
        Ok(())
    })
}

pub(super) fn recover() -> bool {
    if TimerApi::require_active().is_err() {
        return false;
    }
    let Ok(Some(operation_id)) = pending_operation() else {
        return false;
    };
    let Ok(identity) = identity() else {
        return false;
    };
    if !TimerApi::registration_requires_recovery(&identity, true).unwrap_or(false) {
        return false;
    }
    schedule(operation_id).is_ok()
}

async fn run() -> OnceRunResult {
    if TimerApi::require_active().is_err() {
        return OnceRunResult::new(TimerCompletion::no_work(), OnceDecision::Stop);
    }
    let operation_id = match pending_operation() {
        Ok(Some(operation_id)) => operation_id,
        Ok(None) => return OnceRunResult::new(TimerCompletion::no_work(), OnceDecision::Stop),
        Err(_) => {
            return OnceRunResult::new(TimerCompletion::invariant_failure(0), OnceDecision::Stop);
        }
    };
    match Box::pin(super::advance_root_removal_once(operation_id)).await {
        Ok(true) => OnceRunResult::new(TimerCompletion::success(1), OnceDecision::Stop),
        Ok(false) => OnceRunResult::new(
            TimerCompletion::success(1),
            OnceDecision::ContinueImmediately,
        ),
        Err(_) => OnceRunResult::new(
            TimerCompletion::retryable_failure(0),
            OnceDecision::RetryAfter(Duration::from_secs(1)),
        ),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_removal_does_not_create_work_while_suspended() {
        assert!(pending_operation().unwrap().is_none());
        TimerApi::restore_snapshot_suspension(true);
        assert!(schedule([1; 32]).is_ok());
        assert!(!recover());
        assert!(REMOVAL.with(|slot| slot.borrow().is_none()));
        TimerApi::restore_snapshot_suspension(false);
    }
}
