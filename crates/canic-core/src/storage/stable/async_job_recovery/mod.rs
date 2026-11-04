//! Module: storage::stable::async_job_recovery
//!
//! Responsibility: persist bounded serial-attempt fences for recovery-critical async jobs.
//! Does not own: timer scheduling, provider state or application progress.
//! Boundary: storage retains one fixed record; ops validates and commits exact attempt fences.

use crate::{
    cdk::structures::{DefaultMemoryImpl, StableCell as Cell},
    storage::prelude::*,
};
use std::cell::RefCell;

/// Exact maximum encoded bytes for the complete async-job recovery record.
pub const MAX_ASYNC_JOB_RECOVERY_RECORD_BYTES: u32 = 285;

std::thread_local! {
    static ASYNC_JOB_RECOVERY: RefCell<
        Cell<AsyncJobRecoveryRecord, DefaultMemoryImpl>,
    > = RefCell::new({
        let allocation = crate::memory::open(crate::memory::keys::ASYNC_JOB_RECOVERY_V1);
        if ic_storage::Memory::size(&allocation) == 0 {
            Cell::create(allocation, AsyncJobRecoveryRecord::default()).expect("create admitted stable cell")
        } else {
            Cell::load(allocation, canic_contracts::serialization::STORAGE_LIMITS.bytes).expect("retained stable cell")
        }
    });
}

/// Exact durable ownership of one currently executing async attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AsyncAttemptLeaseRecord {
    pub attempt_generation: u64,
    pub lease_expires_at_ns: u64,
}

/// Minimal durable serial-attempt fence for a domain-owned async job.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AsyncAttemptFenceRecord {
    pub last_attempt_generation: u64,
    pub active: Option<AsyncAttemptLeaseRecord>,
}

/// Exact durable ownership of one replay-safe cycle-funding attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReplaySafeAsyncAttemptLeaseRecord {
    pub attempt_generation: u64,
    pub operation_generation: u64,
    pub lease_expires_at_ns: u64,
}

/// Serial-attempt fence plus the exact cycle-funding generation retained for retry.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReplaySafeAsyncAttemptFenceRecord {
    pub last_attempt_generation: u64,
    pub last_operation_generation: u64,
    pub active: Option<ReplaySafeAsyncAttemptLeaseRecord>,
    pub pending_operation_generation: Option<u64>,
}

/// Durable fixture retry pressure; application progress remains in the importer.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureImportRetryRecord {
    pub failures: u32,
    pub not_before_ns: u64,
}

/// Retry custody, keyed to exact installed authority and last committed progress.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureImportRecoveryRecord {
    pub assignment_hash: [u8; 32],
    pub next_chunk: Option<u32>,
    pub unfinished_attempts: u8,
}

/// Complete fixed-shape recovery state for recovery-critical domain jobs.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AsyncJobRecoveryRecord {
    pub auth_renewal: AsyncAttemptFenceRecord,
    pub portable_enrollment: AsyncAttemptFenceRecord,
    pub canister_pool_maintenance: AsyncAttemptFenceRecord,
    pub cycle_topup: ReplaySafeAsyncAttemptFenceRecord,
    pub fixture_import: AsyncAttemptFenceRecord,
    pub fixture_import_retry: FixtureImportRetryRecord,
    pub fixture_import_recovery: Option<FixtureImportRecoveryRecord>,
    pub fixture_import_failure: Option<crate::domain::fixture_import::FixtureImportFailure>,
    pub placement_receipt_acknowledgement: AsyncAttemptFenceRecord,
}

impl AsyncJobRecoveryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "AsyncJobRecoveryRecord";
}

impl_storable_bounded!(
    AsyncJobRecoveryRecord,
    MAX_ASYNC_JOB_RECOVERY_RECORD_BYTES,
    false
);

/// Canonical test/audit snapshot of async-job recovery state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AsyncJobRecoveryData {
    pub record: AsyncJobRecoveryRecord,
}

impl AsyncJobRecoveryData {
    pub const STATE_CONTRACT_NAME: &'static str = "AsyncJobRecoveryData";
}

/// Single-record stable owner for async-job attempt fences.
pub struct AsyncJobRecoveryStore;

impl AsyncJobRecoveryStore {
    #[must_use]
    pub(crate) fn get() -> AsyncJobRecoveryRecord {
        ASYNC_JOB_RECOVERY.with_borrow(|cell| cell.get().clone())
    }

    pub(crate) fn replace(record: AsyncJobRecoveryRecord) {
        ASYNC_JOB_RECOVERY
            .with_borrow_mut(|cell| cell.set(record).expect("Storage operation failed"));
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn export() -> AsyncJobRecoveryData {
        AsyncJobRecoveryData {
            record: Self::get(),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: AsyncJobRecoveryData) {
        Self::replace(data.record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::structures::storable::Storable;

    #[test]
    fn async_job_recovery_roundtrips_at_its_exact_worst_case_bound() {
        let attempt = AsyncAttemptFenceRecord {
            last_attempt_generation: u64::MAX,
            active: Some(AsyncAttemptLeaseRecord {
                attempt_generation: u64::MAX,
                lease_expires_at_ns: u64::MAX,
            }),
        };
        let record = AsyncJobRecoveryRecord {
            auth_renewal: attempt.clone(),
            portable_enrollment: attempt.clone(),
            canister_pool_maintenance: attempt.clone(),
            cycle_topup: ReplaySafeAsyncAttemptFenceRecord {
                last_attempt_generation: u64::MAX,
                last_operation_generation: u64::MAX,
                active: Some(ReplaySafeAsyncAttemptLeaseRecord {
                    attempt_generation: u64::MAX,
                    operation_generation: u64::MAX,
                    lease_expires_at_ns: u64::MAX,
                }),
                pending_operation_generation: Some(u64::MAX),
            },
            fixture_import: attempt.clone(),
            fixture_import_retry: FixtureImportRetryRecord {
                failures: u32::MAX,
                not_before_ns: u64::MAX,
            },
            fixture_import_failure: Some(
                crate::domain::fixture_import::FixtureImportFailure::Interrupted {
                    next_chunk: Some(u32::MAX),
                    attempts: u8::MAX,
                },
            ),
            fixture_import_recovery: Some(FixtureImportRecoveryRecord {
                assignment_hash: [u8::MAX; 32],
                next_chunk: Some(u32::MAX),
                unfinished_attempts: u8::MAX,
            }),
            placement_receipt_acknowledgement: attempt,
        };

        let bytes = record.to_bytes().expect("encode stable fixture");
        assert_eq!(
            bytes.len(),
            MAX_ASYNC_JOB_RECOVERY_RECORD_BYTES as usize,
            "the stable bound must equal the measured worst-case encoding"
        );
        assert_eq!(
            AsyncJobRecoveryRecord::from_bytes(bytes).expect("decode stable fixture"),
            record
        );
    }

    #[test]
    fn async_job_recovery_export_import_replaces_the_complete_record() {
        let record = AsyncJobRecoveryRecord {
            auth_renewal: AsyncAttemptFenceRecord {
                last_attempt_generation: 3,
                active: None,
            },
            cycle_topup: ReplaySafeAsyncAttemptFenceRecord {
                last_attempt_generation: 4,
                last_operation_generation: 2,
                active: None,
                pending_operation_generation: Some(2),
            },
            ..AsyncJobRecoveryRecord::default()
        };
        AsyncJobRecoveryStore::import(AsyncJobRecoveryData {
            record: record.clone(),
        });

        assert_eq!(
            AsyncJobRecoveryStore::export(),
            AsyncJobRecoveryData { record }
        );
    }
}
