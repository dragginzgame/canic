//! Minimal Project Ledger canister for grouped-hierarchy tests.

#![expect(clippy::unused_async)]

use candid::Principal;
use canic::{Error, prelude::*};
use ic_cdk::api::canister_self;

canic::start!();

async fn canic_setup() {}

async fn canic_install(_args: Option<Vec<u8>>) {}

async fn canic_upgrade() {}

/// Return this Canister's ID so tests can prove that the installed Ledger is live.
#[canic_query(public)]
async fn canister_id() -> Result<Principal, Error> {
    Ok(canister_self())
}

canic::finish!();
