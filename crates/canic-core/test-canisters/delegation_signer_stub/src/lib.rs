//! Minimal non-root canister for delegation proof tests.

#![allow(clippy::unused_async)]

use canic::prelude::*;
use canic_internal::canister::USER_SHARD;

canic::start!(USER_SHARD);

async fn canic_setup() {}
async fn canic_install(_args: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

export_candid!();
