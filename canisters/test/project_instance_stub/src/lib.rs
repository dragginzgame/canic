//! Minimal dedicated instance canister for placement-index tests.

#![expect(clippy::unused_async)]

use candid::Principal;
use canic::{
    Error,
    api::call::Call,
    dto::{
        auth::DelegatedToken,
        component_registry::{RootComponentAllocationResponse, RootPeerComponentAllocationRequest},
    },
    ids::cap,
    prelude::*,
    protocol::CANIC_ROOT_PEER_COMPONENT_ALLOCATE,
};
use ic_cdk::api::canister_self;

canic::start!();

// Keep the test instance setup hook empty.
async fn canic_setup() {}

// Keep the test instance install hook empty.
async fn canic_install(_args: Option<Vec<u8>>) {}

// Keep the test instance upgrade hook empty.
async fn canic_upgrade() {}

/// Return this canister's own id so tests can confirm the instance is live.
#[canic_query(public)]
async fn instance_id() -> Result<Principal, Error> {
    Ok(canister_self())
}

/// Verify one self-contained delegated token.
#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn instance_verify_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

/// Verify one self-contained delegated token for the project visit path.
#[canic_update(
    name = "project_instance_record_visit",
    requires(auth::authenticated(cap::VERIFY))
)]
async fn record_visit(token: DelegatedToken, project_key: String) -> Result<(), Error> {
    let _ = (token, project_key);
    Ok(())
}

/// Forward one peer allocation request so the target root observes this Component Child.
#[canic_update(public)]
async fn forward_peer_allocation(
    fleet_subnet_root: Principal,
    request: RootPeerComponentAllocationRequest,
) -> Result<RootComponentAllocationResponse, Error> {
    let response: Result<RootComponentAllocationResponse, Error> =
        Call::bounded_wait(fleet_subnet_root, CANIC_ROOT_PEER_COMPONENT_ALLOCATE)
            .with_arg(request)?
            .execute_candid()
            .await?;
    response
}

canic::finish!();
