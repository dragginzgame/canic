//! Canister Registry (root-authoritative)
//!
//! Purpose
//! - Authoritative ledger of canisters managed by root: type, parent, lifecycle status,
//!   and optional module hash.
//! - Drives operational flows (create/install) and serves as the source for generating
//!   the directory read-model.
//!
//! Lifecycle
//! - `init_root` inserts root as Installed at startup.
//! - `create(pid, ty, parent)` records a new canister as Created immediately after allocation.
//! - `install(pid, module_hash)` flips to Installed once code is installed and records the hash.
//! - `export()` is used by root to derive the directory view.
//!
//! Invariants
//! - An Installed canister remains Installed (idempotent guard on `install`).
//! - Every PID in the registry has an associated `CanisterType`.
//!

use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::{CANISTER_REGISTRY_MEMORY_ID, MemoryError},
    types::CanisterType,
    utils::time::now_secs,
};
use candid::{CandidType, Principal};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// CANISTER_REGISTRY
// (root-only)
//

thread_local! {
    pub static CANISTER_REGISTRY: RefCell<CanisterRegistryCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(CanisterRegistryCore::new(BTreeMap::init(
            icu_register_memory!(CANISTER_REGISTRY_MEMORY_ID),
        )));
}

///
/// CanisterStatus
///

#[derive(CandidType, Clone, Debug, Deserialize, Display, Eq, PartialEq, Serialize)]
pub enum CanisterStatus {
    Created,
    Installed,
}

///
/// CanisterRegistryError
///

#[derive(Debug, ThisError)]
pub enum CanisterRegistryError {
    #[error("canister already installed: {0}")]
    AlreadyInstalled(Principal),

    #[error("canister principal not found: {0}")]
    NotFound(Principal),
}

///
/// CanisterRegistryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterRegistryEntry {
    pub canister_type: CanisterType,
    pub parent_pid: Option<Principal>,
    pub status: CanisterStatus,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

impl_storable_unbounded!(CanisterRegistryEntry);

///
/// CanisterRegistry
///

pub type CanisterRegistryView = Vec<(Principal, CanisterRegistryEntry)>;

pub struct CanisterRegistry;

impl CanisterRegistry {
    /// Initialize the registry with the root canister marked as Installed.
    pub fn init_root(root_pid: Principal) {
        let entry = CanisterRegistryEntry {
            canister_type: CanisterType::ROOT,
            parent_pid: None,
            status: CanisterStatus::Installed,
            module_hash: None,
            created_at: now_secs(),
        };

        CANISTER_REGISTRY.with_borrow_mut(|core| core.insert(root_pid, entry));
    }

    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterRegistryEntry> {
        CANISTER_REGISTRY.with_borrow(|core| core.get(pid))
    }

    pub fn try_get(pid: Principal) -> Result<CanisterRegistryEntry, Error> {
        CANISTER_REGISTRY.with_borrow(|core| core.try_get(pid))
    }

    pub fn create(pid: Principal, ty: &CanisterType, parent: Option<Principal>) {
        let entry = CanisterRegistryEntry {
            canister_type: ty.clone(),
            parent_pid: parent,
            status: CanisterStatus::Created,
            module_hash: None,
            created_at: now_secs(),
        };

        CANISTER_REGISTRY.with_borrow_mut(|core| core.insert(pid, entry));
    }

    pub fn install(pid: Principal, module_hash: Vec<u8>) -> Result<(), Error> {
        CANISTER_REGISTRY.with_borrow_mut(|core| match core.map.get(&pid) {
            Some(mut entry) => {
                if entry.status == CanisterStatus::Installed {
                    return Err(MemoryError::from(CanisterRegistryError::AlreadyInstalled(
                        pid,
                    )))?;
                }

                entry.status = CanisterStatus::Installed;
                entry.module_hash = Some(module_hash);

                core.map.insert(pid, entry);

                Ok(())
            }
            None => Err(MemoryError::from(CanisterRegistryError::NotFound(pid)))?,
        })
    }

    #[must_use]
    pub fn remove(pid: &Principal) -> Option<CanisterRegistryEntry> {
        CANISTER_REGISTRY.with_borrow_mut(|core| core.remove(pid))
    }

    #[must_use]
    pub fn export() -> CanisterRegistryView {
        CANISTER_REGISTRY.with_borrow(CanisterRegistryCore::export)
    }
}

///
/// CanisterRegistryCore
///

pub struct CanisterRegistryCore<M: Memory> {
    map: BTreeMap<Principal, CanisterRegistryEntry, M>,
}

impl<M: Memory> CanisterRegistryCore<M> {
    pub const fn new(map: BTreeMap<Principal, CanisterRegistryEntry, M>) -> Self {
        Self { map }
    }

    pub fn get(&self, pid: Principal) -> Option<CanisterRegistryEntry> {
        self.map.get(&pid)
    }

    pub fn try_get(&self, pid: Principal) -> Result<CanisterRegistryEntry, Error> {
        if let Some(entry) = self.get(pid) {
            Ok(entry)
        } else {
            Err(MemoryError::from(CanisterRegistryError::NotFound(pid)))?
        }
    }

    pub fn insert(&mut self, pid: Principal, entry: CanisterRegistryEntry) {
        self.map.insert(pid, entry);
    }

    pub fn remove(&mut self, pid: &Principal) -> Option<CanisterRegistryEntry> {
        self.map.remove(pid)
    }

    pub fn set_status(&mut self, pid: Principal, status: CanisterStatus) -> Result<(), Error> {
        match self.map.get(&pid) {
            Some(mut entry) => {
                entry.status = status;
                self.map.insert(pid, entry);

                Ok(())
            }
            None => Err(MemoryError::from(CanisterRegistryError::NotFound(pid)))?,
        }
    }

    pub fn export(&self) -> CanisterRegistryView {
        self.map.to_vec()
    }
}
