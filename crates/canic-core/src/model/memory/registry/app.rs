use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::impl_storable_bounded,
    model::memory::id::registry::APP_REGISTRY_ID,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

//
// APP_REGISTRY
// An application-wide map of every subnet_id to subnet information
//

eager_static! {
    static APP_REGISTRY: RefCell<BTreeMap<Principal, AppSubnet, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(AppRegistry, APP_REGISTRY_ID)));
}

///
/// AppSubnet
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppSubnet {
    pub subnet_pid: Principal,
    pub root_pid: Principal,
}

impl_storable_bounded!(AppSubnet, 64, true);

///
/// AppRegistryView
///

pub type AppRegistryView = Vec<(Principal, AppSubnet)>;

///
/// AppRegistry
///

pub struct AppRegistry;

impl AppRegistry {
    #[must_use]
    pub(crate) fn export() -> AppRegistryView {
        APP_REGISTRY.with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect())
    }
}
