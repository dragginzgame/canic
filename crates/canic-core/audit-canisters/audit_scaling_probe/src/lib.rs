#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::{canister::placement::ScalingApi, metrics::MetricsQuery},
    dto::metrics::QueryPerfSample,
    prelude::*,
};
use canic_reference_support::canister::SCALE_HUB;

const POOL_NAME: &str = "scales";

canic::start!(SCALE_HUB);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(requires(env::build_local_only()))]
async fn audit_plan_create_worker_probe() -> Result<QueryPerfSample<bool>, Error> {
    let value = ScalingApi::plan_create_worker(POOL_NAME)?;
    Ok(MetricsQuery::sample_query(value))
}

canic::cdk::export_candid_debug!();
