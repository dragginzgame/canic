//! Canic-owned managed lifecycle and explicit admission guard qualification.

mod checkpoint;
mod initialization;
mod protocol;

use canic::{Error, prelude::*};
use std::cell::Cell;

/// Evidence that a permitted request reached application dispatch.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
struct ManagedGuardReceipt {
    caller: ic_platform::Principal,
    workflow_runs: u32,
}

thread_local! {
    static MANAGED_GUARD_WORKFLOW_RUNS: Cell<u32> = const { Cell::new(0) };
}

canic::start!(
    memory = initialization::memory(),
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

#[query(public, wire(id = protocol::PUBLIC, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn managed_guard_public_probe() -> Result<ic_platform::Principal, Error> {
    Ok(ic_platform::cdk::api::msg_caller())
}

#[update(public, wire(id = protocol::GUARD, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn managed_guard_admission_probe() -> Result<ManagedGuardReceipt, Error> {
    let caller = canic::fleet_admission::require_caller()?;
    let workflow_runs = MANAGED_GUARD_WORKFLOW_RUNS.with(|runs| {
        let next = runs.get().saturating_add(1);
        runs.set(next);
        next
    });
    Ok(ManagedGuardReceipt {
        caller: caller.into(),
        workflow_runs,
    })
}

#[update(public, wire(id = protocol::OWNED, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn managed_guard_owned_probe() -> Result<ic_platform::Principal, Error> {
    let caller = canic::fleet_admission::require_caller()?;
    let application_owner = ic_platform::principal::ExternalPrincipal::from_slice(&[17; 29]);
    if caller != application_owner {
        return Err(Error::from_registered(
            canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    Ok(caller.into())
}

#[query(public, wire(id = protocol::WORKFLOW_RUNS, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn managed_guard_workflow_runs() -> Result<u32, Error> {
    Ok(MANAGED_GUARD_WORKFLOW_RUNS.with(Cell::get))
}

#[query(requires(caller::is_fleet_admitted()), wire(id = protocol::PARITY, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn canic_fleet_admission_parity_probe() -> Result<ic_platform::Principal, Error> {
    Ok(ic_platform::cdk::api::msg_caller())
}

/// Exact application init bytes retained by the synchronous lifecycle owner.
#[query(public, wire(id = protocol::INITIALIZATION, request = protocol::REQUEST_LIMITS, response = protocol::INITIALIZATION_RESPONSE_LIMITS))]
fn managed_initialization_bytes() -> Result<Vec<u8>, Error> {
    Ok(initialization::bytes())
}

canic::finish!();
