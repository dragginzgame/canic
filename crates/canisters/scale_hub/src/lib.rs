//!
//! Scaling hub demo canister showcasing pool orchestration endpoints.
//! Exists under `crates/canisters` strictly for test and example flows.
//!

#![allow(clippy::unused_async)]

use candid::Principal;
use canic::{
    Error, auth::is_controller, ops::model::memory::scaling::ScalingRegistryOps, prelude::*,
};
use canic_internal::canister::SCALE_HUB;

const POOL_NAME: &str = "scales";

//
// CANIC
//

canic::start!(SCALE_HUB);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// Create a new worker in the given pool.
#[update]
async fn create_worker() -> Result<Principal, Error> {
    auth_require_all!(is_controller)?;

    let worker_pid = ScalingRegistryOps::create_worker(POOL_NAME).await?;

    Ok(worker_pid)
}

/// Dry-run the worker creation decision using config-driven policy.
#[query]
async fn plan_create_worker() -> Result<bool, Error> {
    auth_require_all!(is_controller)?;

    // Example: return whether scaling policy says "yes, spawn"
    let plan = ScalingRegistryOps::plan_create_worker(POOL_NAME)?;

    Ok(plan.should_spawn)
}

export_candid!();
