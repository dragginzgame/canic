#![allow(clippy::unused_async)]

use icu::{
    Error,
    canister::PLAYER_HUB,
    ops::shard::{ShardPlan, assign_for_self, plan_for_self},
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

#[query]
const fn hub_name() -> &'static str {
    "icu:player_hub"
}

// Register a new player across both domains: game and instance
#[update]
async fn register_player(pid: Principal) -> Result<(Principal, Principal), Error> {
    let game = assign_for_self("game", pid).await?;
    let instance = assign_for_self("instance", pid).await?;

    Ok((game, instance))
}

#[query]
/// Dry-run the player registration decision using config-driven policy.
async fn plan_register_player(pid: Principal) -> Result<(ShardPlan, ShardPlan), Error> {
    let a = plan_for_self("game", pid)?;
    let b = plan_for_self("instance", pid)?;

    Ok((a, b))
}

export_candid!();
