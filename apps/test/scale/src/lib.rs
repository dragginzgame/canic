#![expect(clippy::unused_async)]

use canic::{Error, api::rpc::RpcApi, prelude::*};

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

canic::start!();

/// Ask the configured parent for a direct cycles top-up.
#[canic_update(public)]
async fn request_cycles_from_parent(cycles: u128) -> Result<u128, Error> {
    RpcApi::request_cycles(cycles)
        .await
        .map(|response| response.cycles_transferred)
}

canic::finish!();
