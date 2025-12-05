use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    model::memory::id::topology::app::APP_SUBNET_REGISTRY_ID,
    utils::impl_storable_bounded,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

//
// APP_SUBNET_REGISTRY
// An application-wide map of every subnet_id to subnet information
//

eager_static! {
    static APP_SUBNET_REGISTRY: RefCell<BTreeMap<Principal, AppSubnet, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(AppSubnetRegistry, APP_SUBNET_REGISTRY_ID)));
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
/// AppSubnetRegistry
///

pub struct AppSubnetRegistry;

pub type AppSubnetRegistryView = Vec<(Principal, AppSubnet)>;

impl AppSubnetRegistry {
    #[must_use]
    pub fn export() -> AppSubnetRegistryView {
        APP_SUBNET_REGISTRY.with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect())
    }
}
