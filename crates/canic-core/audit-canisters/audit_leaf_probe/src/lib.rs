#![allow(clippy::unused_async)]

use canic::{
    __internal::core::{api::log::LogQuery, log::Level, perf},
    Error,
    api::env::EnvQuery,
    dto::{
        env::EnvSnapshotResponse,
        log::LogEntry,
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
async fn audit_time_probe() -> Result<(u64, u64), Error> {
    Ok((canic::cdk::api::time(), perf::perf_counter()))
}

#[canic_query(requires(env::build_local_only()))]
async fn audit_env_probe() -> Result<(EnvSnapshotResponse, u64), Error> {
    Ok((EnvQuery::snapshot(), perf::perf_counter()))
}

#[canic_query(requires(env::build_local_only()))]
async fn audit_log_probe(
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
) -> Result<(Page<LogEntry>, u64), Error> {
    Ok((
        LogQuery::page(crate_name, topic, min_level, page),
        perf::perf_counter(),
    ))
}

canic::cdk::export_candid_debug!();
