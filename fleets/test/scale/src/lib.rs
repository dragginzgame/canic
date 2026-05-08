//!
//! Scaling worker demo canister used to exercise the ops scaling helpers.
//! Part of the `fleets` showcase suite.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic::{Error, api::rpc::RpcApi, ids::CanisterRole, prelude::*};

const SCALE: CanisterRole = CanisterRole::new("scale");

/// Run no-op setup for the scaling worker shell.
pub async fn canic_setup() {}

/// Accept no install payload for the scaling worker shell.
pub async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the scaling worker shell.
pub async fn canic_upgrade() {}

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
}

canic::cdk::export_candid_debug!();
