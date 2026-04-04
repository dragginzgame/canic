//!
//! App demo canister used for local/dev Canic testing.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

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
use canic_internal::{
    canister::APP,
    reference::empty_shell::{canic_install, canic_setup, canic_upgrade},
};

//
// CANIC
//

canic::start!(APP);

//
// TEST PERF PROBES
//

// Measure the shared `canic_time` query in the same call context as the
// returned local instruction counter.
#[canic_query(requires(env::build_local_only()))]
async fn canic_time_perf_test() -> Result<(u64, u64), Error> {
    let value = canic::cdk::api::time();
    Ok((value, perf::perf_counter()))
}

// Measure the shared env snapshot query in the same call context as the
// returned local instruction counter.
#[canic_query(requires(env::build_local_only()))]
async fn canic_env_perf_test() -> Result<(EnvSnapshotResponse, u64), Error> {
    let value = EnvQuery::snapshot();
    Ok((value, perf::perf_counter()))
}

// Measure the shared log pagination query in the same call context as the
// returned local instruction counter.
#[canic_query(requires(env::build_local_only()))]
async fn canic_log_perf_test(
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
) -> Result<(Page<LogEntry>, u64), Error> {
    let value = LogQuery::page(crate_name, topic, min_level, page);
    Ok((value, perf::perf_counter()))
}

canic::cdk::export_candid_debug!();
