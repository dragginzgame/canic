#![allow(clippy::unused_async)]

use icu::{canister::SCALE, prelude::*};

//
// ICU
//

icu_start!(SCALE);

async fn icu_setup() {}
async fn icu_install(_: Option<Vec<u8>>) {}
async fn icu_upgrade() {}

export_candid!();
