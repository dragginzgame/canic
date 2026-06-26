//! Minimal non-root canister for delegation proof tests.

#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::auth::AuthApi,
    cdk::candid::Principal,
    dto::auth::{DelegatedToken, SignedRoleAttestation},
    ids::cap,
    prelude::*,
};

canic::start!();

/// Run no-op setup for the delegation issuer stub.
async fn canic_setup() {}

/// Accept no install payload for the delegation issuer stub.
async fn canic_install(_args: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the delegation issuer stub.
async fn canic_upgrade() {}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn issuer_verify_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

#[canic_update(requires(auth::authenticated()))]
async fn issuer_verify_token_any(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

#[canic_update(public)]
async fn issuer_clear_delegated_session() -> Result<(), Error> {
    AuthApi::clear_delegated_session();
    Ok(())
}

#[canic_query(public)]
async fn issuer_delegated_session_subject() -> Result<Option<Principal>, Error> {
    Ok(AuthApi::delegated_session_subject())
}

#[canic_update(public)]
async fn issuer_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    AuthApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

#[canic_update(requires(caller::is_root()))]
async fn issuer_guard_is_root() -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(caller::is_controller()))]
async fn issuer_guard_is_controller() -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(caller::is_parent()))]
async fn issuer_guard_is_parent() -> Result<(), Error> {
    Ok(())
}

#[canic_update(internal, requires(caller::is_registered_to_subnet()))]
async fn issuer_guard_is_registered_to_subnet() -> Result<(), Error> {
    Ok(())
}

canic::finish!();
