#![allow(clippy::unused_async)]

use icu::{
    Error,
    canister::SHARD_HUB,
    ops::shard::{ShardPlan, assign_to_pool, plan_assign_to_pool},
    prelude::*,
};

//
// ICU
//

icu_start!(SHARD_HUB);

async fn icu_setup() {}
async fn icu_install(_: Option<Vec<u8>>) {}
async fn icu_upgrade() {}

//
// ENDPOINTS
//

#[update]
async fn register_principal(pid: Principal) -> Result<Principal, Error> {
    let shard_pid = assign_to_pool("shards", pid).await?;

    Ok(shard_pid)
}

/// Dry-run the player registration decision using config-driven policy.
#[query]
async fn plan_register_principal(pid: Principal) -> Result<ShardPlan, Error> {
    let plan = plan_assign_to_pool("shards", pid)?;

    Ok(plan)
}

export_candid!();
