#![expect(clippy::unused_async)]

use candid::{CandidType, Deserialize};
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
use ic_cdk::api::time;

canic::start!();

/// Run no-op setup for the audit leaf probe.
async fn canic_setup() {}

/// Accept no install payload for the audit leaf probe.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the audit leaf probe.
async fn canic_upgrade() {}

#[canic_query(requires(env::build_local_only()))]
async fn audit_time_probe() -> Result<QueryPerfSample<u64>, Error> {
    Ok(MetricsQuery::sample_query(time()))
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

// This fixed endpoint is a B1 attribution fixture. Its five slots mirror the
// largest current `Page<T>` fanout in one generated Canic status surface. The
// build script changes aliases only; method, branch, and wire shapes stay fixed.
macro_rules! generic_cohort_nominal {
    ($name:ident) => {
        #[derive(CandidType, Deserialize)]
        struct $name {
            value: u64,
        }

        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self { value }
            }
        }
    };
}

generic_cohort_nominal!(GenericCohortNominal1);
#[cfg(canic_generic_cohort_ge_2)]
generic_cohort_nominal!(GenericCohortNominal2);
#[cfg(canic_generic_cohort_ge_3)]
generic_cohort_nominal!(GenericCohortNominal3);
#[cfg(canic_generic_cohort_ge_4)]
generic_cohort_nominal!(GenericCohortNominal4);
#[cfg(canic_generic_cohort_ge_5)]
generic_cohort_nominal!(GenericCohortNominal5);

type GenericCohortSlot1 = GenericCohortNominal1;

#[cfg(canic_generic_cohort_ge_2)]
type GenericCohortSlot2 = GenericCohortNominal2;
#[cfg(not(canic_generic_cohort_ge_2))]
type GenericCohortSlot2 = GenericCohortNominal1;

#[cfg(canic_generic_cohort_ge_3)]
type GenericCohortSlot3 = GenericCohortNominal3;
#[cfg(not(canic_generic_cohort_ge_3))]
type GenericCohortSlot3 = GenericCohortNominal1;

#[cfg(canic_generic_cohort_ge_4)]
type GenericCohortSlot4 = GenericCohortNominal4;
#[cfg(not(canic_generic_cohort_ge_4))]
type GenericCohortSlot4 = GenericCohortNominal1;

#[cfg(canic_generic_cohort_ge_5)]
type GenericCohortSlot5 = GenericCohortNominal5;
#[cfg(not(canic_generic_cohort_ge_5))]
type GenericCohortSlot5 = GenericCohortNominal1;

#[derive(CandidType, Deserialize)]
enum GenericPageCohortResponse {
    Slot1(Page<GenericCohortSlot1>),
    Slot2(Page<GenericCohortSlot2>),
    Slot3(Page<GenericCohortSlot3>),
    Slot4(Page<GenericCohortSlot4>),
    Slot5(Page<GenericCohortSlot5>),
}

macro_rules! generic_cohort_page {
    ($function:ident, $element:ty) => {
        fn $function(value: u64) -> Page<$element> {
            Page {
                entries: vec![<$element as From<u64>>::from(value)],
                total: 1,
            }
        }
    };
}

generic_cohort_page!(generic_cohort_page_1, GenericCohortSlot1);
generic_cohort_page!(generic_cohort_page_2, GenericCohortSlot2);
generic_cohort_page!(generic_cohort_page_3, GenericCohortSlot3);
generic_cohort_page!(generic_cohort_page_4, GenericCohortSlot4);
generic_cohort_page!(generic_cohort_page_5, GenericCohortSlot5);

#[canic_query(public)]
fn audit_page_generic_cohort(slot: u8, value: u64) -> Result<GenericPageCohortResponse, Error> {
    let response = match slot {
        2 => GenericPageCohortResponse::Slot2(generic_cohort_page_2(value)),
        3 => GenericPageCohortResponse::Slot3(generic_cohort_page_3(value)),
        4 => GenericPageCohortResponse::Slot4(generic_cohort_page_4(value)),
        5 => GenericPageCohortResponse::Slot5(generic_cohort_page_5(value)),
        _ => GenericPageCohortResponse::Slot1(generic_cohort_page_1(value)),
    };
    Ok(response)
}

canic::finish!();
