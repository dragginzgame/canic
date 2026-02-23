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
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationProof},
    prelude::*,
};
use canic_internal::canister::USER_SHARD;
use std::{cell::RefCell, collections::BTreeMap};

const TOKEN_VERSION: u16 = 1;

thread_local! {
    static PENDING_TOKEN_ISSUANCE: RefCell<BTreeMap<Principal, PendingTokenIssuance>> =
        RefCell::new(BTreeMap::new());
}

#[derive(Clone)]
struct PendingTokenIssuance {
    claims: DelegatedTokenClaims,
    proof: DelegationProof,
}

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

    let proof = DelegationApi::require_proof()?;
    DelegationApi::prepare_token_signature(TOKEN_VERSION, &claims, &proof)?;

    let caller = msg_caller();
    PENDING_TOKEN_ISSUANCE.with_borrow_mut(|pending| {
        pending.insert(caller, PendingTokenIssuance { claims, proof });
    });

    Ok(())
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

    let caller = msg_caller();
    let pending = PENDING_TOKEN_ISSUANCE.with_borrow(|all| all.get(&caller).cloned());
    let pending = pending.ok_or_else(|| Error::not_found("pending token issuance not found"))?;

    DelegationApi::get_token_signature(TOKEN_VERSION, pending.claims, pending.proof)
}

#[canic_query(requires(authenticated()))]
async fn hello(token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

export_candid!();
