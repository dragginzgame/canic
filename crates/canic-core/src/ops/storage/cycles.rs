//! Module: ops::storage::cycles
//!
//! Responsibility: mutate and project cycle tracker/top-up event records.
//! Does not own: funding policy, runtime metrics, or endpoint authorization.
//! Boundary: storage ops convert stable records into DTO response shapes.

use crate::{
    domain::cycles::{CycleTopupEventStatus, CycleTopupFailureDisposition},
    dto::{
        cycles::{CycleTopupEvent, CycleTopupFailure, CycleTrackerEntry},
        page::Page,
    },
    model::cycles_funding::FundingLedgerSnapshot,
    model::replay::OperationId,
    ops::prelude::*,
    storage::stable::cycles::{
        CycleTopupEventEntryRecord, CycleTopupEventStatusRecord, CycleTopupEvents,
        CycleTopupFailureRecord, CycleTracker, CyclesFundingLedger, CyclesFundingLedgerRecord,
    },
};

const TOPUP_ERROR_MAX_BYTES: usize = 160;

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
    pub fn purge_before(cutoff: u64, limit: usize) -> usize {
        CycleTracker::purge_before(cutoff, limit)
    }

    #[must_use]
    pub fn latest() -> Option<(u64, Cycles)> {
        CycleTracker::latest()
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
    /// Persist the exact failed request and the timer owner's actual retry decision.
    pub fn record_parent_failure(
        now: u64,
        requested_cycles: Cycles,
        parent: Principal,
        operation_id: OperationId,
        error: &crate::InternalError,
        disposition: CycleTopupFailureDisposition,
    ) {
        CycleTopupEvents::record(
            now,
            requested_cycles,
            None,
            CycleTopupEventStatusRecord::RequestErr,
            Some(truncate_topup_error(error.to_string())),
            Some(CycleTopupFailureRecord {
                parent,
                operation_id: operation_id.into_bytes(),
                public_error_code: error.public_error().raw_code(),
                disposition,
            }),
        );
    }

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
            None,
        );
    }

    #[must_use]
    pub fn purge_before(cutoff: u64, limit: usize) -> usize {
        CycleTopupEvents::purge_before(cutoff, limit)
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
                    parent_failure: entry
                        .record
                        .parent_failure
                        .map(|failure| CycleTopupFailure {
                            parent: failure.parent,
                            operation_id: failure.operation_id,
                            public_error_code: failure.public_error_code,
                            disposition: failure.disposition,
                        }),
                })
                .collect(),
            total: page.total,
        }
    }
}

fn truncate_topup_error(mut error: String) -> String {
    error.truncate(error.floor_char_boundary(TOPUP_ERROR_MAX_BYTES.min(error.len())));
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        InternalError, cdk::structures::Storable, diagnostics::codes,
        storage::stable::cycles::CycleTopupEventRecord, test::seams,
    };

    #[test]
    fn terminal_parent_failure_survives_retention_and_projects_exact_authority() {
        let _guard = seams::lock();
        CycleTopupEvents::clear_for_tests();
        let parent = Principal::from_slice(&[255; 29]);
        let operation = OperationId::from_bytes([255; 32]);
        let error = InternalError::public(codes::SECURITY_CONFLICT);
        CycleTopupEventOps::record_scheduled(1, u128::MAX.into());
        CycleTopupEventOps::record_parent_failure(
            2,
            u128::MAX.into(),
            parent,
            operation,
            &error,
            CycleTopupFailureDisposition::Terminal,
        );
        let snapshot = CycleTopupEvents::data(0, usize::MAX);
        CycleTopupEvents::import(snapshot);
        assert_eq!(CycleTopupEventOps::purge_before(u64::MAX, 128), 1);
        assert_eq!(CycleTopupEventOps::purge_before(u64::MAX, 128), 0);
        let page = CycleTopupEventOps::page_to_response(Page {
            entries: CycleTopupEventOps::entries(),
            total: 1,
        });
        let event = &page.entries[0];
        assert_eq!(event.timestamp_secs, 2);
        let failure = event.parent_failure.as_ref().unwrap();
        assert_eq!(failure.parent, parent);
        assert_eq!(failure.operation_id, operation.into_bytes());
        assert_eq!(failure.public_error_code, error.public_error().raw_code());
        assert_eq!(failure.disposition, CycleTopupFailureDisposition::Terminal);
        CycleTopupEventOps::record_ok(3, 1.into(), 1.into());
        assert_eq!(CycleTopupEventOps::purge_before(u64::MAX, 128), 2);
    }

    #[test]
    fn maximum_parent_diagnostic_fits_existing_stable_slot_and_utf8_bound() {
        let error = truncate_topup_error("🦀".repeat(256));
        assert_eq!(error.len(), TOPUP_ERROR_MAX_BYTES);
        let record = CycleTopupEventRecord {
            requested_cycles: u128::MAX.into(),
            transferred_cycles: Some(u128::MAX.into()),
            status: CycleTopupEventStatusRecord::RequestErr,
            error: Some(error),
            parent_failure: Some(CycleTopupFailureRecord {
                parent: Principal::from_slice(&[255; 29]),
                operation_id: [255; 32],
                public_error_code: u16::MAX,
                disposition: CycleTopupFailureDisposition::Terminal,
            }),
        };
        let bytes = record.to_bytes();
        assert!(
            bytes.len() <= CycleTopupEventRecord::STORABLE_MAX_SIZE as usize,
            "{} bytes",
            bytes.len()
        );
        assert_eq!(CycleTopupEventRecord::from_bytes(bytes), record);
    }
}
