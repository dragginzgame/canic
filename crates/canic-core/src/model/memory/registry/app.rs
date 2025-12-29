use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    model::memory::id::registry::APP_REGISTRY_ID,
};
use candid::Principal;
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
/// AppRegistryData
///

pub type AppRegistryData = Vec<(Principal, Principal)>;

///
/// AppRegistry
///

pub struct AppRegistry;

impl AppRegistry {
    #[must_use]
    pub(crate) fn export() -> AppRegistryData {
        APP_REGISTRY.with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect())
    }
}
