//!
//! User shard canister that issues self-contained delegated auth tokens.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target. Its endpoints may intentionally omit
//! production-grade auth because they are exercised only in controlled tests.
//!

#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::auth::AuthApi,
    dto::auth::{DelegatedToken, DelegatedTokenMintRequest},
    ids::{CanisterRole, cap},
    prelude::*,
};

const USER_SHARD: CanisterRole = CanisterRole::new("user_shard");

//
// CANIC
//

canic::start!(USER_SHARD);

/// Run no-op setup for the user shard shell.
async fn canic_setup() {}

/// Accept no install payload for the user shard shell.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the user shard shell.
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// user_shard_issue_token
/// Issue a self-contained delegated token without local proof-cache state.
///
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_update]
async fn user_shard_issue_token(
    request: DelegatedTokenMintRequest,
) -> Result<DelegatedToken, Error> {
    // Test-only guard: keep this endpoint out of non-local flows.
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    AuthApi::mint_token(request).await
}

#[cfg(not(canic_disable_bundle_observability_env))]
/// Return the local shard public key for local/dev attestation test flows.
#[canic_update]
async fn user_shard_local_public_key_test() -> Result<Vec<u8>, Error> {
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    AuthApi::local_shard_public_key_sec1().await
}

#[canic_query(requires(auth::authenticated(cap::VERIFY)))]
async fn hello(_token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

canic::cdk::export_candid_debug!();
