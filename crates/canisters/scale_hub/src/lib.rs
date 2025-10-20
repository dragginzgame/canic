//!
//! Scaling hub demo canister showcasing pool orchestration endpoints.
//! Exists under `crates/canisters` strictly for test and example flows.
//!

#![allow(clippy::unused_async)]

use candid::Principal;
use canic::{Error, ops::ext::scaling, prelude::*};
use canic_internal::canister::SCALE_HUB;

//
// CANIC
//

canic_start!(SCALE_HUB);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// Create a new worker in the given pool.
#[update]
async fn create_worker() -> Result<Principal, Error> {
    let worker_pid = scaling::create_worker("scales").await?;

    Ok(worker_pid)
}

/// Dry-run the worker creation decision using config-driven policy.
#[query]
async fn plan_create_worker() -> Result<bool, Error> {
    // Example: return whether scaling policy says "yes, spawn"
    let plan = scaling::plan_create_worker("scales")?;

    Ok(plan.should_spawn)
}

export_candid!();
