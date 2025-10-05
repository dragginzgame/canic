//!
//! Scaling hub demo canister showcasing pool orchestration endpoints.
//! Exists under `crates/canisters` strictly for test and example flows.
//!

#![allow(clippy::unused_async)]

use canic::{Error, canister::SCALE_HUB, ops, prelude::*};

//
// ICU
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
async fn create_worker(pool: String) -> Result<Principal, Error> {
    let worker_pid = ops::scaling::create_worker(&pool).await?;

    Ok(worker_pid)
}

/// Dry-run the worker creation decision using config-driven policy.
#[query]
async fn plan_create_worker(pool: String) -> Result<bool, Error> {
    // Example: return whether scaling policy says "yes, spawn"
    let plan = ops::scaling::plan_create_worker(&pool)?;

    Ok(plan.should_spawn)
}

export_candid!();
