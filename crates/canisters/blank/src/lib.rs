//!
//! Blank demo canister used in tests to exercise provisioning flows.
//! Lives in `crates/canisters` solely as a showcase for ops helpers.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    core::ops::command::{
        request::{CreateCanisterParent, create_canister_request},
        response::CreateCanisterResponse,
    },
    prelude::*,
};
use canic_internal::canister::BLANK;

//
// CANIC
//

canic::start!(BLANK);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

/// create_blank
/// no authentication needed as its for local canic testing
#[canic_update]
async fn create_blank() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&BLANK, CreateCanisterParent::ThisCanister, None).await
}

export_candid!();
