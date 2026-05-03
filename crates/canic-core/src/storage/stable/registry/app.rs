use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::topology::APP_REGISTRY_ID},
};
use std::cell::RefCell;

//
// APP_REGISTRY
// An application-wide map of every subnet principal to its
// corresponding root principal
//

eager_static! {
    static APP_REGISTRY: RefCell<BTreeMap<Principal, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(AppRegistry, APP_REGISTRY_ID)));
}

///
/// AppRegistryRecord
///

#[derive(Clone, Debug)]
pub struct AppRegistryRecord {
    pub entries: Vec<(Principal, Principal)>,
}

///
/// AppRegistry
///
/// Stable-memory–backed model relation mapping subnet principals to their
/// corresponding root principals.
///
/// This registry is authoritative and is populated via internal lifecycle
/// operations. It is exported for snapshot/view construction but is not
/// imported wholesale.
///

pub struct AppRegistry;

impl AppRegistry {
    /// Insert or replace the root principal recorded for a subnet.
    pub(crate) fn upsert(subnet_pid: Principal, root_pid: Principal) {
        APP_REGISTRY.with_borrow_mut(|map| {
            map.insert(subnet_pid, root_pid);
        });
    }

    #[must_use]
    pub(crate) fn export() -> AppRegistryRecord {
        AppRegistryRecord {
            entries: APP_REGISTRY
                .with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect()),
        }
    }
}
