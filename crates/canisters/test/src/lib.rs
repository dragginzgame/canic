#![allow(clippy::unused_async)]

use icu::{Error, interface::request::create_canister_request, prelude::*};

//
// ICU
//

icu_start!("test");

const fn icu_setup() {}

async fn icu_install(_: Option<Vec<u8>>) {}

async fn icu_upgrade() {}

// create_test
#[update]
async fn create_test() -> Result<Principal, Error> {
    create_canister_request::<()>("test", None).await
}

export_candid!();
