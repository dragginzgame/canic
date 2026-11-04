//! Module: workflow::component_provisioning::timer
//!
//! Responsibility: retain one native claim for Root's durable provisioning demand.
//! Does not own: operation records, paid-effect reconciliation, or callback execution.
//! Boundary: replay requests a successor; recovery never replaces a live callback.

use crate::{
    ops::component_provisioning::RootComponentProvisioningOps,
    view::component_provisioning::RootComponentProvisioningView,
};
use canic_core::{
    api::timer::TimerApi,
    control_plane_support::{error::InternalError, ops::ic::IcOps},
};
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, RegisterError, TimerCompletion, TimerError,
    TimerIdentity, TimerSchedule, TimerStart, register_once,
};
use std::cell::RefCell;

thread_local! {
    static PROVISIONING: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
}

fn identity() -> Result<TimerIdentity, InternalError> {
    TimerIdentity::try_new("canic", "root_component_provisioning", "advance")
        .map_err(|_| InternalError::invariant())
}

fn pending_operation() -> Result<Option<RootComponentProvisioningView>, InternalError> {
    Ok(
        RootComponentProvisioningOps::active_operation()?.filter(|operation| {
            operation
                .last_failure
                .as_ref()
                .is_none_or(|failure| failure.retry_at_ns.is_some())
        }),
    )
}

/// Reuse the exact claim, including a running callback's successor, for current durable work.
pub(super) fn schedule() -> Result<(), InternalError> {
    let Some(operation) = pending_operation()? else {
        return cancel_pending();
    };
    TimerApi::require_active()?;
    // Prepared Roots also need recovery; use the same watchdog as pool maintenance.
    crate::workflow::canister_pool::ensure_recovery_watchdog()?;
    let deadline = operation
        .last_failure
        .and_then(|failure| failure.retry_at_ns)
        .unwrap_or_else(IcOps::now_nanos);
    let schedule = TimerSchedule::At(deadline);
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
    let Ok(Some(_)) = pending_operation() else {
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
    match pending_operation() {
        Ok(Some(operation)) => {
            Box::pin(super::advance_scheduled_provisioning(
                operation.operation_id,
                operation.plan_hash,
            ))
            .await
        }
        Ok(None) => OnceRunResult::new(TimerCompletion::no_work(), OnceDecision::Stop),
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

#[cfg(test)]
mod tests {
    use super::*;
    use canic_contracts::diagnostics::codes;

    #[test]
    fn absent_work_does_not_create_a_claim_even_while_suspended() {
        assert!(pending_operation().unwrap().is_none());
        assert!(PROVISIONING.with(|slot| slot.borrow().is_none()));
        TimerApi::restore_snapshot_suspension(true);
        assert!(schedule().is_ok());
        assert!(!recover());
        TimerApi::restore_snapshot_suspension(false);
        assert!(schedule().is_ok());
        assert!(!recover());
        assert!(PROVISIONING.with(|slot| slot.borrow().is_none()));
    }

    #[test]
    fn registration_capacity_is_a_typed_retryable_refusal() {
        for error in [
            TimerError::Register(RegisterError::CapacityExceeded { max: 64 }),
            TimerError::BackendCapacity,
        ] {
            assert_eq!(
                registration_error(error).public_code(),
                Some(codes::CAPACITY_LIMIT)
            );
        }
        assert_eq!(
            registration_error(TimerError::RuntimeBusy).public_code(),
            Some(codes::STATE_UNAVAILABLE)
        );
    }
}
