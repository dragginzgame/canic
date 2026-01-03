//!
//! Blank demo canister used in tests to exercise provisioning flows.
//! Lives in `crates/canisters` solely as a showcase for ops helpers.
//!

#![allow(clippy::unused_async)]

use canic::{
    PublicError,
    core::{
        api::rpc::create_canister_request,
        dto::rpc::{CreateCanisterParent, CreateCanisterResponse},
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
async fn create_blank() -> Result<CreateCanisterResponse, PublicError> {
    create_canister_request::<()>(&BLANK, CreateCanisterParent::ThisCanister, None::<()>).await
}

export_candid!();
