use crate::{
    cdk::structures::{DefaultMemoryImpl, Vec as StableVec, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::{CanisterSummary, id::topology::subnet::SUBNET_PARENTS_ID},
    types::CanisterType,
};
use candid::Principal;
use std::cell::RefCell;

//
// SUBNET_PARENTS
//

eager_static! {
    static SUBNET_PARENTS: RefCell<
        StableVec<CanisterSummary, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableVec::init(ic_memory!(SubnetParents, SUBNET_PARENTS_ID)),
    );
}

///
/// SubnetParents
///
/// Public API for accessing parents
///
/// This is a zero-sized handle; the actual state lives in `SUBNET_PARENTS`.
///

#[derive(Clone, Copy, Debug, Default)]
pub struct SubnetParents;

impl SubnetParents {
    /// Lookup a parent by canister principal
    #[must_use]
    pub fn find_by_pid(pid: &Principal) -> Option<CanisterSummary> {
        SUBNET_PARENTS.with_borrow(|vec| vec.iter().find(|p| &p.pid == pid))
    }

    /// Lookup a parent by canister type
    #[must_use]
    pub fn find_by_type(ty: &CanisterType) -> Option<CanisterSummary> {
        SUBNET_PARENTS.with_borrow(|vec| vec.iter().find(|p| &p.ty == ty))
    }

    /// Export current state
    #[must_use]
    pub fn export() -> Vec<CanisterSummary> {
        SUBNET_PARENTS.with_borrow(|vec| vec.iter().collect())
    }

    /// Import state (replace everything)
    pub fn import(entries: Vec<CanisterSummary>) {
        SUBNET_PARENTS.with_borrow_mut(|vec| {
            vec.clear();
            for entry in entries {
                vec.push(&entry);
            }
        });
    }
}
