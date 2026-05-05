//!
//! Manual local sandbox canister for temporary endpoint experiments.
//!
//! This canister is intentionally excluded from the reference release set and
//! test fixtures. Keep throwaway experiments here instead of changing
//! `canisters/test/runtime_probe`.
//!

#![allow(clippy::unused_async)]

use canic::{Error, ids::CanisterRole, prelude::*};

const SANDBOX_MINIMAL: CanisterRole = CanisterRole::new("sandbox_minimal");

/// Run no-op setup for the local sandbox shell.
pub async fn canic_setup() {}

/// Accept no install payload for the local sandbox shell.
pub async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the local sandbox shell.
pub async fn canic_upgrade() {}

//
// CANIC
//

canic::start_local!(SANDBOX_MINIMAL);

/// Return a small liveness response for quick manual calls.
#[canic_query]
fn sandbox_minimal_ping() -> Result<String, Error> {
    Ok("sandbox_minimal:ok".to_string())
}

/// Echo one string under an explicit sandbox payload limit.
#[canic_update(payload(max_bytes = 32))]
fn sandbox_minimal_echo(payload: String) -> Result<String, Error> {
    Ok(payload)
}

canic::cdk::export_candid_debug!();
