use crate::{
    Error,
    canister::CanisterType,
    ic::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::{MemoryError, SUBNET_REGISTRY_MEMORY_ID},
    utils::time::now_secs,
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
/// CanisterStatus
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum CanisterStatus {
    Pending,
    Installed,
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
    pub canister_type: CanisterType,
    pub parent_pid: Option<Principal>,
    pub status: CanisterStatus,
    pub created_at: u64,
}

impl_storable_unbounded!(SubnetRegistryEntry);

///
/// SubnetRegistry
///

pub type SubnetRegistryView = Vec<(Principal, SubnetRegistryEntry)>;

pub struct SubnetRegistry;

impl SubnetRegistry {
    /// Initialize the registry with the root canister marked as Installed.
    pub fn init_root(root_pid: Principal) {
        let entry = SubnetRegistryEntry {
            canister_type: CanisterType::Root,
            parent_pid: None,
            status: CanisterStatus::Installed,
            created_at: now_secs(),
        };

        SUBNET_REGISTRY.with_borrow_mut(|core| core.insert(root_pid, entry));
    }

    #[must_use]
    pub fn get(pid: Principal) -> Option<SubnetRegistryEntry> {
        SUBNET_REGISTRY.with_borrow(|core| core.get(pid))
    }

    pub fn try_get(pid: Principal) -> Result<SubnetRegistryEntry, Error> {
        SUBNET_REGISTRY.with_borrow(|core| core.try_get(pid))
    }

    pub fn register_pending(pid: Principal, ty: &CanisterType, parent: Option<Principal>) {
        let entry = SubnetRegistryEntry {
            canister_type: ty.clone(),
            parent_pid: parent,
            status: CanisterStatus::Pending,
            created_at: now_secs(),
        };

        SUBNET_REGISTRY.with_borrow_mut(|core| core.insert(pid, entry));
    }

    pub fn set_status(pid: Principal, status: CanisterStatus) -> Result<(), Error> {
        SUBNET_REGISTRY.with_borrow_mut(|core| core.set_status(pid, status))
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

    pub fn insert(&mut self, pid: Principal, entry: SubnetRegistryEntry) {
        self.map.insert(pid, entry);
    }

    pub fn set_status(&mut self, pid: Principal, status: CanisterStatus) -> Result<(), Error> {
        match self.map.get(&pid) {
            Some(mut entry) => {
                entry.status = status;
                self.map.insert(pid, entry);

                Ok(())
            }
            None => Err(MemoryError::from(SubnetRegistryError::NotFound(pid)))?,
        }
    }

    pub fn export(&self) -> SubnetRegistryView {
        self.map.iter_pairs().collect()
    }
}
