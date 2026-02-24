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
    api::{auth::DelegationApi, env::EnvQuery, ic::Call},
    dto::{
        auth::{DelegatedToken, DelegatedTokenClaims, DelegationRequest},
        error::ErrorCode,
    },
    prelude::*,
    protocol,
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

/// user_shard_mint_token
/// Mint a delegated token using the locally stored delegation proof.
///
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_update]
async fn user_shard_mint_token(claims: DelegatedTokenClaims) -> Result<DelegatedToken, Error> {
    // Test-only guard: keep this endpoint out of production flows.
    if !cfg!(debug_assertions) {
        return Err(Error::forbidden("test-only canister"));
    }

    let proof = match DelegationApi::require_proof() {
        Ok(proof) => proof,
        Err(err) if err.code == ErrorCode::NotFound => {
            request_delegation(&claims).await?;
            DelegationApi::require_proof()?
        }
        Err(err) => return Err(err),
    };
    DelegationApi::sign_token(claims, proof).await
}

#[canic_query(requires(authenticated("auth:verify")))]
async fn hello(token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

export_candid!();

async fn request_delegation(claims: &DelegatedTokenClaims) -> Result<(), Error> {
    let root_pid = EnvQuery::snapshot()
        .root_pid
        .ok_or_else(|| Error::internal("root pid unavailable"))?;

    let ttl_secs = claims.exp.saturating_sub(claims.iat);
    if ttl_secs == 0 {
        return Err(Error::invalid(
            "delegation ttl_secs must be greater than zero",
        ));
    }

    let request = DelegationRequest {
        shard_pid: canic::cdk::api::canister_self(),
        scopes: claims.scopes.clone(),
        aud: claims.aud.clone(),
        ttl_secs,
        verifier_targets: Vec::new(),
        include_root_verifier: true,
    };

    let response: Result<Result<canic::dto::auth::DelegationProvisionResponse, Error>, Error> =
        Call::unbounded_wait(root_pid, protocol::CANIC_REQUEST_DELEGATION)
            .with_arg(request)?
            .execute()
            .await?
            .candid()?;

    response.map(|_| ())
}
