#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::{canister::placement::ScalingApi, metrics::MetricsQuery},
    dto::metrics::QueryPerfSample,
    ids::CanisterRole,
    prelude::*,
};

const POOL_NAME: &str = "scales";
const SCALE_HUB: CanisterRole = CanisterRole::new("scale_hub");

canic::start!(SCALE_HUB);

/// Run no-op setup for the audit scaling probe.
async fn canic_setup() {}

/// Accept no install payload for the audit scaling probe.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the audit scaling probe.
async fn canic_upgrade() {}

#[canic_query(requires(env::build_local_only()))]
async fn audit_plan_create_worker_probe() -> Result<QueryPerfSample<bool>, Error> {
    let value = ScalingApi::plan_create_worker(POOL_NAME)?;
    Ok(MetricsQuery::sample_query(value))
}

canic::cdk::export_candid_debug!();
