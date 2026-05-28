#![expect(clippy::unused_async)]

use canic::{Error, api::canister::placement::ScalingApi, cdk::types::Principal, prelude::*};

const POOL_NAME: &str = "scales";

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

/// Create a new worker in the configured pool.
#[canic_update]
async fn create_worker() -> Result<Principal, Error> {
    canic::access::require_local()?;
    let worker_pid = ScalingApi::create_worker(POOL_NAME).await?;

    Ok(worker_pid)
}

/// Dry-run the worker creation decision using config-driven policy.
#[canic_query]
async fn plan_create_worker() -> Result<bool, Error> {
    canic::access::require_local()?;
    ScalingApi::plan_create_worker(POOL_NAME)
}

canic::finish!();
