//! Module: storage::stable::registry::app
//!
//! Responsibility: persist subnet-to-root application registry bindings.
//! Does not own: topology policy, workflow orchestration, or endpoint DTOs.
//! Boundary: exports canonical app-registry data for storage ops consumers.

use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    role_contract::allocation::memory::topology::APP_REGISTRY_ID,
    storage::prelude::*,
};
use ic_memory::stable_structures::btreemap::BTreeMap as StableBtreeMap;
use std::cell::RefCell;

//
// APP_REGISTRY
// An application-wide map of every subnet principal to its
// corresponding root principal
//

eager_static! {
    static APP_REGISTRY: RefCell<StableBtreeMap<Principal, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBtreeMap::init(crate::ic_memory_key!("canic.core.app_registry.v1", AppRegistry, APP_REGISTRY_ID)));
}

///
/// AppRegistryEntryRecord
///
/// One logical app-registry row.
///

#[derive(Clone, Debug)]
pub struct AppRegistryEntryRecord {
    pub subnet_pid: Principal,
    pub root_pid: Principal,
}

impl AppRegistryEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "AppRegistryEntryRecord";
}

///
/// AppRegistryData
///
/// Canonical app-registry export snapshot.
///

#[derive(Clone, Debug)]
pub struct AppRegistryData {
    pub entries: Vec<AppRegistryEntryRecord>,
}

impl AppRegistryData {
    pub const STATE_CONTRACT_NAME: &'static str = "AppRegistryData";
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
    pub(crate) fn export() -> AppRegistryData {
        AppRegistryData {
            entries: APP_REGISTRY.with_borrow(|map| {
                map.iter()
                    .map(|entry| AppRegistryEntryRecord {
                        subnet_pid: *entry.key(),
                        root_pid: entry.value(),
                    })
                    .collect()
            }),
        }
    }
}
