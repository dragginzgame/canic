//!
//! Scaling hub demo canister showcasing pool orchestration endpoints.
//! Exists under `crates/canisters` strictly for test and example flows.
//!

#![allow(clippy::unused_async)]

use canic::{
    PublicError, cdk::types::Principal, core::api::placement::scaling::ScalingApi, prelude::*,
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
    let worker_pid = ScalingApi::create_worker(POOL_NAME).await?;

    Ok(worker_pid)
}

/// Dry-run the worker creation decision using config-driven policy.
/// no authentication needed as for canic testing
#[canic_query]
async fn plan_create_worker() -> Result<bool, PublicError> {
    ScalingApi::plan_create_worker(POOL_NAME)
}

export_candid!();
