//!
//! Blank demo canister used in tests to exercise provisioning flows.
//! Lives in `crates/canisters` solely as a showcase for ops helpers.
//!

#![allow(clippy::unused_async)]

use std::time::Duration;

use canic::{Error, prelude::*};
use canic_internal::canister::TEST;

//
// CANIC
//

canic::start!(TEST);

async fn canic_setup() {}

async fn canic_install(_: Option<Vec<u8>>) {
    // Schedule perf-instrumented timers to ensure timer macros are covered.
    timer!(Duration::from_secs(5), timer_once);
    timer_interval!(Duration::from_secs(10), timer_interval);
}

async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// main test endpoint for things that can fail
#[update]
async fn test() -> Result<(), Error> {
    Ok(())
}

//
// timers
//
async fn timer_once() {
    let _ = 1 + 1;
}

async fn timer_interval() {
    let _ = 1 + 1;
}

export_candid!();
