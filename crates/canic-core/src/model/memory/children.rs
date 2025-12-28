use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    model::memory::{CanisterSummary, id::children::CANISTER_CHILDREN_ID},
};
use candid::Principal;
use std::cell::RefCell;

//
// CANISTER_CHILDREN
//

eager_static! {
    static CANISTER_CHILDREN: RefCell<
        BTreeMap<Principal, CanisterSummary, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(CanisterChildren, CANISTER_CHILDREN_ID)),
    );
}

///
/// CanisterChildrenData
///

pub type CanisterChildrenData = Vec<(Principal, CanisterSummary)>;

///
/// CanisterChildren
///

pub struct CanisterChildren;

impl CanisterChildren {
    #[must_use]
    pub fn export() -> CanisterChildrenData {
        CANISTER_CHILDREN.with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect())
    }

    pub fn import(data: CanisterChildrenData) {
        CANISTER_CHILDREN.with_borrow_mut(|map| {
            map.clear();
            for (pid, entry) in data {
                map.insert(pid, entry);
            }
        });
    }
}
