#![allow(clippy::unused_async)]

use icu::{
    Error,
    canister::PLAYER_HUB,
    ops::shard::{ShardPlan, assign_to_pool, plan_assign_to_pool},
    prelude::*,
};

//
// ICU
//

icu_start!(PLAYER_HUB);

const fn icu_setup() {}
async fn icu_install(_: Option<Vec<u8>>) {}
async fn icu_upgrade() {}

//
// ENDPOINTS
//

// Register a new player across both domains: game and instance
#[update]
async fn register_player(pid: Principal) -> Result<(Principal, Principal), Error> {
    let game = assign_to_pool("game", pid).await?;
    let instance = assign_to_pool("instance", pid).await?;

    Ok((game, instance))
}

/// Dry-run the player registration decision using config-driven policy.
#[query]
async fn plan_register_player(pid: Principal) -> Result<(ShardPlan, ShardPlan), Error> {
    let a = plan_assign_to_pool("game", pid)?;
    let b = plan_assign_to_pool("instance", pid)?;

    Ok((a, b))
}

export_candid!();
