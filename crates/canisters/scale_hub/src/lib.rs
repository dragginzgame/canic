//!
//! Scaling hub demo canister showcasing pool orchestration endpoints.
//! Exists under `crates/canisters` strictly for test and example flows.
//!

#![allow(clippy::unused_async)]

use candid::Principal;
use canic::{
    PublicError,
    core::{
        cdk::utils::time::now_secs, policy::placement::scaling::ScalingPolicy,
        workflow::placement::scaling::ScalingWorkflow,
    },
    prelude::*,
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
/// no authentication needed as for canic testing
#[canic_update]
async fn create_worker() -> Result<Principal, PublicError> {
    let worker_pid = ScalingWorkflow::create_worker(POOL_NAME).await?;

    Ok(worker_pid)
}

/// Dry-run the worker creation decision using config-driven policy.
/// no authentication needed as for canic testing
#[canic_query]
async fn plan_create_worker() -> Result<bool, PublicError> {
    // Example: return whether scaling policy says "yes, spawn"
    let plan = ScalingPolicy::plan_create_worker(POOL_NAME, now_secs())?;

    Ok(plan.should_spawn)
}

export_candid!();
