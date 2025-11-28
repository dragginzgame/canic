//!
//! Shard worker demo canister used when exercising sharding ops flows.
//! Included in `crates/canisters` as sample-only code.
//!

#![allow(clippy::unused_async)]

use canic::prelude::*;
use canic_internal::canister::SHARD;

//
// CANIC
//

canic::start!(SHARD);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

export_candid!();
