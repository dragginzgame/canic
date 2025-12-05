//!
//! Shard hub demo canister coordinating shard assignments for testing.
//! Ships in `crates/canisters` solely to showcase sharding functionality.
//!

#![allow(clippy::unused_async)]

use candid::Principal;
use canic::{
    Error,
    auth::is_controller,
    ops::model::memory::sharding::{ShardingOps, ShardingPlan, ShardingPolicyOps},
    prelude::*,
};
use canic_internal::canister::SHARD_HUB;

const POOL_NAME: &str = "shards";

//
// CANIC
//

canic::start!(SHARD_HUB);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

#[update]
async fn register_principal(pid: Principal) -> Result<Principal, Error> {
    auth_require_all!(is_controller)?;

    let shard_pid = ShardingOps::assign_to_pool(POOL_NAME, pid).await?;

    Ok(shard_pid)
}

/// Dry-run the player registration decision using config-driven policy.
#[query]
async fn plan_register_principal(pid: Principal) -> Result<ShardingPlan, Error> {
    auth_require_all!(is_controller)?;

    let plan = ShardingPolicyOps::plan_assign_to_pool(POOL_NAME, pid)?;

    Ok(plan)
}

export_candid!();
