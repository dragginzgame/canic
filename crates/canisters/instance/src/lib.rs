#![allow(clippy::unused_async)]

use icu::{canister::INSTANCE, prelude::*};

//
// ICU
//

icu_start!(INSTANCE);

const fn icu_setup() {}
async fn icu_install(_: Option<Vec<u8>>) {}
async fn icu_upgrade() {}

// Minimal endpoint for visibility
#[query]
const fn name() -> &'static str {
    "icu:instance"
}

export_candid!();

