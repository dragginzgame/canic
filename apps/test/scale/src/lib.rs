#![expect(clippy::unused_async)]

mod protocol;

use canic::{Error, api::rpc::RpcApi, dto::rpc::CyclesResponse, prelude::*};

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

canic::start!();

/// Ask the configured parent for a direct cycles top-up.
#[canic_update(public, wire(id = protocol::REQUEST_CYCLES, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn request_cycles_from_parent(cycles: u128) -> Result<CyclesResponse, Error> {
    RpcApi::request_cycles(cycles).await
}

canic::finish!();
