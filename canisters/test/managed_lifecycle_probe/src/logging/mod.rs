//! Installed diagnostic proof for query isolation and native retention idempotency.

use canic::{Error, prelude::*};
use ic_timers::{TimerIdentity, TimerSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct LogProbeEvidence {
    pub rows: u32,
    pub logs_unchanged: bool,
    pub timer_armed: bool,
    pub schedule_unchanged: bool,
}

fn timer() -> Option<TimerSnapshot> {
    let identity = TimerIdentity::try_new("canic", "log_retention", "run").unwrap();
    ic_timers::timer_snapshot(&identity).unwrap()
}

fn same_schedule(before: Option<&TimerSnapshot>, after: Option<&TimerSnapshot>) -> bool {
    let schedule =
        |value: &TimerSnapshot| (value.registration_id(), value.generation(), value.state());
    before.map(schedule) == after.map(schedule)
}

fn page() -> Result<canic::dto::log::LogPage, Error> {
    canic::__internal::core::api::log::LogQuery::page(
        Some(env!("CARGO_PKG_NAME").into()),
        None,
        None,
        canic::dto::log::LogPageRequest {
            before: None,
            limit: 1000,
        },
    )
}

fn observe_query() -> Result<LogProbeEvidence, Error> {
    let before = page()?;
    let timer_before = timer();
    canic::log!(Info, "query diagnostics remain operator output");
    let after = page()?;
    let timer_after = timer();
    Ok(LogProbeEvidence {
        rows: u32::try_from(after.entries.len()).unwrap(),
        logs_unchanged: before.entries == after.entries,
        timer_armed: timer_after
            .as_ref()
            .is_some_and(|snapshot| snapshot.generation().is_some()),
        schedule_unchanged: same_schedule(timer_before.as_ref(), timer_after.as_ref()),
    })
}

fn append_with_same_deadline() -> Result<LogProbeEvidence, Error> {
    canic::log!(Info, "first replicated diagnostic");
    let before = page()?;
    let timer_before = timer();
    canic::log!(Info, "second replicated diagnostic");
    let after = page()?;
    let timer_after = timer();
    Ok(LogProbeEvidence {
        rows: u32::try_from(after.entries.len()).unwrap(),
        logs_unchanged: before.entries == after.entries,
        timer_armed: timer_after
            .as_ref()
            .is_some_and(|snapshot| snapshot.generation().is_some()),
        schedule_unchanged: same_schedule(timer_before.as_ref(), timer_after.as_ref()),
    })
}

#[query(public, wire(id = 0x5445_5354_0000_040a, request = crate::protocol::REQUEST_LIMITS, response = crate::protocol::RESPONSE_LIMITS))]
fn managed_log_query_probe() -> Result<LogProbeEvidence, Error> {
    observe_query()
}

#[update(public, wire(id = 0x5445_5354_0000_040b, request = crate::protocol::REQUEST_LIMITS, response = crate::protocol::RESPONSE_LIMITS))]
fn managed_log_update_probe() -> Result<LogProbeEvidence, Error> {
    append_with_same_deadline()
}
