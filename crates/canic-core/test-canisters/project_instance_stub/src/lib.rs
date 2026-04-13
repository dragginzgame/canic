//! Minimal dedicated instance canister for directory placement tests.

#![allow(clippy::unused_async)]

use canic::{Error, api::canister::CanisterRole, cdk::types::Principal, prelude::*};

const PROJECT_INSTANCE: CanisterRole = CanisterRole::new("project_instance");

canic::start!(PROJECT_INSTANCE);

// Keep the test instance setup hook empty.
async fn canic_setup() {}

// Keep the test instance install hook empty.
async fn canic_install(_args: Option<Vec<u8>>) {}

// Keep the test instance upgrade hook empty.
async fn canic_upgrade() {}

/// Return this canister's own id so tests can confirm the instance is live.
#[canic_query]
async fn instance_id() -> Result<Principal, Error> {
    Ok(canic::cdk::api::canister_self())
}

canic::cdk::export_candid_debug!();
