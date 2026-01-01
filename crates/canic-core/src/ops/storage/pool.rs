use crate::{
    Error,
    cdk::utils::time::now_secs,
    ids::CanisterRole,
    ops::prelude::*,
    storage::memory::pool::{PoolData, PoolRecord, PoolRecordState, PoolStatus, PoolStore},
};

///
/// PoolSnapshot
/// Internal, operational snapshot of the pool.
///

pub struct PoolSnapshot {
    pub entries: Vec<PoolEntrySnapshot>,
}

impl From<PoolData> for PoolSnapshot {
    fn from(data: PoolData) -> Self {
        Self {
            entries: data.entries.into_iter().map(Into::into).collect(),
        }
    }
}

///
/// PoolEntrySnapshot
/// Identity-carrying snapshot of a single pool entry.
///

pub struct PoolEntrySnapshot {
    pub pid: Principal,
    pub created_at: u64,
    pub cycles: Cycles,
    pub status: PoolStatus,
    pub role: Option<CanisterRole>,
    pub parent: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
}

impl From<(Principal, PoolRecord)> for PoolEntrySnapshot {
    fn from((pid, record): (Principal, PoolRecord)) -> Self {
        Self {
            pid,
            created_at: record.header.created_at,
            cycles: record.state.cycles,
            status: record.state.status,
            role: record.state.role,
            parent: record.state.parent,
            module_hash: record.state.module_hash,
        }
    }
}

///
/// PoolOps
/// Operational façade over pool storage.
///

pub struct PoolOps;

impl PoolOps {
    // ---------------------------------------------------------------
    // Registration
    // ---------------------------------------------------------------

    pub fn register_ready(
        pid: Principal,
        cycles: Cycles,
        role: Option<CanisterRole>,
        parent: Option<Principal>,
        module_hash: Option<Vec<u8>>,
    ) {
        PoolStore::register(
            pid,
            cycles,
            PoolStatus::Ready,
            role,
            parent,
            module_hash,
            now_secs(),
        );
    }

    // ---------------------------------------------------------------
    // State transitions
    // ---------------------------------------------------------------

    pub fn mark_pending_reset(pid: Principal) {
        Self::register_or_update_state(pid, Cycles::default(), PoolStatus::PendingReset, None);
    }

    pub fn mark_ready(pid: Principal, cycles: Cycles) {
        Self::register_or_update_state(pid, cycles, PoolStatus::Ready, None);
    }

    pub fn mark_failed(pid: Principal, err: &Error) {
        Self::register_or_update_state(
            pid,
            Cycles::default(),
            PoolStatus::Failed {
                reason: err.to_string(),
            },
            None,
        );
    }

    // ---------------------------------------------------------------
    // Snapshot (read-only)
    // ---------------------------------------------------------------

    #[must_use]
    pub fn snapshot() -> PoolSnapshot {
        PoolStore::export().into()
    }

    #[must_use]
    pub fn len() -> u64 {
        PoolStore::len()
    }

    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        PoolStore::contains(pid)
    }

    pub fn has_pending_reset() -> bool {
        PoolStore::has_status(PoolStatus::PendingReset)
    }

    /// Pop the oldest READY entry (FIFO by `created_at`).
    #[must_use]
    pub(crate) fn pop_ready() -> Option<PoolEntrySnapshot> {
        PoolStore::pop_ready().map(Into::into)
    }

    /// Pop the oldest PENDING_RESET entry (FIFO by `created_at`).
    #[must_use]
    pub(crate) fn pop_pending_reset() -> Option<PoolEntrySnapshot> {
        PoolStore::pop_pending_reset().map(Into::into)
    }

    // ---------------------------------------------------------------
    // Removal
    // ---------------------------------------------------------------

    pub fn remove(pid: &Principal) {
        PoolStore::remove(pid);
    }

    // ---------------------------------------------------------------
    // Internal helper
    // ---------------------------------------------------------------

    fn register_or_update_state(
        pid: Principal,
        cycles: Cycles,
        status: PoolStatus,
        role: Option<CanisterRole>,
    ) {
        let updated = PoolStore::update_state_with(pid, |mut state: PoolRecordState| {
            state.cycles = cycles.clone();
            state.status = status.clone();

            if role.is_some() {
                state.role.clone_from(&role);
            }

            state
        });

        if !updated {
            PoolStore::register(pid, cycles, status, role, None, None, now_secs());
        }
    }
}
