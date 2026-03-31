//!
//! Blank demo canister used in tests to exercise provisioning flows.
//! Lives in `canisters` solely as a showcase for ops helpers.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic::{Error, dto::auth::DelegatedToken, ids::cap, prelude::*};
use canic_internal::canister::TEST;
use std::time::Duration;

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
#[canic_update]
async fn test() -> Result<(), Error> {
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    Ok(())
}

/// test_verify_delegated_token
/// Verifies delegated tokens using the access guard.
#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn test_verify_delegated_token(_token: DelegatedToken) -> Result<(), Error> {
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

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

canic::cdk::export_candid_debug!();
