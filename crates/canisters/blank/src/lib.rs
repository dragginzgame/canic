//!
//! Blank demo canister used in tests to exercise provisioning flows.
//! Lives in `crates/canisters` solely as a showcase for ops helpers.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    ops::request::{CreateCanisterParent, CreateCanisterResponse, create_canister_request},
    prelude::*,
};
use canic_internal::canister::BLANK;

//
// CANIC
//

canic_start!(BLANK);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

// create_blank
#[update]
async fn create_blank() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&BLANK, CreateCanisterParent::Caller, None).await
}

export_candid!();
