//!
//! User hub canister that initiates user shard provisioning for delegated signing.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target. Its endpoints may intentionally omit
//! production-grade auth because they are exercised only in controlled tests.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::{canister::placement::ShardingApi, env::EnvQuery, ic::Call},
    cdk::types::Principal,
    dto::auth::{DelegationProvisionRequest, DelegationProvisionResponse},
    prelude::*,
    protocol,
};
use canic_internal::canister::USER_HUB;

const POOL_NAME: &str = "user_shards";

//
// CANIC
//

canic::start!(USER_HUB);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// create_account
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_update]
async fn create_account(pid: Principal) -> Result<Principal, Error> {
    // Test-only guard: keep this endpoint out of production flows.
    if !cfg!(debug_assertions) {
        return Err(Error::forbidden("test-only canister"));
    }

    ShardingApi::assign_to_pool(POOL_NAME, pid.to_string()).await
}

/// provision_user_shard
/// Request root provisioning for signer proofs on user shards.
///
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_update]
async fn provision_user_shard(
    request: DelegationProvisionRequest,
) -> Result<DelegationProvisionResponse, Error> {
    // Test-only guard: keep this endpoint out of production flows.
    if !cfg!(debug_assertions) {
        return Err(Error::forbidden("test-only canister"));
    }

    let root_pid = EnvQuery::snapshot()
        .root_pid
        .ok_or_else(|| Error::internal("root pid unavailable"))?;

    let response: Result<DelegationProvisionResponse, Error> =
        Call::unbounded_wait(root_pid, protocol::CANIC_DELEGATION_PROVISION)
            .with_arg(request)?
            .execute()
            .await?
            .candid()?;

    response
}

export_candid!();
