#![allow(clippy::unused_async)]

use canic::{Error, cdk::candid::Reserved, ids::cap, prelude::*};
use canic_internal::canister::TEST;
use std::time::Duration;

canic::start!(TEST);

async fn canic_setup() {}

async fn canic_install(_: Option<Vec<u8>>) {
    // Schedule perf-instrumented timers to ensure timer macros are covered.
    timer!(Duration::from_secs(5), timer_once);
    timer_interval!(Duration::from_secs(10), timer_interval);
}

async fn canic_upgrade() {}

#[canic_update]
async fn test() -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn test_verify_delegated_token(_token: Reserved) -> Result<(), Error> {
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    Ok(())
}

async fn timer_once() {
    let _ = 1 + 1;
}

async fn timer_interval() {
    let _ = 1 + 1;
}

canic::cdk::export_candid_debug!();
