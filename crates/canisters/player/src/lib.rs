#![allow(clippy::unused_async)]

use icu::{canister::PLAYER, prelude::*};

//
// ICU
//

icu_start!(PLAYER);

const fn icu_setup() {}
async fn icu_install(_: Option<Vec<u8>>) {}
async fn icu_upgrade() {}

// Minimal player endpoint for visibility
#[query]
const fn player_name() -> &'static str {
    "icu:player"
}

export_candid!();
