//!
//! Scaling hub demo canister showcasing pool orchestration endpoints.
//! Exists under `fleets` strictly for test and example flows.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![expect(clippy::unused_async)]

use canic::{
    Error, api::canister::placement::ScalingApi, cdk::types::Principal, ids::CanisterRole,
    prelude::*,
};

const POOL_NAME: &str = "scales";
const SCALE_HUB: CanisterRole = CanisterRole::new("scale_hub");

//
// CANIC
//

canic::start!(SCALE_HUB);

/// Run no-op setup for the scaling hub shell.
async fn canic_setup() {}

/// Accept no install payload for the scaling hub shell.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the scaling hub shell.
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// Create a new worker in the configured pool.
///
/// This endpoint is intentionally local-only for the reference Canic test flow.
#[canic_update]
async fn create_worker() -> Result<Principal, Error> {
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    let worker_pid = ScalingApi::create_worker(POOL_NAME).await?;

    Ok(worker_pid)
}

/// Dry-run the worker creation decision using config-driven policy.
///
/// This endpoint is intentionally local-only for the reference Canic test flow.
#[canic_query]
async fn plan_create_worker() -> Result<bool, Error> {
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    ScalingApi::plan_create_worker(POOL_NAME)
}

canic::finish!();
