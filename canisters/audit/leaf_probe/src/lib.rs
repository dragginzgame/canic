#![expect(clippy::unused_async)]

mod ops;
mod protocol;

#[cfg(test)]
mod tests;

use canic::{
    __internal::core::api::log::LogQuery,
    Error,
    api::env::EnvQuery,
    api::metrics::MetricsQuery,
    dto::{log::LogEntry, metrics::QueryPerfSample, page::Page},
    prelude::*,
};
use ic_platform::cdk::api::time;
use serde::{Deserialize, Serialize};

canic::start!();

/// Run no-op setup for the audit leaf probe.
async fn canic_setup() {}

/// Accept no install payload for the audit leaf probe.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the audit leaf probe.
async fn canic_upgrade() {}

const FRONTEND_PROBE_ID: u64 = 0x4155_4454_0000_0001;
const FRONTEND_UPDATE_PROBE_ID: u64 = 0x4155_4454_0000_0002;
const FRONTEND_PROBE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 256,
    allocation: 2048,
    nodes: 16,
    depth: 8,
};

/// Prove browser-owned authenticated ingress through the current Fleet admission policy.
#[canic_query(requires(caller::is_fleet_admitted()),
    wire(id = crate::FRONTEND_PROBE_ID, request = crate::FRONTEND_PROBE_LIMITS, response = crate::FRONTEND_PROBE_LIMITS))]
async fn audit_frontend_admission_probe() -> Result<ic_platform::Principal, Error> {
    Ok(ic_platform::cdk::api::msg_caller())
}

/// Prove authenticated update transport under the same Fleet admission policy.
#[canic_update(requires(caller::is_fleet_admitted()),
    wire(id = crate::FRONTEND_UPDATE_PROBE_ID, request = crate::FRONTEND_PROBE_LIMITS, response = crate::FRONTEND_PROBE_LIMITS))]
async fn audit_frontend_update_probe() -> Result<ic_platform::Principal, Error> {
    Ok(ic_platform::cdk::api::msg_caller())
}

#[canic_query(requires(env::build_local_only()), wire(id = protocol::TIME, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn audit_time_probe() -> Result<QueryPerfSample<u64>, Error> {
    Ok(MetricsQuery::sample_query(time()))
}

#[canic_query(requires(env::build_local_only()), wire(id = protocol::ENV, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn audit_env_probe() -> Result<QueryPerfSample<protocol::Environment>, Error> {
    Ok(MetricsQuery::sample_query(EnvQuery::snapshot().into()))
}

#[canic_query(requires(env::build_local_only()), wire(id = protocol::LOG, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn audit_log_probe(
    request: protocol::LogRequest,
) -> Result<QueryPerfSample<Page<LogEntry>>, Error> {
    Ok(MetricsQuery::sample_query(LogQuery::page(
        request.crate_name,
        request.topic,
        request.min_level,
        request.page,
    )))
}

// This fixed endpoint is a B1 attribution fixture. Its five slots mirror the
// largest current `Page<T>` fanout in one generated Canic status surface. The
// build script changes aliases only; method, branch, and wire shapes stay fixed.
macro_rules! generic_cohort_nominal {
    ($name:ident) => {
        #[derive(Deserialize, Serialize)]
        #[cfg_attr(
            all(feature = "wire-schema", not(target_arch = "wasm32")),
            derive(ic_wire::Wire)
        )]
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

#[derive(Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[canic_query(public, wire(id = protocol::COHORT, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn audit_page_generic_cohort(
    request: protocol::CohortRequest,
) -> Result<GenericPageCohortResponse, Error> {
    Ok(cohort_response(request))
}

fn cohort_response(request: protocol::CohortRequest) -> GenericPageCohortResponse {
    let protocol::CohortRequest { slot, value } = request;
    match slot {
        2 => GenericPageCohortResponse::Slot2(generic_cohort_page_2(value)),
        3 => GenericPageCohortResponse::Slot3(generic_cohort_page_3(value)),
        4 => GenericPageCohortResponse::Slot4(generic_cohort_page_4(value)),
        5 => GenericPageCohortResponse::Slot5(generic_cohort_page_5(value)),
        _ => GenericPageCohortResponse::Slot1(generic_cohort_page_1(value)),
    }
}

/// Spend fixture cycles before reviewing a later release's recovery funding.
#[canic_update(requires(env::build_local_only(), caller::is_controller()), wire(id = protocol::BALANCE, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn audit_recovery_balance(retain: u128) -> Result<u128, Error> {
    let excess = ic_platform::cdk::api::canister_cycle_balance().saturating_sub(retain);
    Ok(ic_platform::cdk::api::cycles_burn(excess))
}

canic::finish!();
