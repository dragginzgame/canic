//!
//! Scaling worker demo canister used to exercise the ops scaling helpers.
//! Part of the `canisters` showcase suite.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic::{Error, api::rpc::RpcApi, prelude::*};
use canic_internal::{
    canister::SCALE,
    reference::empty_shell::{canic_install, canic_setup, canic_upgrade},
};

//
// CANIC
//

canic::start!(SCALE);

/// request_cycles_from_parent
/// Ask the configured parent for a direct cycles top-up.
#[canic_update]
async fn request_cycles_from_parent(cycles: u128) -> Result<u128, Error> {
    RpcApi::request_cycles(cycles)
        .await
        .map(|response| response.cycles_transferred)
        .map_err(Error::from)
}

canic::cdk::export_candid_debug!();
