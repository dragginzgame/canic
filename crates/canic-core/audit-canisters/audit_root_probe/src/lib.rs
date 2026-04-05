#![allow(clippy::unused_async)]

use canic::{
    __internal::core::{api::state::SubnetStateQuery, perf},
    Error,
    api::canister::registry::SubnetRegistryApi,
    dto::{state::SubnetStateResponse, topology::SubnetRegistryResponse},
    prelude::*,
};

canic::start_root!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[canic_query(requires(env::build_local_only()))]
async fn audit_subnet_registry_probe() -> Result<(SubnetRegistryResponse, u64), Error> {
    Ok((SubnetRegistryApi::registry(), perf::perf_counter()))
}

#[canic_query(requires(env::build_local_only()))]
async fn audit_subnet_state_probe() -> Result<(SubnetStateResponse, u64), Error> {
    Ok((SubnetStateQuery::snapshot(), perf::perf_counter()))
}

canic::cdk::export_candid_debug!();
