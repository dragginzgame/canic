#![expect(clippy::unused_async)]

use canic::{
    __internal::core::api::state::SubnetStateQuery,
    Error,
    api::canister::registry::SubnetRegistryApi,
    api::metrics::MetricsQuery,
    dto::{metrics::QueryPerfSample, state::SubnetStateResponse, topology::SubnetRegistryResponse},
    prelude::*,
};

canic::start_root!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[canic_query(requires(env::build_local_only()))]
async fn audit_subnet_registry_probe() -> Result<QueryPerfSample<SubnetRegistryResponse>, Error> {
    Ok(MetricsQuery::sample_query(SubnetRegistryApi::registry()))
}

#[canic_query(requires(env::build_local_only()))]
async fn audit_subnet_state_probe() -> Result<QueryPerfSample<SubnetStateResponse>, Error> {
    Ok(MetricsQuery::sample_query(SubnetStateQuery::snapshot()))
}

canic::cdk::export_candid_debug!();
