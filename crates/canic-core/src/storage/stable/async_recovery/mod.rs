//! Module: storage::stable::async_recovery
//!
//! Responsibility: persist bounded serial-attempt fences for recovery-critical async timers.
//! Does not own: timer scheduling, lease policy, operation-id derivation, or takeover decisions.
//! Boundary: storage retains one fixed record; ops validates and commits exact transitions.

use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static,
    role_contract::allocation::memory::async_recovery::ASYNC_TIMER_RECOVERY_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

/// Maximum encoded bytes admitted for the complete async timer recovery record.
pub const MAX_ASYNC_TIMER_RECOVERY_RECORD_BYTES: u32 = 2_048;

eager_static! {
    static ASYNC_TIMER_RECOVERY: RefCell<
        Cell<AsyncTimerRecoveryRecord, VirtualMemory<DefaultMemoryImpl>>,
    > = RefCell::new(Cell::init(
        crate::ic_memory_key!(
            authority = CANIC_CORE_MEMORY_AUTHORITY,
            key = "canic.core.async_timer_recovery.v1",
            ty = AsyncTimerRecoveryRecord,
            id = ASYNC_TIMER_RECOVERY_ID,
        ),
        AsyncTimerRecoveryRecord::default(),
    ));
}

/// Exact durable ownership of one currently executing async attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AsyncRecoveryLeaseRecord {
    pub attempt_generation: u64,
    pub operation_generation: u64,
    pub lease_expires_at_ns: u64,
}

/// Scheduling request that arrived while one recovery attempt was active.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AsyncRecoveryPendingScheduleRecord {
    Ensure(u64),
    Reconcile(Option<u64>),
}

/// Durable generations and optional active/pending operation for one owner.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AsyncRecoveryOwnerRecord {
    pub last_attempt_generation: u64,
    pub last_operation_generation: u64,
    pub active: Option<AsyncRecoveryLeaseRecord>,
    pub pending_operation_generation: Option<u64>,
    pub pending_schedule: Option<AsyncRecoveryPendingScheduleRecord>,
    pub recovery_due_at_ns: Option<u64>,
    pub recovery_owned: bool,
    pub retry_streak: u64,
    pub terminal_failure: bool,
}

/// Complete fixed-shape recovery state for all recovery-critical async timer owners.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AsyncTimerRecoveryRecord {
    pub auth_renewal: AsyncRecoveryOwnerRecord,
    pub canister_pool_maintenance: AsyncRecoveryOwnerRecord,
    pub cycle_topup: AsyncRecoveryOwnerRecord,
    pub placement_receipt_acknowledgement: AsyncRecoveryOwnerRecord,
}

impl AsyncTimerRecoveryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "AsyncTimerRecoveryRecord";
}

impl_storable_bounded!(
    AsyncTimerRecoveryRecord,
    MAX_ASYNC_TIMER_RECOVERY_RECORD_BYTES,
    false
);

/// Canonical test/audit snapshot of async timer recovery state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AsyncTimerRecoveryData {
    pub record: AsyncTimerRecoveryRecord,
}

impl AsyncTimerRecoveryData {
    pub const STATE_CONTRACT_NAME: &'static str = "AsyncTimerRecoveryData";
}

/// Single-record stable owner for async timer recovery fences.
pub struct AsyncTimerRecoveryStore;

impl AsyncTimerRecoveryStore {
    #[must_use]
    pub(crate) fn get() -> AsyncTimerRecoveryRecord {
        ASYNC_TIMER_RECOVERY.with_borrow(|cell| cell.get().clone())
    }

    pub(crate) fn replace(record: AsyncTimerRecoveryRecord) {
        ASYNC_TIMER_RECOVERY.with_borrow_mut(|cell| cell.set(record));
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn export() -> AsyncTimerRecoveryData {
        AsyncTimerRecoveryData {
            record: Self::get(),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: AsyncTimerRecoveryData) {
        Self::replace(data.record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::structures::storable::Storable;

    #[test]
    fn async_timer_recovery_roundtrips_with_a_bounded_encoding() {
        let owner = AsyncRecoveryOwnerRecord {
            last_attempt_generation: u64::MAX,
            last_operation_generation: u64::MAX,
            active: Some(AsyncRecoveryLeaseRecord {
                attempt_generation: u64::MAX,
                operation_generation: u64::MAX,
                lease_expires_at_ns: u64::MAX,
            }),
            pending_operation_generation: Some(u64::MAX),
            pending_schedule: Some(AsyncRecoveryPendingScheduleRecord::Reconcile(Some(
                u64::MAX,
            ))),
            recovery_due_at_ns: Some(u64::MAX),
            recovery_owned: true,
            retry_streak: u64::MAX,
            terminal_failure: true,
        };
        let record = AsyncTimerRecoveryRecord {
            auth_renewal: owner.clone(),
            canister_pool_maintenance: owner.clone(),
            cycle_topup: owner.clone(),
            placement_receipt_acknowledgement: owner,
        };

        let bytes = record.to_bytes();
        assert!(bytes.len() <= MAX_ASYNC_TIMER_RECOVERY_RECORD_BYTES as usize);
        assert_eq!(AsyncTimerRecoveryRecord::from_bytes(bytes), record);
    }

    #[test]
    fn async_timer_recovery_export_import_replaces_the_complete_record() {
        let record = AsyncTimerRecoveryRecord {
            auth_renewal: AsyncRecoveryOwnerRecord {
                last_attempt_generation: 3,
                last_operation_generation: 2,
                active: None,
                pending_operation_generation: Some(2),
                pending_schedule: None,
                recovery_due_at_ns: None,
                recovery_owned: false,
                retry_streak: 1,
                terminal_failure: false,
            },
            ..AsyncTimerRecoveryRecord::default()
        };
        AsyncTimerRecoveryStore::import(AsyncTimerRecoveryData {
            record: record.clone(),
        });

        assert_eq!(
            AsyncTimerRecoveryStore::export(),
            AsyncTimerRecoveryData { record }
        );
    }
}
