//! Minimal non-root canister for delegation proof tests.

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::auth::DelegationApi,
    dto::auth::{DelegatedToken, DelegatedTokenClaims},
    ids::cap,
    prelude::*,
};
use canic_internal::canister::USER_SHARD;

canic::start!(USER_SHARD);

async fn canic_setup() {}
async fn canic_install(_args: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_update]
async fn signer_mint_token(claims: DelegatedTokenClaims) -> Result<DelegatedToken, Error> {
    let proof = DelegationApi::require_proof()?;
    DelegationApi::sign_token(claims, proof).await
}

#[canic_update(requires(auth::is_authenticated(cap::VERIFY)))]
async fn signer_verify_token(_token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(auth::is_authenticated()))]
async fn signer_verify_token_any(_token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

export_candid!();
