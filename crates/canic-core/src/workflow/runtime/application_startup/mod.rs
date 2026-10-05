//! Module: workflow::runtime::application_startup
//!
//! Release-bound application initialization through one reusable native timer claim.
//!
//! The activation owner retains arguments and completion; this module owns only scheduling.

use crate::{
    InternalError,
    access::AccessError,
    dto::caller_authority::CallerAuthorityPublication,
    ops::{
        caller_authority::CallerAuthorityOps,
        storage::{StorageOpsError, fleet_activation::FleetActivationOps},
    },
    workflow::runtime::timer::{TimerError, require_active, retain_owned_once, with_owned_once},
};
use ic_timers::{
    DeclarationLifetime, OnceRegistration, TimerCompletion, TimerDirective, TimerIdentity,
    TimerRunResult, TimerSchedule, register_once,
};
use std::{cell::RefCell, future::Future, time::Duration};

thread_local! {
    static STARTUP: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
}

pub fn release(publication: CallerAuthorityPublication) -> Result<(), InternalError> {
    FleetActivationOps::release_application_startup(CallerAuthorityOps::publication_from_dto(
        publication,
    ))
    .map_err(StorageOpsError::from)?;
    Ok(())
}

pub fn require_started() -> Result<(), InternalError> {
    FleetActivationOps::require_application_started()
        .map_err(AccessError::from)
        .map_err(Into::into)
}

/// Recheck release authority between asynchronous application initialization hooks.
pub fn require_initialization_effect() -> Result<(), InternalError> {
    FleetActivationOps::application_startup_work()
        .map_err(AccessError::from)?
        .ok_or_else(InternalError::unavailable)?;
    Ok(())
}

/// This mandatory boundary is independent of the application's Boolean access expression.
pub fn require_endpoint_started(call: crate::ids::EndpointCall) -> Result<(), AccessError> {
    use crate::{
        domain::policy::pure::caller_authority::CallerAdmissionError,
        ops::runtime::{env::EnvOps, fleet_activation::FleetActivationRuntimeOps},
    };
    if EnvOps::is_root()
        || EnvOps::is_fleet_coordinator_runtime()
        || FleetActivationRuntimeOps::is_standalone_local()
    {
        return Ok(());
    }
    let role = EnvOps::canister_role()
        .map_err(|_| AccessError::from(CallerAdmissionError::AuthorityUnavailable))?;
    if role.is_wasm_store()
        || crate::domain::policy::pure::fleet_activation::is_nonroot_infrastructure_endpoint(call)
    {
        return Ok(());
    }
    FleetActivationOps::require_application_started().map_err(Into::into)
}

/// Exact retries request a successor on the same native claim; they cannot overlap hooks.
pub fn schedule<F: Future<Output = ()> + 'static>(
    hook: fn(Option<Vec<u8>>) -> F,
) -> Result<(), InternalError> {
    let pending = FleetActivationOps::application_startup_work()
        .map_err(AccessError::from)?
        .is_some();
    if with_owned_once(&STARTUP, |_| ())?.is_none() {
        let claim = register_once(
            TimerIdentity::try_new("canic", "application", "initialize")
                .map_err(TimerError::from)?,
            DeclarationLifetime::Retained,
            move |_| run(hook),
        )
        .map_err(TimerError::from)?;
        retain_owned_once(&STARTUP, claim)?;
    }
    if pending {
        require_active()?;
        with_owned_once(&STARTUP, |claim| {
            claim.ensure_scheduled(TimerSchedule::After(Duration::ZERO))
        })?
        .ok_or(TimerError::MissingClaim)?
        .map_err(TimerError::from)?;
    }
    Ok(())
}

async fn run<F: Future<Output = ()>>(hook: fn(Option<Vec<u8>>) -> F) -> TimerRunResult {
    let work = match FleetActivationOps::application_startup_work() {
        Ok(Some(work)) => work,
        Ok(None) => return TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop),
        Err(_) => {
            return TimerRunResult::new(
                TimerCompletion::invariant_failure(0),
                TimerDirective::Stop,
            );
        }
    };
    hook(work.arguments).await;
    match FleetActivationOps::complete_application_startup(&work.release) {
        Ok(()) => TimerRunResult::new(TimerCompletion::success(1), TimerDirective::Stop),
        Err(_) => TimerRunResult::new(TimerCompletion::invariant_failure(0), TimerDirective::Stop),
    }
}
