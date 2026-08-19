//! Module: storage::stable::async_job_recovery
//!
//! Responsibility: persist bounded serial-attempt fences for recovery-critical async jobs.
//! Does not own: timer scheduling, provider state, retry timing, or domain operation records.
//! Boundary: storage retains one fixed record; ops validates and commits exact attempt fences.

use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static,
    role_contract::allocation::memory::async_job_recovery::ASYNC_JOB_RECOVERY_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

/// Exact maximum encoded bytes for the complete async-job recovery record.
pub const MAX_ASYNC_JOB_RECOVERY_RECORD_BYTES: u32 = 589;

eager_static! {
    static ASYNC_JOB_RECOVERY: RefCell<
        Cell<AsyncJobRecoveryRecord, VirtualMemory<DefaultMemoryImpl>>,
    > = RefCell::new(Cell::init(
        crate::ic_memory_key!(
            authority = CANIC_CORE_MEMORY_AUTHORITY,
            key = "canic.core.async_job_recovery.v1",
            ty = AsyncJobRecoveryRecord,
            id = ASYNC_JOB_RECOVERY_ID,
        ),
        AsyncJobRecoveryRecord::default(),
    ));
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

/// Complete fixed-shape recovery state for recovery-critical domain jobs.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AsyncJobRecoveryRecord {
    pub auth_renewal: AsyncAttemptFenceRecord,
    pub canister_pool_maintenance: AsyncAttemptFenceRecord,
    pub cycle_topup: ReplaySafeAsyncAttemptFenceRecord,
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
        ASYNC_JOB_RECOVERY.with_borrow_mut(|cell| cell.set(record));
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
            placement_receipt_acknowledgement: attempt,
        };

        let bytes = record.to_bytes();
        assert_eq!(
            bytes.len(),
            MAX_ASYNC_JOB_RECOVERY_RECORD_BYTES as usize,
            "the stable bound must equal the measured worst-case encoding"
        );
        assert_eq!(AsyncJobRecoveryRecord::from_bytes(bytes), record);
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
