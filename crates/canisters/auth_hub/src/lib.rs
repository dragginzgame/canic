//!
//! Auth hub canister that provisions auth shard workers for delegated signing.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target. Its endpoints may intentionally omit
//! production-grade auth because they are exercised only in controlled tests.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::{auth::DelegationApi, canister::placement::ShardingApi, env::EnvQuery, ic::Call},
    cdk::types::Principal,
    dto::auth::DelegationProof,
    prelude::*,
};
use canic_internal::canister::AUTH_HUB;

const POOL_NAME: &str = "auth_shards";
const AUTH_SHARD_SET_PROOF: &str = "auth_shard_set_proof";
//
// CANIC
//

canic::start!(AUTH_HUB);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// create_auth_shard
/// Provision or re-use an auth shard for the provided tenant label.
///
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_update]
async fn create_auth_shard(tenant: Principal) -> Result<Principal, Error> {
    if !cfg!(debug_assertions) {
        return Err(Error::forbidden("test-only canister"));
    }

    ShardingApi::assign_to_pool(POOL_NAME, tenant.to_string()).await
}

/// plan_create_auth_shard
/// Dry-run shard allocation decision for a tenant.
///
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_query]
async fn plan_create_auth_shard(tenant: Principal) -> Result<String, Error> {
    if !cfg!(debug_assertions) {
        return Err(Error::forbidden("test-only canister"));
    }

    let plan = ShardingApi::plan_assign_to_pool(POOL_NAME, tenant.to_string())?;
    Ok(format!("{plan:?}"))
}

/// finalize_auth_shard
/// Install a root-signed delegation proof on the shard.
///
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_update]
async fn finalize_auth_shard(shard_pid: Principal, proof: DelegationProof) -> Result<(), Error> {
    if !cfg!(debug_assertions) {
        return Err(Error::forbidden("test-only canister"));
    }

    if proof.cert.signer_pid != shard_pid {
        return Err(Error::invalid("proof signer does not match shard"));
    }

    let root_pid = EnvQuery::snapshot()
        .root_pid
        .ok_or_else(|| Error::internal("root pid unavailable"))?;

    DelegationApi::verify_delegation_proof(&proof, root_pid)?;
    install_proof(shard_pid, proof).await
}

async fn install_proof(shard_pid: Principal, proof: DelegationProof) -> Result<(), Error> {
    let response: Result<(), Error> = Call::unbounded_wait(shard_pid, AUTH_SHARD_SET_PROOF)
        .with_arg(proof)?
        .execute()
        .await?
        .candid()?;

    response
}

export_candid!();
