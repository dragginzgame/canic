//! Module: ops::storage::cycles
//!
//! Responsibility: mutate and project cycle tracker/top-up event records.
//! Does not own: funding policy, runtime metrics, or endpoint authorization.
//! Boundary: storage ops convert stable records into DTO response shapes.

use crate::{
    domain::cycles::CycleTopupEventStatus,
    dto::{
        cycles::{CycleTopupEvent, CycleTrackerEntry},
        page::Page,
    },
    model::cycles_funding::FundingLedgerSnapshot,
    ops::prelude::*,
    storage::stable::cycles::{
        CycleTopupEventEntryRecord, CycleTopupEventStatusRecord, CycleTopupEvents, CycleTracker,
        CyclesFundingLedger, CyclesFundingLedgerRecord,
    },
};

const TOPUP_ERROR_MAX_CHARS: usize = 256;

impl From<CycleTopupEventStatus> for CycleTopupEventStatusRecord {
    fn from(status: CycleTopupEventStatus) -> Self {
        match status {
            CycleTopupEventStatus::RequestErr => Self::RequestErr,
            CycleTopupEventStatus::RequestOk => Self::RequestOk,
            CycleTopupEventStatus::RequestScheduled => Self::RequestScheduled,
        }
    }
}

impl From<CycleTopupEventStatusRecord> for CycleTopupEventStatus {
    fn from(status: CycleTopupEventStatusRecord) -> Self {
        match status {
            CycleTopupEventStatusRecord::RequestErr => Self::RequestErr,
            CycleTopupEventStatusRecord::RequestOk => Self::RequestOk,
            CycleTopupEventStatusRecord::RequestScheduled => Self::RequestScheduled,
        }
    }
}

///
/// CycleTrackerOps
///
/// Stable storage wrapper for the cycle tracker.
/// Owned by storage ops and consumed by runtime cycle workflows.
///

pub struct CycleTrackerOps;

impl CycleTrackerOps {
    pub fn record(now: u64, cycles: Cycles) {
        CycleTracker::record(now, cycles);
    }

    #[must_use]
    pub fn purge_before(cutoff: u64) -> usize {
        CycleTracker::purge_before(cutoff)
    }

    #[must_use]
    pub fn entries() -> Vec<(u64, Cycles)> {
        CycleTracker::entries(0, usize::MAX)
    }

    #[must_use]
    pub fn page_to_response(page: Page<(u64, Cycles)>) -> Page<CycleTrackerEntry> {
        Page {
            entries: page
                .entries
                .into_iter()
                .map(|(timestamp_secs, cycles)| CycleTrackerEntry {
                    timestamp_secs,
                    cycles,
                })
                .collect(),
            total: page.total,
        }
    }
}

///
/// CyclesFundingLedgerStoreOps
///
/// Stable storage wrapper for child cycles funding budget state.
/// Owned by storage ops and consumed by runtime funding ops.
///

pub struct CyclesFundingLedgerStoreOps;

impl CyclesFundingLedgerStoreOps {
    #[must_use]
    pub fn snapshot(child: Principal) -> Option<FundingLedgerSnapshot> {
        CyclesFundingLedger::snapshot(child).map(|record| FundingLedgerSnapshot {
            granted_total: record.granted_total.to_u128(),
            last_granted_at: record.last_granted_at,
        })
    }

    pub fn record_child_grant(child: Principal, granted_cycles: u128, now_secs: u64) {
        CyclesFundingLedger::record_child_grant(child, Cycles::new(granted_cycles), now_secs);
    }

    pub fn restore_child_snapshot(child: Principal, snapshot: FundingLedgerSnapshot) {
        CyclesFundingLedger::set_snapshot(
            child,
            CyclesFundingLedgerRecord {
                granted_total: Cycles::new(snapshot.granted_total),
                last_granted_at: snapshot.last_granted_at,
            },
        );
    }

    #[cfg(test)]
    pub fn reset_for_tests() {
        CyclesFundingLedger::clear_for_tests();
    }
}

///
/// CycleTopupEventOps
///
/// Stable storage wrapper for cycle top-up event history.
/// Owned by storage ops and consumed by cycle funding workflows.
///

pub struct CycleTopupEventOps;

impl CycleTopupEventOps {
    pub fn record_scheduled(now: u64, requested_cycles: Cycles) {
        Self::record_event(
            now,
            requested_cycles,
            None,
            CycleTopupEventStatus::RequestScheduled,
            None,
        );
    }

    pub fn record_ok(now: u64, requested_cycles: Cycles, transferred_cycles: Cycles) {
        Self::record_event(
            now,
            requested_cycles,
            Some(transferred_cycles),
            CycleTopupEventStatus::RequestOk,
            None,
        );
    }

    pub fn record_err(now: u64, requested_cycles: Cycles, error: String) {
        Self::record_event(
            now,
            requested_cycles,
            None,
            CycleTopupEventStatus::RequestErr,
            Some(truncate_topup_error(error)),
        );
    }

    fn record_event(
        now: u64,
        requested_cycles: Cycles,
        transferred_cycles: Option<Cycles>,
        status: CycleTopupEventStatus,
        error: Option<String>,
    ) {
        CycleTopupEvents::record(
            now,
            requested_cycles,
            transferred_cycles,
            status.into(),
            error,
        );
    }

    #[must_use]
    pub fn purge_before(cutoff: u64) -> usize {
        CycleTopupEvents::purge_before(cutoff)
    }

    #[must_use]
    pub fn entries() -> Vec<CycleTopupEventEntryRecord> {
        CycleTopupEvents::data(0, usize::MAX).entries
    }

    #[must_use]
    pub fn page_to_response(page: Page<CycleTopupEventEntryRecord>) -> Page<CycleTopupEvent> {
        Page {
            entries: page
                .entries
                .into_iter()
                .map(|entry| CycleTopupEvent {
                    timestamp_secs: entry.key.timestamp_secs,
                    sequence: entry.key.sequence,
                    requested_cycles: entry.record.requested_cycles,
                    transferred_cycles: entry.record.transferred_cycles,
                    status: entry.record.status.into(),
                    error: entry.record.error,
                })
                .collect(),
            total: page.total,
        }
    }
}

fn truncate_topup_error(error: String) -> String {
    error.chars().take(TOPUP_ERROR_MAX_CHARS).collect()
}
