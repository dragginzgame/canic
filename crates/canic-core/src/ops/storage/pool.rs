use crate::{
    Error,
    cdk::utils::time::now_secs,
    ops::prelude::*,
    storage::memory::pool::{
        CanisterPool, CanisterPoolData, CanisterPoolEntry, CanisterPoolState, CanisterPoolStatus,
    },
};

///
/// PoolOps
/// Stable storage wrapper for the canister pool registry.
///
/// This module contains *storage* operations:
/// - register entries
/// - mutate existing entry state (without reordering)
/// - export and basic lookups
///

pub struct PoolOps;

impl PoolOps {
    //
    // ---- Registration ----
    //

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

    //
    // ---- State transitions ----
    //

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

    // ------- Export ------------------------

    #[must_use]
    pub fn export() -> CanisterPoolData {
        CanisterPool::export()
    }

    //
    // ---- Mechanical storage access ----
    //

    /// Fetch a pool entry by canister id.
    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterPoolEntry> {
        CanisterPool::get(pid)
    }

    #[must_use]
    pub(crate) fn pop_ready() -> Option<(Principal, CanisterPoolEntry)> {
        CanisterPool::pop_ready()
    }

    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        CanisterPool::contains(pid)
    }

    #[must_use]
    pub(crate) fn take(pid: &Principal) -> Option<CanisterPoolEntry> {
        CanisterPool::take(pid)
    }

    #[must_use]
    pub fn len() -> u64 {
        CanisterPool::len()
    }

    /// Iterate over all pool entries (read-only, internal).
    pub fn iter() -> impl Iterator<Item = (Principal, CanisterPoolEntry)> {
        // Clone data out so the iterator does not hold a borrow
        CanisterPool::export().entries.into_iter()
    }
    //
    // ---- Internal helper ----
    //

    fn register_or_update_state(
        pid: Principal,
        cycles: Cycles,
        status: CanisterPoolStatus,
        role: Option<CanisterRole>,
    ) {
        // Try update first: preserves header/created_at by construction.
        let updated = CanisterPool::update_state_with(pid, |mut state: CanisterPoolState| {
            state.cycles = cycles.clone();
            state.status = status.clone();

            // Preserve existing role unless caller supplies a replacement.
            if role.is_some() {
                state.role.clone_from(&role);
            }

            state
        });

        if !updated {
            // For new entries, we don’t know parent/module_hash here (same as before).
            let created_at = now_secs();
            CanisterPool::register(pid, cycles, status, role, None, None, created_at);
        }
    }
}
