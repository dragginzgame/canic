//! Module: cycles::parse
//!
//! Responsibility: decode typed cycle-history pages and project CLI samples.
//! Does not own: ICP command execution, cycle accounting, or report aggregation.
//! Boundary: accepts only the canonical ICP JSON envelope with typed Candid bytes.

use crate::cycles::model::{
    CycleTopupEventPage, CycleTopupEventSample, CycleTopupStatus, CycleTrackerPage,
    CycleTrackerSample,
};
use canic_core::dto::{
    cycles::{CycleTopupEvent, CycleTopupEventStatus, CycleTrackerEntry},
    page::Page,
};
use canic_host::icp::{IcpJsonResponseError, decode_json_result_response};

pub(super) fn parse_cycle_tracker_page(
    output: &str,
) -> Result<CycleTrackerPage, IcpJsonResponseError> {
    let page = decode_json_result_response::<Page<CycleTrackerEntry>>(output)?;
    Ok(CycleTrackerPage {
        entries: page
            .entries
            .into_iter()
            .map(|entry| CycleTrackerSample {
                timestamp_secs: entry.timestamp_secs,
                cycles: entry.cycles.to_u128(),
            })
            .collect(),
        total: page.total,
    })
}

pub(super) fn parse_topup_event_page(
    output: &str,
) -> Result<CycleTopupEventPage, IcpJsonResponseError> {
    let page = decode_json_result_response::<Page<CycleTopupEvent>>(output)?;
    Ok(CycleTopupEventPage {
        entries: page
            .entries
            .into_iter()
            .map(|entry| CycleTopupEventSample {
                timestamp_secs: entry.timestamp_secs,
                transferred_cycles: entry.transferred_cycles.map(|cycles| cycles.to_u128()),
                status: topup_status(entry.status),
            })
            .collect(),
        total: page.total,
    })
}

const fn topup_status(status: CycleTopupEventStatus) -> CycleTopupStatus {
    match status {
        CycleTopupEventStatus::RequestErr => CycleTopupStatus::RequestErr,
        CycleTopupEventStatus::RequestOk => CycleTopupStatus::RequestOk,
        CycleTopupEventStatus::RequestScheduled => CycleTopupStatus::RequestScheduled,
    }
}
