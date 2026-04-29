//! Minimal non-root canister for delegation proof tests.

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::auth::AuthApi,
    cdk::candid::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenMintRequest, SignedRoleAttestation},
    ids::cap,
    prelude::*,
};
use canic_reference_support::canister::USER_SHARD;

canic::start!(USER_SHARD);

async fn canic_setup() {}
async fn canic_install(_args: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_update]
async fn signer_issue_token(request: DelegatedTokenMintRequest) -> Result<DelegatedToken, Error> {
    AuthApi::mint_token(request).await
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

canic::cdk::export_candid_debug!();
