#![allow(clippy::unused_async)]

use icu::{
    Error,
    canister::{PLAYER, PLAYER_HUB},
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

// Register a new player: assign to an existing player canister or create one on demand
#[update]
async fn register_player(item: Principal) -> Result<Principal, Error> {
    icu::ops::partition::assign_with_config(&PLAYER, &PLAYER_HUB, item).await
}

#[query]
/// Dry-run the player registration decision using config-driven policy.
async fn plan_register_player(
    item: Principal,
) -> Result<icu::ops::partition::PartitionPlan, Error> {
    icu::ops::partition::plan_with_config(&PLAYER_HUB, item)
}

export_candid!();
