#![allow(clippy::unused_async)]

use icu::{canister::DELEGATION, prelude::*};

//
// ICU
//

icu_start!(DELEGATION);

async fn icu_setup() {}
async fn icu_install(_: Option<Vec<u8>>) {}
async fn icu_upgrade() {}

export_candid!();
