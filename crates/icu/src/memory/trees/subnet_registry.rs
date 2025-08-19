use crate::{
    Error,
    ic::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::{MemoryError, SUBNET_REGISTRY_MEMORY_ID},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SUBNET_REGISTRY
//

thread_local! {
    pub static SUBNET_REGISTRY: RefCell<SubnetRegistryCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(SubnetRegistryCore::new(BTreeMap::init(
            icu_register_memory!(SUBNET_REGISTRY_MEMORY_ID),
        )));
}

///
/// SubnetRegistryError
///

#[derive(Debug, ThisError)]
pub enum SubnetRegistryError {
    #[error("canister principal not found: {0}")]
    NotFound(Principal),
}

///
/// SubnetRegistryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryEntry {
    pub canister_pid: Principal,
    pub kind: String,
    pub parent_pid: Option<Principal>,
}

impl_storable_unbounded!(SubnetRegistryEntry);

///
/// SubnetRegistry
///

pub type SubnetRegistryView = Vec<(Principal, SubnetRegistryEntry)>;

pub struct SubnetRegistry;

impl SubnetRegistry {
    #[must_use]
    pub fn get(pid: Principal) -> Option<SubnetRegistryEntry> {
        SUBNET_REGISTRY.with_borrow(|core| core.get(pid))
    }

    pub fn try_get(pid: Principal) -> Result<SubnetRegistryEntry, Error> {
        SUBNET_REGISTRY.with_borrow(|core| core.try_get(pid))
    }

    pub fn insert(entry: SubnetRegistryEntry) {
        SUBNET_REGISTRY.with_borrow_mut(|core| core.insert(entry));
    }

    #[must_use]
    pub fn export() -> SubnetRegistryView {
        SUBNET_REGISTRY.with_borrow(SubnetRegistryCore::export)
    }
}

///
/// SubnetRegistryCore
///

pub struct SubnetRegistryCore<M: Memory> {
    map: BTreeMap<Principal, SubnetRegistryEntry, M>,
}

impl<M: Memory> SubnetRegistryCore<M> {
    pub const fn new(map: BTreeMap<Principal, SubnetRegistryEntry, M>) -> Self {
        Self { map }
    }

    pub fn get(&self, pid: Principal) -> Option<SubnetRegistryEntry> {
        self.map.get(&pid)
    }

    pub fn try_get(&self, pid: Principal) -> Result<SubnetRegistryEntry, Error> {
        if let Some(entry) = self.get(pid) {
            Ok(entry)
        } else {
            Err(MemoryError::from(SubnetRegistryError::NotFound(pid)))?
        }
    }

    pub fn insert(&mut self, entry: SubnetRegistryEntry) {
        self.map.insert(entry.canister_pid, entry);
    }

    pub fn export(&self) -> SubnetRegistryView {
        self.map.iter_pairs().collect()
    }
}
