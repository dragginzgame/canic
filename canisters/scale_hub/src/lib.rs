//!
//! Scaling hub demo canister showcasing pool orchestration endpoints.
//! Exists under `canisters` strictly for test and example flows.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic::{
    __internal::core::perf, Error, api::canister::placement::ScalingApi, cdk::types::Principal,
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
async fn create_worker() -> Result<Principal, Error> {
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    let worker_pid = ScalingApi::create_worker(POOL_NAME).await?;

    Ok(worker_pid)
}

/// Dry-run the worker creation decision using config-driven policy.
/// no authentication needed as for canic testing
#[canic_query]
async fn plan_create_worker() -> Result<bool, Error> {
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    ScalingApi::plan_create_worker(POOL_NAME)
}

// Measure the scaling dry-run query in the same call context as the returned
// local instruction counter.
#[canic_query(requires(env::build_local_only()))]
async fn plan_create_worker_perf_test() -> Result<(bool, u64), Error> {
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    let value = ScalingApi::plan_create_worker(POOL_NAME)?;
    Ok((value, perf::perf_counter()))
}

canic::cdk::export_candid_debug!();
