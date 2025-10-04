use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::{CanisterSummary, id::topology::subnet::SUBNET_CHILDREN_ID, topology::TopologyError},
    types::CanisterType,
};
use candid::Principal;
use std::cell::RefCell;

//
// SUBNET_CHILDREN
//

eager_static! {
    static SUBNET_CHILDREN: RefCell<
        BTreeMap<Principal, CanisterSummary, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(SubnetChildren, SUBNET_CHILDREN_ID)),
    );
}

///
/// SubnetChildren
///
/// Public API for accessing children
/// This is a zero-sized handle; the actual state lives in `SUBNET_CHILDREN`.
///

pub struct SubnetChildren;

impl SubnetChildren {
    /// Lookup a child by principal
    #[must_use]
    pub fn find_by_pid(pid: &Principal) -> Option<CanisterSummary> {
        SUBNET_CHILDREN.with_borrow(|map| map.get(pid))
    }

    /// Same as `find_by_pid` but returns a typed error
    pub fn try_find_by_pid(pid: &Principal) -> Result<CanisterSummary, Error> {
        Self::find_by_pid(pid).ok_or_else(|| TopologyError::PrincipalNotFound(*pid).into())
    }

    /// Lookup all children of a given type
    #[must_use]
    pub fn find_by_type(ty: &CanisterType) -> Vec<CanisterSummary> {
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
    pub fn find_first_by_type(ty: &CanisterType) -> Option<CanisterSummary> {
        SUBNET_CHILDREN.with_borrow(|map| {
            map.iter().find_map(|e| {
                let value = e.value();
                if value.ty == *ty { Some(value) } else { None }
            })
        })
    }

    /// Clear all children
    pub fn clear() {
        SUBNET_CHILDREN.with_borrow_mut(BTreeMap::clear);
    }

    /// Export state
    #[must_use]
    pub fn export() -> Vec<CanisterSummary> {
        SUBNET_CHILDREN.with_borrow(|map| map.iter().map(|e| e.value()).collect())
    }

    /// Import state (replace everything)
    pub fn import(data: Vec<CanisterSummary>) {
        SUBNET_CHILDREN.with_borrow_mut(|map| {
            map.clear();
            for entry in data {
                map.insert(entry.pid, entry);
            }
        });
    }
}
