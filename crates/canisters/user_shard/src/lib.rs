//!
//! User shard canister that stores delegation proofs and mints delegated tokens.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target. Its endpoints may intentionally omit
//! production-grade auth because they are exercised only in controlled tests.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::auth::DelegationApi,
    dto::auth::{DelegatedToken, DelegatedTokenClaims},
    prelude::*,
};
use canic_internal::canister::USER_SHARD;

const TOKEN_VERSION: u16 = 1;

//
// CANIC
//

canic::start!(USER_SHARD);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// user_shard_issue_token_prepare
/// Prepare delegated token issuance using the locally stored delegation proof.
///
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_update]
async fn user_shard_issue_token_prepare(claims: DelegatedTokenClaims) -> Result<(), Error> {
    // Test-only guard: keep this endpoint out of production flows.
    if !cfg!(debug_assertions) {
        return Err(Error::forbidden("test-only canister"));
    }

    DelegationApi::issue_token_prepare(TOKEN_VERSION, claims)
}

/// user_shard_issue_token_get
/// Retrieve the delegated token prepared by `user_shard_issue_token_prepare`.
///
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_query]
fn user_shard_issue_token_get() -> Result<DelegatedToken, Error> {
    if !cfg!(debug_assertions) {
        return Err(Error::forbidden("test-only canister"));
    }

    DelegationApi::issue_token_get()
}

#[canic_query(requires(authenticated()))]
async fn hello(token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

export_candid!();
