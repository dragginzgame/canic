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
/// CanisterChildren
/// Public API for accessing children
///

pub struct CanisterChildren;

impl CanisterChildren {
    /// Export state
    #[must_use]
    pub(crate) fn export() -> Vec<CanisterSummary> {
        CANISTER_CHILDREN.with_borrow(|map| map.iter().map(|e| e.value()).collect())
    }

    /// Import state (replace everything)
    pub(crate) fn import(data: Vec<CanisterSummary>) {
        CANISTER_CHILDREN.with_borrow_mut(|map| {
            map.clear();
            for entry in data {
                map.insert(entry.pid, entry);
            }
        });
    }
}
