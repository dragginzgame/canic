//! Module: workflow::component_registry::lifecycle_drivers::child_timer
//!
//! Responsibility: retain one native continuation for requested child allocations.
//! Does not own: effect journals, retry budgets, or callback execution.
//! Boundary: each callback advances one eligible child step and preserves authority fences.

use crate::{
    ops::component_registry::ComponentRegistryOps,
    view::component_registry::RootComponentChildContinuationView,
};
use canic_contracts::ids::ComponentInstanceId;
use canic_core::{
    api::timer::TimerApi,
    control_plane_support::{error::InternalError, ops::ic::IcOps},
};
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, RegisterError, TimerCompletion, TimerError,
    TimerIdentity, TimerSchedule, TimerStart, register_once,
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static CONTINUATION: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
    static LAST_CHILD: RefCell<Option<ChildOperation>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ChildOperation {
    component: ComponentInstanceId,
    operation_id: [u8; 32],
}

#[derive(Debug, Eq, PartialEq)]
enum Selection {
    Ready(ChildOperation),
    WaitUntil(u64),
    Idle,
}

fn identity() -> Result<TimerIdentity, InternalError> {
    TimerIdentity::try_new("canic", "component_child_allocation", "advance")
        .map_err(|_| InternalError::invariant())
}

fn select(
    pending: Vec<RootComponentChildContinuationView>,
    previous: Option<ChildOperation>,
    now_ns: u64,
) -> Selection {
    let mut first = None;
    let mut next = None;
    let mut deadline: Option<u64> = None;
    for child in pending {
        if let Some(failure) = child.last_failure {
            if !super::child_failure_can_continue(
                failure.diagnostic_code,
                failure.consecutive_failures,
            ) {
                continue;
            }
            if failure.retry_at_ns > now_ns {
                deadline =
                    Some(deadline.map_or(failure.retry_at_ns, |old| old.min(failure.retry_at_ns)));
                continue;
            }
        }
        let key = ChildOperation {
            component: child.component,
            operation_id: child.operation_id,
        };
        first = Some(first.map_or(key, |old: ChildOperation| old.min(key)));
        if previous.is_none_or(|previous| key > previous) {
            next = Some(next.map_or(key, |old: ChildOperation| old.min(key)));
        }
    }
    next.or(first).map_or_else(
        || deadline.map_or(Selection::Idle, Selection::WaitUntil),
        Selection::Ready,
    )
}

fn pending() -> Result<Selection, InternalError> {
    let children = ComponentRegistryOps::child_continuations()?;
    // Empty registries need neither a platform clock nor a native registration.
    if children.is_empty() {
        return Ok(Selection::Idle);
    }
    Ok(select(
        children,
        LAST_CHILD.with(|last| *last.borrow()),
        IcOps::now_nanos(),
    ))
}

pub(in crate::workflow::component_registry) fn schedule() -> Result<(), InternalError> {
    let schedule = match pending()? {
        Selection::Ready(_) => TimerSchedule::After(Duration::ZERO),
        Selection::WaitUntil(deadline) => TimerSchedule::At(deadline),
        Selection::Idle => return cancel_pending(),
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
    if TimerApi::require_active().is_err() || matches!(pending(), Ok(Selection::Idle) | Err(_)) {
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
    let key = match pending() {
        Ok(Selection::Ready(key)) => key,
        selection => return result(TimerCompletion::no_work(), selection),
    };
    let before_failure =
        match ComponentRegistryOps::child_allocation(key.component, key.operation_id) {
            Ok(Some(child)) => child.last_failure,
            _ => {
                return OnceRunResult::new(
                    TimerCompletion::invariant_failure(0),
                    OnceDecision::Stop,
                );
            }
        };
    LAST_CHILD.with(|last| *last.borrow_mut() = Some(key));
    let completion = match Box::pin(super::advance_component_child_allocation_once(
        key.component,
        key.operation_id,
    ))
    .await
    {
        Ok(_) => TimerCompletion::success(1),
        Err(error) => {
            canic_core::log!(
                canic_core::log::Topic::Fleet,
                Warn,
                "child allocation attempt failed operation_id={:?} diagnostic={}",
                key.operation_id,
                error.code()
            );
            // A failed diagnostic commit must not turn into immediate unbounded retry.
            if ComponentRegistryOps::child_allocation(key.component, key.operation_id)
                .ok()
                .flatten()
                .is_none_or(|child| {
                    child.last_failure.is_none() || child.last_failure == before_failure
                })
            {
                return OnceRunResult::new(
                    TimerCompletion::invariant_failure(0),
                    OnceDecision::Stop,
                );
            }
            TimerCompletion::retryable_failure(0)
        }
    };
    result(completion, pending())
}

fn result(
    completion: TimerCompletion,
    selection: Result<Selection, InternalError>,
) -> OnceRunResult {
    let decision = match selection {
        Ok(Selection::Ready(_)) => OnceDecision::ContinueImmediately,
        Ok(Selection::WaitUntil(deadline)) => OnceDecision::RetryAfter(Duration::from_nanos(
            deadline.saturating_sub(IcOps::now_nanos()),
        )),
        Ok(Selection::Idle) => OnceDecision::Stop,
        Err(_) => {
            return OnceRunResult::new(TimerCompletion::invariant_failure(0), OnceDecision::Stop);
        }
    };
    OnceRunResult::new(completion, decision)
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
    use crate::view::component_registry::RootComponentChildAllocationFailureView;
    use canic_contracts::diagnostics::codes;

    fn child(id: u8) -> RootComponentChildContinuationView {
        RootComponentChildContinuationView {
            component: ComponentInstanceId::from_generated_bytes([1; 32]),
            operation_id: [id; 32],
            last_failure: None,
        }
    }

    fn key(id: u8) -> ChildOperation {
        ChildOperation {
            component: child(id).component,
            operation_id: [id; 32],
        }
    }

    #[test]
    fn ready_children_rotate_without_depending_on_registry_iteration_order() {
        let children = vec![child(3), child(1), child(2)];
        for (previous, expected) in [
            (None, 1),
            (Some(key(1)), 2),
            (Some(key(2)), 3),
            (Some(key(3)), 1),
        ] {
            assert_eq!(
                select(children.clone(), previous, 0),
                Selection::Ready(key(expected))
            );
        }
    }

    #[test]
    fn retained_deadlines_and_exhausted_budgets_survive_selection() {
        let mut waiting = child(1);
        waiting.last_failure = Some(RootComponentChildAllocationFailureView {
            diagnostic_code: codes::STATE_UNAVAILABLE.raw_code().raw(),
            platform_rejection: None,
            failed_at_ns: 1,
            consecutive_failures: 1,
            retry_at_ns: 50,
        });
        assert_eq!(
            select(vec![waiting.clone()], None, 49),
            Selection::WaitUntil(50)
        );
        assert_eq!(
            select(vec![waiting.clone()], None, 50),
            Selection::Ready(key(1))
        );
        assert_eq!(
            select(vec![waiting.clone(), child(2)], None, 49),
            Selection::Ready(key(2))
        );
        waiting.last_failure.as_mut().unwrap().consecutive_failures = 16;
        assert_eq!(select(vec![waiting.clone()], None, 100), Selection::Idle);
        let failure = waiting.last_failure.as_mut().unwrap();
        failure.consecutive_failures = 1;
        failure.diagnostic_code = codes::AUTHORITY_CONFLICT.raw_code().raw();
        assert_eq!(select(vec![waiting], None, 100), Selection::Idle);
    }

    #[test]
    fn absent_work_does_not_create_a_claim_while_suspended() {
        TimerApi::restore_snapshot_suspension(true);
        assert!(schedule().is_ok());
        assert!(!recover());
        assert!(CONTINUATION.with(|slot| slot.borrow().is_none()));
        TimerApi::restore_snapshot_suspension(false);
    }
}
