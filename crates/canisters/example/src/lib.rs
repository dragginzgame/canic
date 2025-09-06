#![allow(clippy::unused_async)]

use icu::{
    Error,
    canister::EXAMPLE,
    ops::{request::create_canister_request, response::CreateCanisterResponse},
    prelude::*,
};

//
// ICU
//

icu_start!(EXAMPLE);

const fn icu_setup() {}
async fn icu_install(_: Option<Vec<u8>>) {}
async fn icu_upgrade() {}

// create_example
#[update]
async fn create_example() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&EXAMPLE, None).await
}

export_candid!();
