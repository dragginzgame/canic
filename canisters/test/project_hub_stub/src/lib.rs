//! Minimal placement-index hub canister for keyed instance placement tests.

#![expect(clippy::unused_async)]

use candid::Principal;
use canic::{
    Error,
    api::auth::AuthApi,
    api::canister::placement::PlacementIndexApi,
    dto::{
        auth::{DelegatedToken, SignedRoleAttestation},
        placement::index::{PlacementIndexRecoveryResponse, PlacementIndexStatusResponse},
    },
    ids::cap,
    prelude::*,
};

const PROJECTS_POOL: &str = "projects";

canic::start!();

// Keep the test hub setup hook empty.
async fn canic_setup() {}

// Keep the test hub install hook empty.
async fn canic_install(_args: Option<Vec<u8>>) {}

// Keep the test hub upgrade hook empty.
async fn canic_upgrade() {}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn verifier_verify_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

#[canic_update(requires(auth::authenticated()))]
async fn verifier_verify_token_any(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

#[canic_update(public)]
async fn verifier_clear_delegated_session() -> Result<(), Error> {
    AuthApi::clear_delegated_session();
    Ok(())
}

#[canic_query(public)]
async fn verifier_delegated_session_subject() -> Result<Option<Principal>, Error> {
    Ok(AuthApi::delegated_session_subject())
}

#[canic_update(public)]
async fn verifier_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    AuthApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

#[canic_update(requires(auth::attested_local_subnet()))]
async fn verifier_require_attested_local_subnet(
    attestation: SignedRoleAttestation,
) -> Result<(), Error> {
    let _ = attestation;
    Ok(())
}

/// Resolve one logical project key to a dedicated instance, creating it when absent.
#[canic_update(public)]
async fn resolve_project(project_key: String) -> Result<PlacementIndexStatusResponse, Error> {
    PlacementIndexApi::resolve_or_create(PROJECTS_POOL, project_key).await
}

/// Repair or release one placement index after partial failure.
#[canic_update(public)]
async fn recover_project(project_key: String) -> Result<PlacementIndexRecoveryResponse, Error> {
    PlacementIndexApi::recover_entry(PROJECTS_POOL, project_key).await
}

/// Look up the currently bound instance pid for one project key.
#[canic_query(public)]
async fn lookup_project(project_key: String) -> Result<Option<Principal>, Error> {
    Ok(PlacementIndexApi::lookup_key(PROJECTS_POOL, &project_key))
}

/// Return the full placement-index state for one project key.
#[canic_query(public)]
async fn lookup_project_entry(
    project_key: String,
) -> Result<Option<PlacementIndexStatusResponse>, Error> {
    Ok(PlacementIndexApi::lookup_entry(PROJECTS_POOL, &project_key))
}

canic::finish!();
