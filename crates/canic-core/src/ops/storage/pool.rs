use crate::{
    Error,
    cdk::types::Principal,
    dto::pool::{CanisterPoolEntryView, CanisterPoolView},
    ids::CanisterRole,
    model::memory::pool::{CanisterPool, CanisterPoolEntry, CanisterPoolStatus},
    ops::{
        adapter::pool::{canister_pool_entry_to_view, canister_pool_to_view},
        config::ConfigOps,
        prelude::*,
    },
};

///
/// PoolOps
/// Stable storage wrapper for the canister pool registry.
///

pub struct PoolOps;

impl PoolOps {
    pub fn register_ready(
        pid: Principal,
        cycles: Cycles,
        role: Option<CanisterRole>,
        parent: Option<Principal>,
        module_hash: Option<Vec<u8>>,
    ) {
        CanisterPool::register(
            pid,
            cycles,
            CanisterPoolStatus::Ready,
            role,
            parent,
            module_hash,
        );
    }

    pub fn mark_pending_reset(pid: Principal) {
        Self::register_or_update(
            pid,
            Cycles::default(),
            CanisterPoolStatus::PendingReset,
            None,
        );
    }

    pub fn mark_ready(pid: Principal, cycles: Cycles) {
        Self::register_or_update(pid, cycles, CanisterPoolStatus::Ready, None);
    }

    pub fn mark_failed(pid: Principal, err: &Error) {
        let status = CanisterPoolStatus::Failed {
            reason: err.to_string(),
        };
        Self::register_or_update(pid, Cycles::default(), status, None);
    }

    #[must_use]
    pub fn get_view(pid: Principal) -> Option<CanisterPoolEntryView> {
        CanisterPool::get(pid).map(|entry| canister_pool_entry_to_view(&entry))
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
    pub fn export_view() -> CanisterPoolView {
        let data = CanisterPool::export();

        canister_pool_to_view(data)
    }

    #[must_use]
    pub fn len() -> u64 {
        CanisterPool::len()
    }

    fn register_or_update(
        pid: Principal,
        cycles: Cycles,
        status: CanisterPoolStatus,
        role: Option<CanisterRole>,
    ) {
        if let Some(mut entry) = CanisterPool::get(pid) {
            entry.cycles = cycles;
            entry.status = status;
            entry.role = role.or(entry.role);
            let _ = CanisterPool::update(pid, entry);
        } else {
            CanisterPool::register(pid, cycles, status, role, None, None);
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
#[must_use]
pub fn pool_controllers() -> Vec<Principal> {
    let mut controllers = ConfigOps::controllers();

    let root = canister_self();
    if !controllers.contains(&root) {
        controllers.push(root);
    }

    controllers
}
