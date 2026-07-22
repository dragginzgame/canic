#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::canister::registry::SubnetRegistryApi,
    api::metrics::MetricsQuery,
    dto::{metrics::QueryPerfSample, topology::SubnetRegistryResponse},
    prelude::*,
};

canic::start!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[canic_query(requires(env::build_local_only()))]
async fn audit_subnet_registry_probe() -> Result<QueryPerfSample<SubnetRegistryResponse>, Error> {
    Ok(MetricsQuery::sample_query(SubnetRegistryApi::registry()))
}

canic::finish!();
