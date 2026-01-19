//!
//! Blank demo canister used in tests to exercise provisioning flows.
//! Lives in `crates/canisters` solely as a showcase for ops helpers.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::rpc::RpcApi,
    dto::rpc::{CreateCanisterParent, CreateCanisterResponse},
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
    RpcApi::create_canister_request::<()>(&BLANK, CreateCanisterParent::ThisCanister, None::<()>)
        .await
}

export_candid!();
