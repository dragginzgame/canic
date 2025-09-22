use crate::{
    cdk::structures::{DefaultMemoryImpl, Vec as StableVec, memory::VirtualMemory},
    icu_register_memory,
    memory::{CanisterView, SUBNET_PARENTS_MEMORY_ID},
    types::CanisterType,
};
use candid::Principal;
use std::cell::RefCell;

// thread_local
thread_local! {
    static SUBNET_PARENTS: RefCell<
        StableVec<CanisterView, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableVec::init(icu_register_memory!(SUBNET_PARENTS_MEMORY_ID)),
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
    pub fn find_by_pid(pid: &Principal) -> Option<CanisterView> {
        SUBNET_PARENTS.with_borrow(|vec| vec.iter().find(|p| &p.pid == pid))
    }

    /// Lookup a parent by canister type
    #[must_use]
    pub fn find_by_type(ty: &CanisterType) -> Option<CanisterView> {
        SUBNET_PARENTS.with_borrow(|vec| vec.iter().find(|p| &p.ty == ty))
    }

    /// Export current state
    #[must_use]
    pub fn export() -> Vec<CanisterView> {
        SUBNET_PARENTS.with_borrow(|vec| vec.iter().collect())
    }

    /// Import state (replace everything)
    pub fn import(entries: Vec<CanisterView>) {
        SUBNET_PARENTS.with_borrow_mut(|vec| {
            vec.clear();
            for entry in entries {
                vec.push(&entry);
            }
        });
    }

    /// Clear all parents
    pub fn clear() {
        SUBNET_PARENTS.with_borrow_mut(StableVec::clear);
    }
}
