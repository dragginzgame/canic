use crate::{
    cdk::structures::{DefaultMemoryImpl, Memory, Vec as StableVec, memory::VirtualMemory},
    icu_register_memory,
    memory::{CanisterEntry, SUBNET_PARENTS_MEMORY_ID},
    types::CanisterType,
};
use candid::Principal;
use std::cell::RefCell;

thread_local! {
    static SUBNET_PARENTS: RefCell<SubnetParentsCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(SubnetParentsCore::new(
            StableVec::init(icu_register_memory!(SUBNET_PARENTS_MEMORY_ID)),
        ));
}

///
/// SubnetParentsView
/// view of all subnet parents
///

pub type SubnetParentsView = Vec<CanisterEntry>;

///
/// SubnetParents
///

pub struct SubnetParents;

impl SubnetParents {
    /// Lookup by canister principal
    #[must_use]
    pub fn find_by_pid(pid: &Principal) -> Option<CanisterEntry> {
        SUBNET_PARENTS.with_borrow(|core| core.find_by_pid(pid))
    }

    /// Lookup by canister type
    #[must_use]
    pub fn find_by_type(ty: &CanisterType) -> Option<CanisterEntry> {
        SUBNET_PARENTS.with_borrow(|core| core.find_by_type(ty))
    }

    /// Export current state
    pub(super) fn export() -> SubnetParentsView {
        SUBNET_PARENTS.with_borrow(SubnetParentsCore::export)
    }

    /// Import state (replace existing entries)
    pub fn import(data: SubnetParentsView) {
        SUBNET_PARENTS.with_borrow_mut(|core| core.import(data));
    }
}

///
/// SubnetParentsCore
///

pub struct SubnetParentsCore<M: Memory>(StableVec<CanisterEntry, M>);

impl<M: Memory> SubnetParentsCore<M> {
    pub const fn new(parents: StableVec<CanisterEntry, M>) -> Self {
        Self(parents)
    }

    /// Find by pid
    pub fn find_by_pid(&self, pid: &Principal) -> Option<CanisterEntry> {
        self.0.iter().find(|p| &p.pid == pid)
    }

    /// Find by type
    pub fn find_by_type(&self, ty: &CanisterType) -> Option<CanisterEntry> {
        self.0.iter().find(|p| &p.ty == ty)
    }

    /// Export all entries
    pub fn export(&self) -> SubnetParentsView {
        self.0.iter().collect()
    }

    /// Replace all entries
    pub fn import(&mut self, data: SubnetParentsView) {
        self.0.clear();
        for entry in data {
            self.0.push(&entry);
        }
    }
}
