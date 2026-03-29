//! Minimal non-root canister for delegation proof tests.

#![allow(clippy::unused_async)]

#[cfg(canic_test_delegation_material)]
use canic::dto::auth::DelegationProof;
use canic::{
    Error,
    api::auth::DelegationApi,
    cdk::candid::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, SignedRoleAttestation},
    ids::cap,
    prelude::*,
};
use canic_internal::canister::USER_SHARD;

canic::start!(USER_SHARD);

async fn canic_setup() {}
async fn canic_install(_args: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_update]
async fn signer_issue_token(claims: DelegatedTokenClaims) -> Result<DelegatedToken, Error> {
    DelegationApi::issue_token(claims).await
}

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

#[canic_update(requires(caller::is_root()))]
async fn signer_guard_is_root() -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(caller::is_controller()))]
async fn signer_guard_is_controller() -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(caller::is_parent()))]
async fn signer_guard_is_parent() -> Result<(), Error> {
    Ok(())
}

#[canic_update(internal, requires(caller::is_registered_to_subnet()))]
async fn signer_guard_is_registered_to_subnet() -> Result<(), Error> {
    Ok(())
}

canic::export_candid!();
