//!
//! User shard canister that stores delegation proofs and issues delegated tokens.
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
    ids::cap,
    prelude::*,
};
use canic_internal::canister::USER_SHARD;

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

/// user_shard_issue_token
/// Issue a delegated token using the locally stored delegation proof.
///
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_update]
async fn user_shard_issue_token(claims: DelegatedTokenClaims) -> Result<DelegatedToken, Error> {
    // Test-only guard: keep this endpoint out of production flows.
    if !cfg!(debug_assertions) {
        return Err(Error::forbidden("test-only canister"));
    }

    DelegationApi::issue_token(claims).await
}

#[canic_query(requires(auth::authenticated(cap::VERIFY)))]
async fn hello(token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

export_candid!();
