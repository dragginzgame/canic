use crate::{
    Error,
    cdk::structures::{
        BTreeMap as StableBTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory,
    },
    icu_register_memory,
    memory::{CanisterEntry, MemoryError, SUBNET_CHILDREN_MEMORY_ID, subnet::SubnetError},
};
use candid::Principal;
use std::cell::RefCell;

thread_local! {
    static SUBNET_CHILDREN: RefCell<SubnetChildrenCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(SubnetChildrenCore::new(
            StableBTreeMap::init(icu_register_memory!(SUBNET_CHILDREN_MEMORY_ID)),
        ));
}

///
/// SubnetChildrenView
///

pub type SubnetChildrenView = Vec<CanisterEntry>;

///
/// SubnetChildren
///

pub struct SubnetChildren;

impl SubnetChildren {
    /// Lookup a child by principal
    #[must_use]
    pub fn find_by_pid(pid: &Principal) -> Option<CanisterEntry> {
        SUBNET_CHILDREN.with_borrow(|core| core.find_by_pid(pid))
    }

    /// Same as `find_by_pid` but returns a typed error
    pub fn try_find_by_pid(pid: &Principal) -> Result<CanisterEntry, Error> {
        Self::find_by_pid(pid)
            .ok_or_else(|| MemoryError::from(SubnetError::PrincipalNotFound(*pid)).into())
    }

    /// Export state
    pub(super) fn export() -> SubnetChildrenView {
        SUBNET_CHILDREN.with_borrow(SubnetChildrenCore::export)
    }

    /// Import state (replace existing entries)
    pub fn import(data: SubnetChildrenView) {
        SUBNET_CHILDREN.with_borrow_mut(|core| core.import(data));
    }
}

///
/// SubnetChildrenCore
/// internal storage wrapper
///

pub struct SubnetChildrenCore<M: Memory>(StableBTreeMap<Principal, CanisterEntry, M>);

impl<M: Memory> SubnetChildrenCore<M> {
    pub const fn new(children: StableBTreeMap<Principal, CanisterEntry, M>) -> Self {
        Self(children)
    }

    /// Lookup by PID
    pub fn find_by_pid(&self, pid: &Principal) -> Option<CanisterEntry> {
        self.0.get(pid)
    }

    /// Insert or update
    pub fn insert(&mut self, pid: Principal, entry: CanisterEntry) {
        self.0.insert(pid, entry);
    }

    /// Remove child
    pub fn remove(&mut self, pid: &Principal) {
        self.0.remove(pid);
    }

    /// Clear all children
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Export view
    pub fn export(&self) -> SubnetChildrenView {
        self.0.iter().map(|e| e.value()).collect()
    }

    /// Import view (replace everything)
    pub fn import(&mut self, data: SubnetChildrenView) {
        self.0.clear();
        for entry in data {
            self.0.insert(entry.pid, entry);
        }
    }
}
