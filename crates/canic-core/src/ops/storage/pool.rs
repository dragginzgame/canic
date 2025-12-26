pub use crate::model::memory::pool::{CanisterPoolEntry, CanisterPoolStatus, CanisterPoolView};

use crate::types::Cycles;
use crate::{cdk::types::Principal, ids::CanisterRole, model::memory::pool::CanisterPool};

///
/// CanisterPoolStorageOps
/// Stable storage wrapper for the canister pool registry.
///

pub struct CanisterPoolStorageOps;

impl CanisterPoolStorageOps {
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
    pub fn export() -> CanisterPoolView {
        CanisterPool::export()
    }

    #[must_use]
    pub fn len() -> u64 {
        CanisterPool::len()
    }
}
