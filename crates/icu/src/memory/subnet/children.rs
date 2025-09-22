use crate::{
    Error,
    cdk::structures::{BTreeMap as StableBTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    icu_register_memory,
    memory::{CanisterEntry, MemoryError, SUBNET_CHILDREN_MEMORY_ID, subnet::SubnetError},
    types::CanisterType,
};
use candid::Principal;
use std::cell::RefCell;

// thread_local
thread_local! {
    static SUBNET_CHILDREN: RefCell<
        StableBTreeMap<Principal, CanisterEntry, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBTreeMap::init(icu_register_memory!(SUBNET_CHILDREN_MEMORY_ID)),
    );
}

///
/// SubnetChildren
///
/// Public API for accessing children
/// This is a zero-sized handle; the actual state lives in `SUBNET_CHILDREN`.
///

pub type SubnetChildrenView = Vec<CanisterEntry>;

#[derive(Clone, Copy, Debug, Default)]
pub struct SubnetChildren;

impl SubnetChildren {
    /// Lookup a child by principal
    #[must_use]
    pub fn find_by_pid(&self, pid: &Principal) -> Option<CanisterEntry> {
        SUBNET_CHILDREN.with_borrow(|map| map.get(pid))
    }

    /// Same as `find_by_pid` but returns a typed error
    pub fn try_find_by_pid(&self, pid: &Principal) -> Result<CanisterEntry, Error> {
        self.find_by_pid(pid)
            .ok_or_else(|| MemoryError::from(SubnetError::PrincipalNotFound(*pid)).into())
    }

    /// Lookup all children of a given type
    #[must_use]
    pub fn find_by_type(&self, ty: &CanisterType) -> Vec<CanisterEntry> {
        SUBNET_CHILDREN.with_borrow(|map| {
            map.iter()
                .filter_map(|e| {
                    let value = e.value();
                    if value.ty == *ty { Some(value) } else { None }
                })
                .collect()
        })
    }

    /// Lookup the first child of a given type
    #[must_use]
    pub fn find_first_by_type(&self, ty: &CanisterType) -> Option<CanisterEntry> {
        SUBNET_CHILDREN.with_borrow(|map| {
            map.iter().find_map(|e| {
                let value = e.value();
                if value.ty == *ty { Some(value) } else { None }
            })
        })
    }

    /// Insert or update a child
    pub fn insert(&self, pid: Principal, entry: CanisterEntry) {
        SUBNET_CHILDREN.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
    }

    /// Remove a child
    pub fn remove(&self, pid: &Principal) {
        SUBNET_CHILDREN.with_borrow_mut(|map| {
            map.remove(pid);
        });
    }

    /// Clear all children
    pub fn clear(&self) {
        SUBNET_CHILDREN.with_borrow_mut(|map| map.clear());
    }

    /// Export state
    #[must_use]
    pub fn export(&self) -> SubnetChildrenView {
        SUBNET_CHILDREN.with_borrow(|map| map.iter().map(|e| e.value()).collect())
    }

    /// Import state (replace everything)
    pub fn import(&self, data: SubnetChildrenView) {
        SUBNET_CHILDREN.with_borrow_mut(|map| {
            map.clear();
            for entry in data {
                map.insert(entry.pid, entry);
            }
        });
    }
}
