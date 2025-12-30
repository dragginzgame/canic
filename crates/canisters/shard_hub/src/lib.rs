//!
//! Shard hub demo canister coordinating shard assignments for testing.
//! Ships in `crates/canisters` solely to showcase sharding functionality.
//!

#![allow(clippy::unused_async)]

use candid::Principal;
use canic::{
    Error,
    core::{
        policy::placement::sharding::policy::ShardingPolicy,
        workflow::placement::sharding::assign::ShardingWorkflow,
    },
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

// don't need authentication as this is a local canic test
#[canic_update]
async fn register_principal(pid: Principal) -> Result<Principal, Error> {
    let shard_pid = ShardingWorkflow::assign_to_pool(POOL_NAME, pid.to_string()).await?;

    Ok(shard_pid)
}

/// Dry-run the player registration decision using config-driven policy.
#[canic_query]
async fn plan_register_principal(pid: Principal) -> Result<String, Error> {
    let plan = ShardingPolicy::plan_assign_to_pool(POOL_NAME, pid.to_string())?;

    Ok(format!("{:?}", plan.state))
}

export_candid!();
