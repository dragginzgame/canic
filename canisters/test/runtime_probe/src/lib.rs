#![allow(clippy::unused_async)]

use canic::{
    Error,
    dto::auth::DelegatedToken,
    ids::{CanisterRole, cap},
    prelude::*,
};
use std::time::Duration;

const TEST: CanisterRole = CanisterRole::new("test");

canic::start!(TEST);

/// Run no-op setup for the runtime probe.
async fn canic_setup() {}

/// Schedule timers used by runtime macro coverage tests.
async fn canic_install(_: Option<Vec<u8>>) {
    // Schedule perf-instrumented timers to ensure timer macros are covered.
    timer!(Duration::from_secs(5), timer_once);
    timer_interval!(Duration::from_secs(10), timer_interval);
}

/// Run no-op upgrade handling for the runtime probe.
async fn canic_upgrade() {}

#[canic_update]
async fn test() -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn test_verify_delegated_token(_token: DelegatedToken) -> Result<(), Error> {
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
