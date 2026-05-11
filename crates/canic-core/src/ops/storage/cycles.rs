use crate::{
    dto::{
        cycles::{CycleTopupEvent, CycleTopupEventStatus, CycleTrackerEntry},
        page::Page,
    },
    ops::prelude::*,
    storage::stable::cycles::{
        CycleTopupEventKey, CycleTopupEventRecord, CycleTopupEvents, CycleTracker,
    },
};

const TOPUP_ERROR_MAX_CHARS: usize = 256;

///
/// CycleTrackerOps
/// Stable storage wrapper for the cycle tracker.
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
/// CycleTopupEventOps
/// Stable storage wrapper for cycle top-up event history.
///

pub struct CycleTopupEventOps;

impl CycleTopupEventOps {
    pub fn record_scheduled(now: u64, requested_cycles: Cycles) {
        CycleTopupEvents::record(
            now,
            requested_cycles,
            None,
            CycleTopupEventStatus::RequestScheduled,
            None,
        );
    }

    pub fn record_ok(now: u64, requested_cycles: Cycles, transferred_cycles: Cycles) {
        CycleTopupEvents::record(
            now,
            requested_cycles,
            Some(transferred_cycles),
            CycleTopupEventStatus::RequestOk,
            None,
        );
    }

    pub fn record_err(now: u64, requested_cycles: Cycles, error: String) {
        CycleTopupEvents::record(
            now,
            requested_cycles,
            None,
            CycleTopupEventStatus::RequestErr,
            Some(truncate_topup_error(error)),
        );
    }

    #[must_use]
    pub fn purge_before(cutoff: u64) -> usize {
        CycleTopupEvents::purge_before(cutoff)
    }

    #[must_use]
    pub fn entries() -> Vec<(CycleTopupEventKey, CycleTopupEventRecord)> {
        CycleTopupEvents::entries(0, usize::MAX)
    }

    #[must_use]
    pub fn page_to_response(
        page: Page<(CycleTopupEventKey, CycleTopupEventRecord)>,
    ) -> Page<CycleTopupEvent> {
        Page {
            entries: page
                .entries
                .into_iter()
                .map(|(key, record)| CycleTopupEvent {
                    timestamp_secs: key.timestamp_secs,
                    sequence: key.sequence,
                    requested_cycles: record.requested_cycles,
                    transferred_cycles: record.transferred_cycles,
                    status: record.status,
                    error: record.error,
                })
                .collect(),
            total: page.total,
        }
    }
}

fn truncate_topup_error(error: String) -> String {
    error.chars().take(TOPUP_ERROR_MAX_CHARS).collect()
}
