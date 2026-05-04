#![allow(clippy::unused_async)]

use canic::{
    __internal::core::{api::log::LogQuery, log::Level},
    Error,
    api::env::EnvQuery,
    api::metrics::MetricsQuery,
    dto::{
        env::EnvSnapshotResponse,
        log::LogEntry,
        metrics::QueryPerfSample,
        page::{Page, PageRequest},
    },
    prelude::*,
};
use canic_reference_support::canister::APP;

canic::start!(APP);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(requires(env::build_local_only()))]
async fn audit_time_probe() -> Result<QueryPerfSample<u64>, Error> {
    Ok(MetricsQuery::sample_query(canic::cdk::api::time()))
}

#[canic_query(requires(env::build_local_only()))]
async fn audit_env_probe() -> Result<QueryPerfSample<EnvSnapshotResponse>, Error> {
    Ok(MetricsQuery::sample_query(EnvQuery::snapshot()))
}

#[canic_query(requires(env::build_local_only()))]
async fn audit_log_probe(
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
) -> Result<QueryPerfSample<Page<LogEntry>>, Error> {
    Ok(MetricsQuery::sample_query(LogQuery::page(
        crate_name, topic, min_level, page,
    )))
}

canic::cdk::export_candid_debug!();
