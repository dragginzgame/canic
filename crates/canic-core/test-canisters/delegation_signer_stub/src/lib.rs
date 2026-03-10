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

export_candid!();
