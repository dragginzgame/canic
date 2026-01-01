use crate::{
    Error,
    cdk::utils::time::now_secs,
    ids::CanisterRole,
    ops::prelude::*,
    storage::memory::pool::{
        CanisterPool, CanisterPoolData, CanisterPoolState, CanisterPoolStatus,
    },
};

///
/// PoolSnapshot
/// Internal snapshot of the canister pool registry.
///

pub struct PoolSnapshot {
    pub entries: Vec<PoolEntrySnapshot>,
}

impl From<CanisterPoolData> for PoolSnapshot {
    fn from(data: CanisterPoolData) -> Self {
        Self {
            entries: data
                .entries
                .into_iter()
                .map(|(pid, entry)| PoolEntrySnapshot {
                    pid,
                    created_at: entry.header.created_at,
                    cycles: entry.state.cycles,
                    status: entry.state.status,
                    role: entry.state.role,
                    parent: entry.state.parent,
                    module_hash: entry.state.module_hash,
                })
                .collect(),
        }
    }
}

///
/// PoolEntrySnapshot
/// Internal, operational snapshot of a single pool entry.
///

pub struct PoolEntrySnapshot {
    pub pid: Principal,
    pub created_at: u64,
    pub cycles: Cycles,
    pub status: CanisterPoolStatus,
    pub role: Option<CanisterRole>,
    pub parent: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
}

///
/// PoolOps
/// Operational wrapper for canister pool storage.
///

pub struct PoolOps;

impl PoolOps {
    // -----------------------------------------------------------------
    // Registration
    // -----------------------------------------------------------------

    pub fn register_ready(
        pid: Principal,
        cycles: Cycles,
        role: Option<CanisterRole>,
        parent: Option<Principal>,
        module_hash: Option<Vec<u8>>,
    ) {
        let created_at = now_secs();
        CanisterPool::register(
            pid,
            cycles,
            CanisterPoolStatus::Ready,
            role,
            parent,
            module_hash,
            created_at,
        );
    }

    // -----------------------------------------------------------------
    // State transitions
    // -----------------------------------------------------------------

    pub fn mark_pending_reset(pid: Principal) {
        Self::register_or_update_state(
            pid,
            Cycles::default(),
            CanisterPoolStatus::PendingReset,
            None,
        );
    }

    pub fn mark_ready(pid: Principal, cycles: Cycles) {
        Self::register_or_update_state(pid, cycles, CanisterPoolStatus::Ready, None);
    }

    pub fn mark_failed(pid: Principal, err: &Error) {
        let status = CanisterPoolStatus::Failed {
            reason: err.to_string(),
        };
        Self::register_or_update_state(pid, Cycles::default(), status, None);
    }

    // -----------------------------------------------------------------
    // Snapshot (read-only)
    // -----------------------------------------------------------------

    #[must_use]
    pub fn snapshot() -> PoolSnapshot {
        let data = CanisterPool::export();
        data.into()
    }

    #[must_use]
    pub fn len() -> u64 {
        CanisterPool::len()
    }

    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        CanisterPool::contains(pid)
    }

    /// Pop the oldest READY canister from the pool.
    /// FIFO by `created_at`.
    #[must_use]
    pub(crate) fn pop_ready() -> Option<PoolEntrySnapshot> {
        CanisterPool::pop_ready().map(|(pid, entry)| PoolEntrySnapshot {
            pid,
            created_at: entry.header.created_at,
            cycles: entry.state.cycles,
            status: entry.state.status,
            role: entry.state.role,
            parent: entry.state.parent,
            module_hash: entry.state.module_hash,
        })
    }

    // -----------------------------------------------------------------
    // Removal
    // -----------------------------------------------------------------

    pub fn remove(pid: &Principal) {
        let _ = CanisterPool::remove(pid);
    }

    // -----------------------------------------------------------------
    // Internal helper
    // -----------------------------------------------------------------

    fn register_or_update_state(
        pid: Principal,
        cycles: Cycles,
        status: CanisterPoolStatus,
        role: Option<CanisterRole>,
    ) {
        let updated = CanisterPool::update_state_with(pid, |mut state: CanisterPoolState| {
            state.cycles = cycles.clone();
            state.status = status.clone();

            if role.is_some() {
                state.role.clone_from(&role);
            }

            state
        });

        if !updated {
            let created_at = now_secs();
            CanisterPool::register(pid, cycles, status, role, None, None, created_at);
        }
    }
}
