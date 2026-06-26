//!
//! Manual local blank sandbox canister for temporary endpoint experiments.
//!
//! This canister is intentionally excluded from the reference release set and
//! test fixtures. Keep throwaway experiments here instead of changing
//! `fleets/test/runtime_probe`.
//!

#![expect(clippy::unused_async)]

use canic::{Error, prelude::*};

/// Run no-op setup for the local blank sandbox shell.
async fn canic_setup() {}

/// Accept no install payload for the local blank sandbox shell.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the local blank sandbox shell.
async fn canic_upgrade() {}

//
// CANIC
//

canic::start_local!();

/// Return a small liveness response for quick manual calls.
#[canic_query(public)]
fn sandbox_blank_ping() -> Result<String, Error> {
    Ok("sandbox_blank:ok".to_string())
}

/// Echo one string under an explicit sandbox payload limit.
#[canic_update(public, payload(max_bytes = 32))]
fn sandbox_blank_echo(payload: String) -> Result<String, Error> {
    Ok(payload)
}

canic::finish!();
