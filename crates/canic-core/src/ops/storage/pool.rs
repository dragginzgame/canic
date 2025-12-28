pub use crate::model::memory::pool::{CanisterPoolData, CanisterPoolEntry, CanisterPoolStatus};

use crate::{
    cdk::types::Principal,
    dto::pool::CanisterPoolView,
    ids::CanisterRole,
    model::memory::pool::CanisterPool,
    ops::{adapter::pool::canister_pool_to_view, config::ConfigOps, prelude::*},
};

///
/// PoolOps
/// Stable storage wrapper for the canister pool registry.
///

pub struct PoolOps;

impl PoolOps {
    pub fn register(
        pid: Principal,
        cycles: Cycles,
        status: CanisterPoolStatus,
        role: Option<CanisterRole>,
        parent: Option<Principal>,
        module_hash: Option<Vec<u8>>,
    ) {
        CanisterPool::register(pid, cycles, status, role, parent, module_hash);
    }

    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterPoolEntry> {
        CanisterPool::get(pid)
    }

    #[must_use]
    pub fn update(pid: Principal, entry: CanisterPoolEntry) -> bool {
        CanisterPool::update(pid, entry)
    }

    #[must_use]
    pub fn pop_ready() -> Option<(Principal, CanisterPoolEntry)> {
        CanisterPool::pop_ready()
    }

    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        CanisterPool::contains(pid)
    }

    #[must_use]
    pub fn take(pid: &Principal) -> Option<CanisterPoolEntry> {
        CanisterPool::take(pid)
    }

    #[must_use]
    pub fn export() -> CanisterPoolData {
        CanisterPool::export()
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
