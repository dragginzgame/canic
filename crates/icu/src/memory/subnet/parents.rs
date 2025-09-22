use crate::{
    cdk::structures::{DefaultMemoryImpl, Vec as StableVec, memory::VirtualMemory},
    icu_register_memory,
    memory::{CanisterEntry, SUBNET_PARENTS_MEMORY_ID},
    types::CanisterType,
};
use candid::Principal;
use std::cell::RefCell;

// thread_local
thread_local! {
    static SUBNET_PARENTS: RefCell<
        StableVec<CanisterEntry, VirtualMemory<DefaultMemoryImpl>>
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

pub type SubnetParentsView = Vec<CanisterEntry>;

#[derive(Clone, Copy, Debug, Default)]
pub struct SubnetParents;

impl SubnetParents {
    /// Lookup a parent by canister principal
    #[must_use]
    pub fn find_by_pid(&self, pid: &Principal) -> Option<CanisterEntry> {
        SUBNET_PARENTS.with_borrow(|vec| vec.iter().find(|p| &p.pid == pid))
    }

    /// Lookup a parent by canister type
    #[must_use]
    pub fn find_by_type(&self, ty: &CanisterType) -> Option<CanisterEntry> {
        SUBNET_PARENTS.with_borrow(|vec| vec.iter().find(|p| &p.ty == ty))
    }

    /// Export current state
    #[must_use]
    pub fn export(&self) -> SubnetParentsView {
        SUBNET_PARENTS.with_borrow(|vec| vec.iter().collect())
    }

    /// Import state (replace everything)
    pub fn import(&self, data: SubnetParentsView) {
        SUBNET_PARENTS.with_borrow_mut(|vec| {
            vec.clear();
            for entry in data {
                vec.push(&entry);
            }
        });
    }

    /// Clear all parents
    pub fn clear(&self) {
        SUBNET_PARENTS.with_borrow_mut(|vec| vec.clear());
    }
}
