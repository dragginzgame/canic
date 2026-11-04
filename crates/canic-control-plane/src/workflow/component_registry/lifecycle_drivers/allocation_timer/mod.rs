//! Module: workflow::component_registry::lifecycle_drivers::allocation_timer
//!
//! Responsibility: retain the native continuation for ordinary and peer allocations.
//! Does not own: aggregate provisioning, operation journals, or callback execution.
//! Boundary: replay shares one claim; reconstruction and recovery preserve accepted demand.

use crate::{
    ops::continuation_retry::{self, Selection},
    workflow::continuation_retry as retry_workflow,
};
use canic_contracts::dto::operation_retry::Operation;
use canic_core::control_plane_support::ops::ic::IcOps;

use crate::ops::component_registry::ComponentRegistryOps;
use canic_core::{api::timer::TimerApi, control_plane_support::error::InternalError};
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, RegisterError, TimerCompletion, TimerError,
    TimerIdentity, TimerStart, register_once,
};
use std::cell::RefCell;

thread_local! {
    static CONTINUATION: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
    static LAST_OPERATION: RefCell<Option<Operation>> = const { RefCell::new(None) };
}

fn identity() -> Result<TimerIdentity, InternalError> {
    TimerIdentity::try_new("canic", "component_allocation", "advance")
        .map_err(|_| InternalError::invariant())
}

fn pending() -> Result<Selection, InternalError> {
    let operations = ComponentRegistryOps::allocation_continuation_operations()?;
    if operations.is_empty() {
        return Ok(Selection::Idle);
    }
    continuation_retry::select(
        operations,
        LAST_OPERATION.with(|last| *last.borrow()),
        IcOps::now_nanos(),
    )
}

pub(in crate::workflow::component_registry) fn schedule() -> Result<(), InternalError> {
    let Some(schedule) = retry_workflow::schedule(&pending()?) else {
        return cancel_pending();
    };
    TimerApi::require_active()?;
    crate::workflow::canister_pool::ensure_recovery_watchdog()?;
    CONTINUATION.with(|slot| {
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

fn cancel_pending() -> Result<(), InternalError> {
    CONTINUATION.with(|slot| {
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

pub(in crate::workflow::component_registry) fn recover() -> bool {
    if TimerApi::require_active().is_err()
        || !matches!(pending(), Ok(Selection::Ready(_) | Selection::WaitUntil(_)))
    {
        return false;
    }
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
    LAST_OPERATION.with(|last| *last.borrow_mut() = Some(operation));
    let completion = retry_workflow::run(operation, async {
        match operation {
            Operation::RootAllocation { operation_id } => {
                Box::pin(super::advance_component_allocation_once(operation_id)).await
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

fn registration_error(error: TimerError) -> InternalError {
    match error {
        TimerError::Register(RegisterError::CapacityExceeded { .. })
        | TimerError::BackendCapacity => InternalError::resource_exhausted(),
        TimerError::RuntimeBusy => InternalError::unavailable(),
        _ => InternalError::invariant(),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_allocation_demand_does_not_create_a_claim_while_suspended() {
        TimerApi::restore_snapshot_suspension(true);
        assert!(schedule().is_ok());
        assert!(!recover());
        assert!(CONTINUATION.with(|slot| slot.borrow().is_none()));
        TimerApi::restore_snapshot_suspension(false);
    }
}
