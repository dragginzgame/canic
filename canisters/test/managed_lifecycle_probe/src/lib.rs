//! Canic-owned managed lifecycle and explicit admission guard qualification.

mod initialization;

use candid::CandidType;
use canic::{Error, prelude::*};
use std::cell::Cell;

/// Evidence that a permitted request reached application dispatch.
#[derive(CandidType, Clone, Debug, Eq, PartialEq)]
struct ManagedGuardReceipt {
    caller: candid::Principal,
    workflow_runs: u32,
}

thread_local! {
    static MANAGED_GUARD_WORKFLOW_RUNS: Cell<u32> = const { Cell::new(0) };
}

canic::start!(
    argument_limits = initialization::LIMITS,
    lifecycle_participant(
        init = initialization::install,
        post_upgrade = initialization::restore
    ),
);

#[expect(clippy::unused_async, reason = "framework lifecycle hook signature")]
async fn canic_setup() {}

async fn canic_install(_: Option<Vec<u8>>) {}

#[expect(clippy::unused_async, reason = "framework lifecycle hook signature")]
async fn canic_upgrade() {}

#[canic_query(public)]
fn managed_guard_public_probe() -> Result<candid::Principal, Error> {
    Ok(ic_cdk::api::msg_caller())
}

#[canic_update(public)]
fn managed_guard_admission_probe() -> Result<ManagedGuardReceipt, Error> {
    let caller = canic::fleet_admission::require_caller()?;
    let workflow_runs = MANAGED_GUARD_WORKFLOW_RUNS.with(|runs| {
        let next = runs.get().saturating_add(1);
        runs.set(next);
        next
    });
    Ok(ManagedGuardReceipt {
        caller,
        workflow_runs,
    })
}

#[canic_update(public)]
fn managed_guard_owned_probe() -> Result<candid::Principal, Error> {
    let caller = canic::fleet_admission::require_caller()?;
    let application_owner = candid::Principal::from_slice(&[17; 29]);
    if caller != application_owner {
        return Err(Error::from_registered(
            canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    Ok(caller)
}

#[canic_query(public)]
fn managed_guard_workflow_runs() -> Result<u32, Error> {
    Ok(MANAGED_GUARD_WORKFLOW_RUNS.with(Cell::get))
}

#[canic_query(requires(caller::is_fleet_admitted()))]
fn canic_fleet_admission_parity_probe() -> Result<candid::Principal, Error> {
    Ok(ic_cdk::api::msg_caller())
}

/// Exact application init bytes retained by the synchronous lifecycle owner.
#[canic_query(public)]
fn managed_initialization_bytes() -> Result<Vec<u8>, Error> {
    Ok(initialization::bytes())
}

canic::finish!();
