#![allow(clippy::unused_async)]

use icu::{
    Error,
    canister::PLAYER_HUB,
    ops::shard::{ShardPlan, assign_in_pool, plan_pool},
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
async fn register_player(item: Principal) -> Result<(Principal, Principal), Error> {
    let game = assign_in_pool(&PLAYER_HUB, "game", item).await?;
    let instance = assign_in_pool(&PLAYER_HUB, "instance", item).await?;

    Ok((game, instance))
}

#[query]
/// Dry-run the player registration decision using config-driven policy.
async fn plan_register_player(item: Principal) -> Result<(ShardPlan, ShardPlan), Error> {
    let a = plan_pool(&PLAYER_HUB, "game", item)?;
    let b = plan_pool(&PLAYER_HUB, "instance", item)?;

    Ok((a, b))
}

export_candid!();
