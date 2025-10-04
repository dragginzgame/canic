use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory, impl_storable_bounded,
    memory::{id::topology::app::APP_TOPOLOGY_ID, topology::TopologyError},
};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

//
// APP_TOPOLOGY
// a application-wide map of every subnet_id to subnet information
//

eager_static! {
    static APP_TOPOLOGY: RefCell<BTreeMap<Principal, AppSubnet, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(SubnetDirectory, APP_TOPOLOGY_ID)));
}

///
/// AppSubnet
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppSubnet {
    pub subnet_pid: Principal,
    pub root_pid: Principal,
}

impl_storable_bounded!(AppSubnet, 64, true);

///
/// IcRegistry
///

pub struct IcRegistry;

pub type IcRegistryView = Vec<(Principal, AppSubnet)>;

impl IcRegistry {
    #[must_use]
    pub fn get(subnet_pid: Principal) -> Option<AppSubnet> {
        APP_TOPOLOGY.with_borrow(|map| map.get(&subnet_pid))
    }

    pub fn try_get(subnet_pid: Principal) -> Result<AppSubnet, Error> {
        Self::get(subnet_pid).ok_or_else(|| TopologyError::SubnetNotFound(subnet_pid).into())
    }

    pub fn import(view: IcRegistryView) {
        APP_TOPOLOGY.with_borrow_mut(|map| {
            map.clear();
            for (pid, subnet) in view {
                map.insert(pid, subnet);
            }
        });
    }

    #[must_use]
    pub fn export() -> IcRegistryView {
        APP_TOPOLOGY.with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect())
    }
}
