//! Minimal Fleet Subnet Root stub for Registry lifecycle tests.

#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::auth::AuthApi,
    dto::auth::{DelegatedToken, SignedRoleAttestation},
    prelude::*,
};

canic::start!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[canic_update(public)]
async fn root_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    AuthApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

#[canic_query(public)]
async fn root_now_secs() -> Result<u64, Error> {
    Ok(ic_cdk::api::time() / 1_000_000_000)
}

#[canic_update(requires(caller::is_controller()))]
async fn test_provision_chain_key_delegation_proof_for_issuer(
    issuer_pid: candid::Principal,
) -> Result<(), Error> {
    AuthApi::provision_chain_key_delegation_proof_for_issuer_root(issuer_pid).await
}

#[canic_update(public)]
async fn root_bootstrap_delegated_session(
    token: DelegatedToken,
    delegated_subject: candid::Principal,
    requested_ttl_ns: Option<u64>,
) -> Result<(), Error> {
    AuthApi::set_delegated_session_subject(delegated_subject, token, requested_ttl_ns)
}

#[canic_update(public)]
async fn root_clear_delegated_session() -> Result<(), Error> {
    AuthApi::clear_delegated_session();
    Ok(())
}

#[canic_query(public)]
async fn root_delegated_session_subject() -> Result<Option<candid::Principal>, Error> {
    Ok(AuthApi::delegated_session_subject())
}

canic::finish!();
