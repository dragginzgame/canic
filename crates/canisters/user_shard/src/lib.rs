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
    // Test-only guard: keep this endpoint out of non-local flows.
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    DelegationApi::issue_token(claims).await
}

#[canic_query(requires(auth::authenticated(cap::VERIFY)))]
async fn hello(token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

canic::cdk::export_candid_debug!();
