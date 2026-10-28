#![expect(clippy::unused_async)]

mod protocol;

use canic::{Error, api::canister::placement::ScalingApi, prelude::*};
use ic_platform::Principal;

const POOL_NAME: &str = "scales";

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

/// Create a new worker in the configured pool.
#[update(requires(env::build_local_only()), wire(id = protocol::CREATE_WORKER, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn create_worker() -> Result<Principal, Error> {
    let worker_pid = ScalingApi::create_worker(POOL_NAME).await?;

    Ok(worker_pid.into())
}

/// Dry-run the worker creation decision using config-driven policy.
#[query(requires(env::build_local_only()), wire(id = protocol::PLAN_WORKER, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn plan_create_worker() -> Result<bool, Error> {
    ScalingApi::plan_create_worker(POOL_NAME)
}

canic::finish!();
