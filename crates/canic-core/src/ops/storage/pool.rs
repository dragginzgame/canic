use crate::{InternalError, ops::prelude::*, storage::stable::pool::PoolStore};

pub use crate::storage::stable::pool::{PoolData, PoolRecord, PoolStatus};

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
        created_at: u64,
    ) {
        PoolStore::register(
            pid,
            cycles,
            PoolStatus::Ready,
            role,
            parent,
            module_hash,
            created_at,
        );
    }

    // ---------------------------------------------------------------
    // State transitions
    // ---------------------------------------------------------------

    pub fn mark_pending_reset(pid: Principal, created_at: u64) {
        Self::register_or_update_state(
            pid,
            Cycles::default(),
            PoolStatus::PendingReset,
            None,
            created_at,
        );
    }

    pub fn mark_ready(pid: Principal, cycles: Cycles, created_at: u64) {
        Self::register_or_update_state(pid, cycles, PoolStatus::Ready, None, created_at);
    }

    pub fn mark_failed(pid: Principal, err: &InternalError, created_at: u64) {
        Self::register_or_update_state(
            pid,
            Cycles::default(),
            PoolStatus::Failed {
                reason: err.to_string(),
            },
            None,
            created_at,
        );
    }

    // ---------------------------------------------------------------
    // Data (read-only)
    // ---------------------------------------------------------------

    #[must_use]
    pub fn data() -> PoolData {
        PoolStore::export()
    }

    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        PoolStore::contains(pid)
    }

    pub fn has_pending_reset() -> bool {
        PoolStore::has_status(PoolStatus::PendingReset)
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
        created_at: u64,
    ) {
        let updated = PoolStore::update_state_with(pid, |mut state| {
            state.cycles = cycles.clone();
            state.status = status.clone();

            if role.is_some() {
                state.role.clone_from(&role);
            }

            state
        });

        if !updated {
            PoolStore::register(pid, cycles, status, role, None, None, created_at);
        }
    }
}
