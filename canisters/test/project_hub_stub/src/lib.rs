//! Minimal directory-bearing hub canister for keyed instance placement tests.

#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::auth::AuthApi,
    api::canister::placement::DirectoryApi,
    cdk::candid::Principal,
    dto::{
        auth::{DelegatedToken, SignedRoleAttestation},
        placement::directory::{DirectoryEntryStatusResponse, DirectoryRecoveryResponse},
    },
    ids::cap,
    prelude::*,
};
use project_protocol_stub::project_instance_record_visit_endpoint;

const PROJECTS_POOL: &str = "projects";

canic::start!();

canic::canic_internal_client! {
    struct ProjectInstanceInternalClient {
        fn record_visit = project_instance_record_visit_endpoint; (
            project_key: String,
        ) -> ();
    }
}

// Keep the test hub setup hook empty.
async fn canic_setup() {}

// Keep the test hub install hook empty.
async fn canic_install(_args: Option<Vec<u8>>) {}

// Keep the test hub upgrade hook empty.
async fn canic_upgrade() {}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn signer_verify_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

#[canic_update(requires(auth::authenticated()))]
async fn signer_verify_token_any(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
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

/// Notify one project instance through the protected internal-call client path.
#[canic_update]
async fn notify_project_instance(instance_id: Principal, project_key: String) -> Result<(), Error> {
    ProjectInstanceInternalClient::new(instance_id)
        .record_visit(project_key)
        .await
}

canic::finish!();
