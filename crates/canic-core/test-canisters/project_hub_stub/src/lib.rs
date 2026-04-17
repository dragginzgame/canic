//! Minimal directory-bearing hub canister for keyed instance placement tests.

#![allow(clippy::unused_async)]

#[cfg(canic_test_delegation_material)]
use canic::dto::auth::DelegationProof;
use canic::{
    Error,
    api::auth::DelegationApi,
    api::canister::{CanisterRole, placement::DirectoryApi},
    cdk::candid::Principal,
    dto::auth::DelegationProvisionTargetKind,
    dto::{
        auth::{DelegatedToken, DelegationProofInstallRequest, SignedRoleAttestation},
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
async fn signer_bootstrap_delegated_session(
    token: DelegatedToken,
    delegated_subject: Principal,
    requested_ttl_secs: Option<u64>,
) -> Result<(), Error> {
    DelegationApi::set_delegated_session_subject(delegated_subject, token, requested_ttl_secs)
}

#[canic_update]
async fn signer_clear_delegated_session() -> Result<(), Error> {
    DelegationApi::clear_delegated_session();
    Ok(())
}

#[canic_query]
async fn signer_delegated_session_subject() -> Result<Option<Principal>, Error> {
    Ok(DelegationApi::delegated_session_subject())
}

// This endpoint is test-only and is compiled in when
// CANIC_TEST_DELEGATION_MATERIAL enables `canic_test_delegation_material`.
#[canic_update(internal, requires(caller::is_root()))]
#[cfg(canic_test_delegation_material)]
async fn signer_install_test_delegation_material(
    proof: DelegationProof,
    root_public_key: Vec<u8>,
    shard_public_key: Vec<u8>,
) -> Result<(), Error> {
    DelegationApi::install_test_delegation_material(proof, root_public_key, shard_public_key)
}

#[canic_query]
#[cfg(canic_test_delegation_material)]
async fn signer_current_signing_proof_test() -> Result<Option<DelegationProof>, Error> {
    Ok(DelegationApi::current_signing_proof_for_test())
}

#[canic_update]
async fn signer_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    DelegationApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

#[canic_update(internal, requires(caller::is_root()))]
async fn canic_delegation_set_verifier_proof(
    request: DelegationProofInstallRequest,
) -> Result<(), Error> {
    DelegationApi::store_proof(request, DelegationProvisionTargetKind::Verifier).await
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
