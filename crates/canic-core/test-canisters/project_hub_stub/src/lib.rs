//! Minimal directory-bearing hub canister for keyed instance placement tests.

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::auth::AuthApi,
    api::canister::{CanisterRole, placement::DirectoryApi},
    cdk::candid::Principal,
    dto::{
        auth::{DelegatedToken, SignedRoleAttestation},
        placement::directory::{DirectoryEntryStatusResponse, DirectoryRecoveryResponse},
    },
    ids::cap,
    prelude::*,
};

const PROJECT_HUB: CanisterRole = CanisterRole::new("project_hub");
const PROJECTS_POOL: &str = "projects";

canic::start!(PROJECT_HUB);

// Keep the test hub setup hook empty.
async fn canic_setup() {}

// Keep the test hub install hook empty.
async fn canic_install(_args: Option<Vec<u8>>) {}

// Keep the test hub upgrade hook empty.
async fn canic_upgrade() {}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn signer_verify_token(_token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(auth::authenticated()))]
async fn signer_verify_token_any(_token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

#[canic_update]
async fn signer_clear_delegated_session() -> Result<(), Error> {
    AuthApi::clear_delegated_session();
    Ok(())
}

#[canic_query]
async fn signer_delegated_session_subject() -> Result<Option<Principal>, Error> {
    Ok(AuthApi::delegated_session_subject())
}

#[canic_update]
async fn signer_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    AuthApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

/// Resolve one logical project key to a dedicated instance, creating it when absent.
#[canic_update]
async fn resolve_project(project_key: String) -> Result<DirectoryEntryStatusResponse, Error> {
    DirectoryApi::resolve_or_create(PROJECTS_POOL, project_key).await
}

/// Repair or release one directory entry after partial failure.
#[canic_update]
async fn recover_project(project_key: String) -> Result<DirectoryRecoveryResponse, Error> {
    DirectoryApi::recover_entry(PROJECTS_POOL, project_key).await
}

/// Look up the currently bound instance pid for one project key.
#[canic_query]
async fn lookup_project(project_key: String) -> Result<Option<Principal>, Error> {
    Ok(DirectoryApi::lookup_key(PROJECTS_POOL, &project_key))
}

/// Return the full directory entry state for one project key.
#[canic_query]
async fn lookup_project_entry(
    project_key: String,
) -> Result<Option<DirectoryEntryStatusResponse>, Error> {
    Ok(DirectoryApi::lookup_entry(PROJECTS_POOL, &project_key))
}

canic::cdk::export_candid_debug!();
