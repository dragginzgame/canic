//!
//! Blank demo canister used in tests to exercise provisioning flows.
//! Lives in `crates/canisters` solely as a showcase for ops helpers.
//!

#![allow(clippy::unused_async)]

use std::time::Duration;

use canic::{
    Error,
    core::{
        ops::perf::{PerfOps, PerfSnapshot},
        types::PageRequest,
    },
    prelude::*,
};
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

/// Run a small perf-instrumented workload and return the snapshot.
#[update]
async fn test_perf() -> PerfSnapshot {
    // Track total instructions for this call
    perf_defer!();

    // Reset the baseline for intra-call checkpoints
    perf!("baseline");

    let mut acc = 0u64;
    for i in 0..10_000 {
        acc = acc.wrapping_add(i);
    }
    perf!("workload_one");

    for chunk in 0..5 {
        acc = acc.wrapping_add(chunk * 11);
        for _ in 0..500 {
            acc = acc.rotate_left(3).wrapping_add(0xA5A5 ^ acc);
        }
    }
    perf!("workload_two");

    PerfOps::snapshot(PageRequest::DEFAULT)
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
