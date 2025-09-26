#![allow(clippy::unused_async)]

use icu::{
    Error,
    canister::BLANK,
    ops::{
        request::{CreateCanisterParent, create_canister_request},
        response::CreateCanisterResponse,
    },
    prelude::*,
};

//
// ICU
//

icu_start!(BLANK);

async fn icu_setup() {}
async fn icu_install(_: Option<Vec<u8>>) {}
async fn icu_upgrade() {}

// create_blank
#[update]
async fn create_blank() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&BLANK, CreateCanisterParent::Caller, None).await
}

export_candid!();
