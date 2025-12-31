use crate::{
    Error,
    cdk::utils::time::now_secs,
    dto::pool::{CanisterPoolEntryView, CanisterPoolView},
    ops::{
        adapter::pool::{canister_pool_entry_to_view, canister_pool_to_view},
        config::ConfigOps,
        prelude::*,
    },
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

    //
    // ---- Views ----
    //

    #[must_use]
    pub fn get_view(pid: Principal) -> Option<CanisterPoolEntryView> {
        CanisterPool::get(pid).map(|entry| canister_pool_entry_to_view(&entry.header, &entry.state))
    }

    // ------- Export ------------------------

    #[must_use]
    pub fn export() -> CanisterPoolData {
        CanisterPool::export()
    }

    #[must_use]
    pub fn export_view() -> CanisterPoolView {
        let data = CanisterPool::export();
        canister_pool_to_view(data)
    }

    //
    // ---- Mechanical storage access ----
    //

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

/// Return the controller set for pool canisters.
///
/// Mechanical helper used by workflow when creating or resetting
/// pool canisters.
///
/// Guarantees:
/// - Includes all configured controllers from `Config`
/// - Always includes the root canister as a controller
/// - Deduplicates the root if already present
///
/// This function:
/// - Does NOT perform authorization checks
/// - Does NOT mutate state
/// - Does NOT make IC calls
///
/// Policy decisions about *who* should control pool canisters
/// are assumed to be encoded in configuration.
pub fn pool_controllers() -> Result<Vec<Principal>, Error> {
    let mut controllers = ConfigOps::controllers()?;

    let root = canister_self();
    if !controllers.contains(&root) {
        controllers.push(root);
    }

    Ok(controllers)
}
