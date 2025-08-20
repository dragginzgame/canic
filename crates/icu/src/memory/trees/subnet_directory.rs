use crate::{
    Error,
    canister::CanisterType,
    ic::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::{MemoryError, SUBNET_DIRECTORY_MEMORY_ID},
    state::canister::CanisterRegistry,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SUBNET_DIRECTORY
//

thread_local! {
    pub static SUBNET_DIRECTORY: RefCell<SubnetDirectoryCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(SubnetDirectoryCore::new(BTreeMap::init(
            icu_register_memory!(SUBNET_DIRECTORY_MEMORY_ID),
        )));
}

///
/// SubnetDirectoryError
///

#[derive(Debug, ThisError)]
pub enum SubnetDirectoryError {
    #[error("canister not found: {0}")]
    NotFound(CanisterType),

    #[error("canister type '{0}' is not a singleton")]
    NotSingleton(CanisterType),

    #[error("canister type '{0}' cannot be in the directory")]
    NotDirectory(CanisterType),

    #[error("subnet directory capacity reached for type '{ty}' (cap {cap})")]
    CapacityReached { ty: CanisterType, cap: u16 },
}

///
/// SubnetDirectoryEntry
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct SubnetDirectoryEntry {
    pub canisters: Vec<Principal>,
}

impl_storable_unbounded!(SubnetDirectoryEntry);

///
/// SubnetDirectory
///

pub type SubnetDirectoryView = Vec<(CanisterType, SubnetDirectoryEntry)>;

pub struct SubnetDirectory;

impl SubnetDirectory {
    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<SubnetDirectoryEntry> {
        SUBNET_DIRECTORY.with_borrow(|core| core.get(ty))
    }

    pub fn try_get(ty: &CanisterType) -> Result<SubnetDirectoryEntry, Error> {
        SUBNET_DIRECTORY.with_borrow(|core| core.try_get(ty))
    }

    pub fn try_get_singleton(ty: &CanisterType) -> Result<Principal, Error> {
        SUBNET_DIRECTORY.with_borrow(|core| core.try_get_singleton(ty))
    }

    pub fn can_insert(ty: &CanisterType) -> Result<(), Error> {
        SUBNET_DIRECTORY.with_borrow(|core| core.can_insert(ty))
    }

    pub fn insert(ty: CanisterType, id: Principal) -> Result<(), Error> {
        SUBNET_DIRECTORY.with_borrow_mut(|core| core.insert(ty, id))
    }

    pub fn remove(ty: &CanisterType, id: Principal) -> Result<(), Error> {
        SUBNET_DIRECTORY.with_borrow_mut(|core| core.remove(ty, id))
    }

    pub fn import(view: SubnetDirectoryView) {
        SUBNET_DIRECTORY.with_borrow_mut(|core| core.import(view));
    }

    #[must_use]
    pub fn export() -> SubnetDirectoryView {
        SUBNET_DIRECTORY.with_borrow(SubnetDirectoryCore::export)
    }
}

///
/// SubnetDirectoryCore
///

pub struct SubnetDirectoryCore<M: Memory> {
    map: BTreeMap<CanisterType, SubnetDirectoryEntry, M>,
}

impl<M: Memory> SubnetDirectoryCore<M> {
    pub const fn new(map: BTreeMap<CanisterType, SubnetDirectoryEntry, M>) -> Self {
        Self { map }
    }

    pub fn get(&self, ty: &CanisterType) -> Option<SubnetDirectoryEntry> {
        self.map.get(ty)
    }

    pub fn try_get(&self, ty: &CanisterType) -> Result<SubnetDirectoryEntry, Error> {
        if let Some(entry) = self.get(ty) {
            Ok(entry)
        } else {
            Err(MemoryError::from(SubnetDirectoryError::NotFound(
                ty.clone(),
            )))?
        }
    }

    pub fn try_get_singleton(&self, ty: &CanisterType) -> Result<Principal, Error> {
        let entry = self.try_get(ty)?;

        if entry.canisters.len() == 1 {
            Ok(entry.canisters[0])
        } else {
            Err(MemoryError::from(SubnetDirectoryError::NotSingleton(
                ty.clone(),
            )))?
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn can_insert(&self, ty: &CanisterType) -> Result<(), Error> {
        let canister = CanisterRegistry::try_get(ty)?;

        match canister.attributes.directory.limit() {
            Some(cap) => {
                let entry = self.get(ty).unwrap_or_default();

                if (entry.canisters.len() as u16) >= cap {
                    Err(MemoryError::from(SubnetDirectoryError::CapacityReached {
                        ty: ty.clone(),
                        cap,
                    }))
                } else {
                    Ok(())
                }
            }
            None => Err(MemoryError::from(SubnetDirectoryError::NotDirectory(
                ty.clone(),
            ))),
        }?;

        Ok(())
    }

    pub fn insert(&mut self, ty: CanisterType, id: Principal) -> Result<(), Error> {
        self.can_insert(&ty)?;

        let mut entry = self.get(&ty).unwrap_or_default();

        if !entry.canisters.contains(&id) {
            entry.canisters.push(id);
            self.map.insert(ty, entry);
        }

        Ok(())
    }

    pub fn remove(&mut self, ty: &CanisterType, id: Principal) -> Result<(), Error> {
        if let Some(mut entry) = self.get(ty) {
            entry.canisters.retain(|p| p != &id);

            if entry.canisters.is_empty() {
                self.map.remove(ty);
            } else {
                self.map.insert(ty.clone(), entry);
            }
        }

        Ok(())
    }

    pub fn import(&mut self, view: SubnetDirectoryView) {
        self.map.clear();
        for (k, v) in view {
            self.map.insert(k.clone(), v);
        }
    }

    pub fn export(&self) -> SubnetDirectoryView {
        self.map.iter_pairs().collect()
    }
}
