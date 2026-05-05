//!
//! Manual local playground canister for temporary endpoint experiments.
//!
//! This canister is intentionally excluded from the reference release set and
//! test fixtures. Keep throwaway experiments here instead of changing
//! `crates/canic-core/test-canisters/canister_test`.
//!

#![allow(clippy::unused_async)]

use canic::{Error, prelude::*};
use canic_reference_support::{
    canister::PLAYGROUND,
    reference::empty_shell::{canic_install, canic_setup, canic_upgrade},
};

//
// CANIC
//

canic::start_local!(PLAYGROUND);

/// Return a small liveness response for quick manual calls.
#[canic_query]
fn playground_ping() -> Result<String, Error> {
    Ok("playground:ok".to_string())
}

/// Echo one string under an explicit playground payload limit.
#[canic_update(payload(max_bytes = 32))]
fn playground_echo(payload: String) -> Result<String, Error> {
    Ok(payload)
}

canic::cdk::export_candid_debug!();
