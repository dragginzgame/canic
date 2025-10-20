//!
//! Shard hub demo canister coordinating shard assignments for testing.
//! Ships in `crates/canisters` solely to showcase sharding functionality.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    ops::ext::sharding::{ShardingOps, ShardingPlan, ShardingPolicyOps},
    prelude::*,
};
use canic_internal::canister::SHARD_HUB;

const SHARD_POOL: &str = "shards";

//
// CANIC
//

canic_start!(SHARD_HUB);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

#[update]
async fn register_principal(pid: Principal) -> Result<Principal, Error> {
    let shard_pid = ShardingOps::assign_to_pool(SHARD_POOL, pid).await?;

    Ok(shard_pid)
}

/// Dry-run the player registration decision using config-driven policy.
#[query]
async fn plan_register_principal(pid: Principal) -> Result<ShardingPlan, Error> {
    let plan = ShardingPolicyOps::plan_assign_to_pool(SHARD_POOL, pid)?;

    Ok(plan)
}

export_candid!();
