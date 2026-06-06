//! Minimal dedicated instance canister for directory placement tests.

#![expect(clippy::unused_async)]

use canic::{Error, cdk::types::Principal, dto::auth::DelegatedToken, ids::cap, prelude::*};

canic::start!();

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

/// Verify one self-contained delegated token.
#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn instance_verify_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

/// Protected app-style instance call accepted only from the project hub role.
#[canic_update(
    internal,
    name = "project_instance_record_visit",
    requires(caller::has_role("project_hub"))
)]
async fn record_visit(project_key: String) -> Result<(), Error> {
    let _ = project_key;
    Ok(())
}

canic::finish!();
