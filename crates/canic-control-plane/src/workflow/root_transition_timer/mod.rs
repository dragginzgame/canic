//! Module: workflow::root_transition_timer
//!
//! Responsibility: retain one native owner for Root admission and Registry synchronization.
//! Does not own: participant journals, Registry authority, or callback execution.
//! Boundary: exact replay and recovery select retained demand without replacing a live call.

use crate::ops::{fleet_registry_mirror::FleetRegistryMirrorOps, root_admission::RootAdmissionOps};
use canic_core::{api::timer::TimerApi, control_plane_support::error::InternalError};
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, RegisterError, TimerCompletion, TimerError,
    TimerIdentity, TimerSchedule, TimerStart, register_once,
};
use std::{cell::RefCell, time::Duration};

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Work {
    Admission([u8; 32]),
    Registry([u8; 32]),
}

thread_local! {
    static REGISTRATION: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
    static LAST: RefCell<Option<Work>> = const { RefCell::new(None) };
}

fn identity() -> Result<TimerIdentity, InternalError> {
    TimerIdentity::try_new("canic", "root_transition", "advance")
        .map_err(|_| InternalError::invariant())
}

fn pending() -> Result<Option<Work>, InternalError> {
    let mut work = Vec::new();
    if let Some(operation) = RootAdmissionOps::pending_operation()? {
        work.push(Work::Admission(operation));
    }
    let mirror = FleetRegistryMirrorOps::current();
    if let Some(candidate) = mirror.candidate {
        let completed = mirror.active.is_some()
            && mirror
                .synchronization
                .is_some_and(|receipt| receipt.operation_id == candidate.operation_id);
        if !completed {
            work.push(Work::Registry(candidate.operation_id));
        }
    }
    let previous = LAST.with(|last| *last.borrow());
    Ok(work
        .iter()
        .copied()
        .find(|item| previous.is_none_or(|last| *item > last))
        .or_else(|| work.first().copied()))
}

/// Reconcile the one declaration from current durable demand; no replay adds a chain.
pub fn schedule() -> Result<(), InternalError> {
    let desired = pending()?.map(|_| TimerSchedule::After(Duration::ZERO));
    if desired.is_some() {
        TimerApi::require_active()?;
        super::canister_pool::ensure_recovery_watchdog()?;
    }
    REGISTRATION.with(|slot| {
        let mut registration = slot
            .try_borrow_mut()
            .map_err(|_| InternalError::unavailable())?;
        let Some(schedule) = desired else {
            if let Some(claim) = registration.as_ref() {
                match claim.cancel() {
                    Ok(()) => {}
                    Err(TimerError::RegistrationExpired) => *registration = None,
                    Err(error) => return Err(registration_error(error)),
                }
            }
            return Ok(());
        };
        if let Some(claim) = registration.as_ref() {
            match claim.ensure_scheduled(schedule) {
                Ok(()) => return Ok(()),
                Err(TimerError::RegistrationExpired) => *registration = None,
                Err(error) => return Err(registration_error(error)),
            }
        }
        *registration = Some(
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

/// Upgrade reconstruction respects the authority suspension restored by Core.
pub fn resume_after_restart() -> Result<(), InternalError> {
    if TimerApi::require_active().is_err() {
        return Ok(());
    }
    schedule()
}

/// The shared Root watchdog repairs only absent or confirmed-abandoned native work.
pub fn recover() -> bool {
    if TimerApi::require_active().is_err() || !matches!(pending(), Ok(Some(_))) {
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
    let work = match pending() {
        Ok(Some(work)) => work,
        Ok(None) => return OnceRunResult::new(TimerCompletion::no_work(), OnceDecision::Stop),
        Err(_) => {
            return OnceRunResult::new(TimerCompletion::invariant_failure(0), OnceDecision::Stop);
        }
    };
    LAST.with(|last| *last.borrow_mut() = Some(work));
    let result = match work {
        Work::Admission(operation) => {
            Box::pin(super::root_admission::advance_once(operation)).await
        }
        Work::Registry(operation) => {
            Box::pin(super::fleet_registry_mirror::advance_registry_synchronization_once(operation))
                .await
        }
    };
    let (completion, retry) = match result {
        Ok(_) => (
            TimerCompletion::success(1),
            OnceDecision::ContinueImmediately,
        ),
        Err(_) => (
            TimerCompletion::retryable_failure(0),
            OnceDecision::RetryAfter(Duration::from_secs(1)),
        ),
    };
    match pending() {
        Ok(Some(_)) => OnceRunResult::new(completion, retry),
        Ok(None) => OnceRunResult::new(completion, OnceDecision::Stop),
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

    #[test]
    fn absent_demand_does_not_allocate_a_native_claim() {
        assert!(pending().unwrap().is_none());
        TimerApi::restore_snapshot_suspension(true);
        assert!(schedule().is_ok());
        assert!(!recover());
        assert!(REGISTRATION.with(|slot| slot.borrow().is_none()));
        TimerApi::restore_snapshot_suspension(false);
    }
}
