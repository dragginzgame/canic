use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::topology::SUBNET_INDEX_ID},
};
use std::cell::RefCell;

eager_static! {
    static SUBNET_INDEX: RefCell<BTreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(SubnetIndex, SUBNET_INDEX_ID)));
}

///
/// SubnetIndexRecord
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetIndexRecord {
    pub entries: Vec<(CanisterRole, Principal)>,
}

///
/// SubnetIndex
///
/// Stable-memory–backed model relation mapping subnet-scoped canister
/// roles to their principals.
///
/// Invariants:
/// - Each role appears at most once.
/// - This index is authoritative and replaced wholesale on import.
/// - DTO/snapshot representations are constructed in higher layers.
///

pub struct SubnetIndex;

impl SubnetIndex {
    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn export() -> SubnetIndexRecord {
        SubnetIndexRecord {
            entries: SUBNET_INDEX.with_borrow(|map| {
                map.iter()
                    .map(|entry| (entry.key().clone(), entry.value()))
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: SubnetIndexRecord) {
        SUBNET_INDEX.with_borrow_mut(|map| {
            map.clear();
            for (role, pid) in data.entries {
                map.insert(role, pid);
            }
        });
    }
}
