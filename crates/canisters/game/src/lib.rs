#![allow(clippy::unused_async)]

use icu::{canister::GAME, prelude::*};

//
// ICU
//

icu_start!(GAME);

const fn icu_setup() {}
async fn icu_install(_: Option<Vec<u8>>) {}
async fn icu_upgrade() {}

// Minimal endpoint for visibility
#[query]
const fn name() -> &'static str {
    "icu:game"
}

export_candid!();
