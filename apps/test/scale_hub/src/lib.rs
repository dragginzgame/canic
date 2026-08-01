#![expect(clippy::unused_async)]

use candid::Principal;
use canic::{Error, api::canister::placement::ScalingApi, prelude::*};

const POOL_NAME: &str = "scales";

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

/// Create a new worker in the configured pool.
#[canic_update(requires(env::build_local_only()))]
async fn create_worker() -> Result<Principal, Error> {
    let worker_pid = ScalingApi::create_worker(POOL_NAME).await?;

    Ok(worker_pid)
}

/// Dry-run the worker creation decision using config-driven policy.
#[canic_query(requires(env::build_local_only()))]
async fn plan_create_worker() -> Result<bool, Error> {
    ScalingApi::plan_create_worker(POOL_NAME)
}

canic::finish!();
