#![allow(clippy::unused_async)]

use canic::{__internal::core::perf, Error, api::canister::placement::ScalingApi, prelude::*};
use canic_reference_support::canister::SCALE_HUB;

const POOL_NAME: &str = "scales";

canic::start!(SCALE_HUB);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(requires(env::build_local_only()))]
async fn audit_plan_create_worker_probe() -> Result<(bool, u64), Error> {
    let value = ScalingApi::plan_create_worker(POOL_NAME)?;
    Ok((value, perf::perf_counter()))
}

canic::cdk::export_candid_debug!();
