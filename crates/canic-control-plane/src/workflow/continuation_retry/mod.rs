//! Orchestrate one charged business step within the existing native timer owner.
use crate::ops::continuation_retry::{self, Selection};
use canic_contracts::dto::operation_retry::{Operation, ResumeRequest, Status};
use canic_core::{
    api::timer::TimerApi,
    control_plane_support::{error::InternalError, ops::ic::IcOps},
};
use ic_timers::{OnceDecision, TimerCompletion, TimerSchedule};
use std::{future::Future, time::Duration};

pub const fn schedule(selection: &Selection) -> Option<TimerSchedule> {
    match selection {
        Selection::Ready(_) => Some(TimerSchedule::After(Duration::ZERO)),
        Selection::WaitUntil(deadline) => Some(TimerSchedule::At(*deadline)),
        Selection::Idle => None,
    }
}

pub fn decision(selection: Selection) -> OnceDecision {
    match selection {
        Selection::Ready(_) => OnceDecision::ContinueImmediately,
        Selection::WaitUntil(deadline) => OnceDecision::RetryAfter(Duration::from_nanos(
            deadline.saturating_sub(IcOps::now_nanos()),
        )),
        Selection::Idle => OnceDecision::Stop,
    }
}

pub async fn run(
    operation: Operation,
    action: impl Future<Output = Result<bool, InternalError>>,
) -> TimerCompletion {
    let Ok(attempt) = continuation_retry::begin(operation, IcOps::now_nanos()) else {
        return TimerCompletion::invariant_failure(0);
    };
    let result = action.await;
    let Ok(progressed) = continuation_retry::finish(attempt, result.as_ref().err()) else {
        return TimerCompletion::invariant_failure(0);
    };
    match result {
        Ok(complete) if complete || progressed => TimerCompletion::success(1),
        Ok(_) => TimerCompletion::no_work(),
        Err(error)
            if crate::policy::continuation_retry::requires_review(
                error.public_error().raw_code(),
            ) =>
        {
            // The journal now owns the review refusal. Other operations sharing this
            // native owner remain eligible; this is not a broken timer invariant.
            TimerCompletion::no_work()
        }
        Err(_) => TimerCompletion::retryable_failure(0),
    }
}

pub fn status(operation: Operation) -> Result<Status, InternalError> {
    continuation_retry::status(operation)
}

pub fn resume(request: ResumeRequest) -> Result<Status, InternalError> {
    TimerApi::require_active()?;
    let owner = match request.operation {
        Operation::CoordinatorProvisioning { .. } => "coordinator_component_provisioning",
        Operation::CoordinatorAdmission { .. }
        | Operation::CoordinatorFunding { .. }
        | Operation::CoordinatorRootRemoval { .. } => "coordinator_transition",
        Operation::RootAllocation { .. } => "component_allocation",
        Operation::ComponentRemoval { .. } | Operation::SubtreeRemoval { .. } => {
            "component_retirement"
        }
    };
    let identity = ic_timers::TimerIdentity::try_new("canic", owner, "advance")
        .map_err(|_| InternalError::invariant())?;
    if ic_timers::timer_snapshot(&identity)
        .map_err(|_| InternalError::unavailable())?
        .is_some_and(|snapshot| {
            snapshot.registration_status() == ic_timers::TimerRegistrationStatus::Running
        })
    {
        return Err(InternalError::unavailable());
    }
    let result = continuation_retry::resume_operation(request)?;
    match request.operation {
        #[cfg(feature = "fleet-coordinator-canister")]
        Operation::CoordinatorAdmission { .. }
        | Operation::CoordinatorFunding { .. }
        | Operation::CoordinatorProvisioning { .. }
        | Operation::CoordinatorRootRemoval { .. } => {
            crate::workflow::fleet_coordinator::FleetCoordinatorWorkflow::resume_after_restart()?;
        }
        #[cfg(feature = "root-control-plane")]
        Operation::RootAllocation { operation_id } => {
            crate::workflow::component_registry::schedule_component_allocation(operation_id)?;
        }
        #[cfg(feature = "root-control-plane")]
        Operation::ComponentRemoval {
            component,
            operation_id,
        } => crate::workflow::component_registry::schedule_component_removal(
            component,
            operation_id,
        )?,
        #[cfg(feature = "root-control-plane")]
        Operation::SubtreeRemoval {
            component,
            operation_id,
        } => {
            crate::workflow::component_registry::schedule_subtree_removal(component, operation_id)?;
        }
        #[cfg(not(all(feature = "fleet-coordinator-canister", feature = "root-control-plane")))]
        _ => return Err(InternalError::invalid_input()),
    }
    Ok(result)
}
