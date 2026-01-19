//!
//! Blank demo canister used in tests to exercise provisioning flows.
//! Lives in `crates/canisters` solely as a showcase for ops helpers.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::{access::DelegatedTokenApi, auth::DelegationApi, env::EnvQuery},
    dto::auth::{DelegatedToken, DelegationProof},
    prelude::*,
};
use canic_internal::canister::TEST;
use std::time::Duration;

//
// CANIC
//

canic::start!(TEST);

async fn canic_setup() {}

async fn canic_install(_: Option<Vec<u8>>) {
    // Schedule perf-instrumented timers to ensure timer macros are covered.
    timer!(Duration::from_secs(5), timer_once);
    timer_interval!(Duration::from_secs(10), timer_interval);
}

async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// main test endpoint for things that can fail
#[canic_update]
async fn test() -> Result<(), Error> {
    Ok(())
}

/// test_set_delegation_proof
/// Root-only helper to install a delegation proof for auth tests.
#[canic_update(auth(caller_is_root))]
async fn test_set_delegation_proof(proof: DelegationProof) -> Result<(), Error> {
    let root_pid = EnvQuery::view()
        .root_pid
        .ok_or_else(|| Error::internal("root pid unavailable"))?;

    DelegatedTokenApi::verify_delegation_proof(&proof, root_pid)?;
    DelegationApi::store_proof(proof)
}

/// test_verify_delegation_structure
/// Root-only helper to validate delegation structure without signatures.
#[canic_update(auth(caller_is_root))]
async fn test_verify_delegation_structure(proof: DelegationProof) -> Result<(), Error> {
    DelegatedTokenApi::verify_delegation_structure(&proof, None)
}

/// test_verify_delegation_signature
/// Root-only helper to validate delegation signatures.
#[canic_update(auth(caller_is_root))]
async fn test_verify_delegation_signature(proof: DelegationProof) -> Result<(), Error> {
    let root_pid = EnvQuery::view()
        .root_pid
        .ok_or_else(|| Error::internal("root pid unavailable"))?;

    DelegatedTokenApi::verify_delegation_signature(&proof, root_pid)
}

/// test_verify_delegated_token
/// Verifies delegated tokens using the access guard.
#[canic_update(auth(delegated_token_valid))]
async fn test_verify_delegated_token(_token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}

//
// timers
//
async fn timer_once() {
    let _ = 1 + 1;
}

async fn timer_interval() {
    let _ = 1 + 1;
}

export_candid!();
