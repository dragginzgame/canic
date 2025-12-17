pub use crate::model::memory::reserve::{CanisterReserveEntry, CanisterReserveView};

use crate::{
    cdk::types::Principal, ids::CanisterRole, model::memory::reserve::CanisterReserve,
    types::Cycles,
};

///
/// CanisterReserveStorageOps
/// Stable storage wrapper for the canister reserve registry.
///

pub struct CanisterReserveStorageOps;

impl CanisterReserveStorageOps {
    pub fn register(
        pid: Principal,
        cycles: Cycles,
        role: Option<CanisterRole>,
        parent: Option<Principal>,
        module_hash: Option<Vec<u8>>,
    ) {
        CanisterReserve::register(pid, cycles, role, parent, module_hash);
    }

    #[must_use]
    pub fn pop_first() -> Option<(Principal, CanisterReserveEntry)> {
        CanisterReserve::pop_first()
    }

    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        CanisterReserve::contains(pid)
    }

    #[must_use]
    pub fn take(pid: &Principal) -> Option<CanisterReserveEntry> {
        CanisterReserve::take(pid)
    }

    #[must_use]
    pub fn export() -> CanisterReserveView {
        CanisterReserve::export()
    }

    #[must_use]
    pub fn len() -> u64 {
        CanisterReserve::len()
    }
}
