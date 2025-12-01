use crate::{
    Error,
    cdk::structures::{
        BTreeMap as StableBTreeMap, DefaultMemoryImpl,
        memory::{MemoryId, VirtualMemory},
    },
    impl_storable_bounded,
    model::memory::{MEMORY_MANAGER, MEMORY_RANGES_ID, MEMORY_REGISTRY_ID, MemoryError},
    types::BoundedString256,
    utils::time::now_secs,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// MEMORY_REGISTRY
//

thread_local! {
    static MEMORY_REGISTRY: RefCell<StableBTreeMap<u8, MemoryRegistryEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|this| {
                this.get(MemoryId::new(MEMORY_REGISTRY_ID))
            }),
        ));
}

//
// MEMORY_RANGES
//

thread_local! {
    static MEMORY_RANGES: RefCell<StableBTreeMap<String, MemoryRange, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|mgr| {
                mgr.get(MemoryId::new(MEMORY_RANGES_ID))
            }),
        ));
}

//
// PENDING_REGISTRATIONS
//
// Queue of memory registrations produced during TLS initialization
// Each entry is (id, crate_name, label).
// These are deferred until `flush_pending_registrations()` is called,
// which validates and inserts them into the global MemoryRegistry.
//

thread_local! {
    static PENDING_REGISTRATIONS: RefCell<Vec<(u8, &'static str, &'static str)>> = const {
        RefCell::new(Vec::new())
    };
}

pub fn defer_register(id: u8, crate_name: &'static str, label: &'static str) {
    PENDING_REGISTRATIONS.with(|q| {
        q.borrow_mut().push((id, crate_name, label));
    });
}

/// Drain (and clear) all pending registrations.
/// Intended to be called from the ops layer during init/post-upgrade.
#[must_use]
pub fn drain_pending_registrations() -> Vec<(u8, &'static str, &'static str)> {
    PENDING_REGISTRATIONS.with(|q| q.borrow_mut().drain(..).collect())
}

//
// PENDING_RANGES
//

thread_local! {
    pub static PENDING_RANGES: RefCell<Vec<(&'static str, u8, u8)>> = const {
        RefCell::new(Vec::new())
    };
}

pub fn defer_reserve_range(crate_name: &'static str, start: u8, end: u8) {
    PENDING_RANGES.with(|q| q.borrow_mut().push((crate_name, start, end)));
}

/// Drain (and clear) all pending ranges.
/// Intended to be called from the ops layer during init/post-upgrade.
#[must_use]
pub fn drain_pending_ranges() -> Vec<(&'static str, u8, u8)> {
    PENDING_RANGES.with(|q| q.borrow_mut().drain(..).collect())
}

///
/// MemoryRegistryError
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryError {
    #[error("ID {0} is already registered with type {1}, tried to register type {2}")]
    AlreadyRegistered(u8, String, String),

    #[error("crate `{0}` already has a reserved range")]
    DuplicateRange(String),

    #[error("crate `{0}` attempted to register ID {1}, but it is outside its allowed ranges")]
    OutOfRange(String, u8),

    #[error("crate `{0}` range {1}-{2} overlaps with crate `{3}` range {4}-{5}")]
    Overlap(String, u8, u8, String, u8, u8),

    #[error("crate `{0}` has not reserved any memory range")]
    NoRange(String),
}

impl From<MemoryRegistryError> for Error {
    fn from(err: MemoryRegistryError) -> Self {
        MemoryError::from(err).into()
    }
}

///
/// MemoryRange
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryRange {
    pub crate_key: BoundedString256,
    pub start: u8,
    pub end: u8,
    pub created_at: u64,
}

impl MemoryRange {
    #[must_use]
    pub fn new(crate_key: &str, start: u8, end: u8) -> Self {
        Self {
            crate_key: BoundedString256::new(crate_key),
            start,
            end,
            created_at: now_secs(),
        }
    }

    #[must_use]
    pub fn contains(&self, id: u8) -> bool {
        (self.start..=self.end).contains(&id)
    }
}

impl_storable_bounded!(MemoryRange, 320, false);

///
/// MemoryRegistryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MemoryRegistryEntry {
    pub label: BoundedString256,
    pub created_at: u64,
}

impl MemoryRegistryEntry {
    #[must_use]
    pub fn new(label: &str) -> Self {
        Self {
            label: BoundedString256::new(label),
            created_at: now_secs(),
        }
    }
}

impl_storable_bounded!(MemoryRegistryEntry, 320, false);

///
/// MemoryRegistryView
///

pub type MemoryRegistryView = Vec<(u8, MemoryRegistryEntry)>;

///
/// MemoryRegistry
///

pub struct MemoryRegistry;

impl MemoryRegistry {
    #[must_use]
    pub fn is_empty() -> bool {
        MEMORY_REGISTRY.with_borrow(|map| map.is_empty())
    }

    /// Register an ID, enforcing crate’s allowed range.
    ///
    /// Pure domain/model-level function:
    /// - no logging
    /// - no unwrap
    /// - no mapping to `crate::Error`
    pub fn register(id: u8, crate_name: &str, label: &str) -> Result<(), MemoryRegistryError> {
        let crate_key = crate_name.to_string();

        // 1. Check reserved range
        let range = MEMORY_RANGES.with_borrow(|ranges| ranges.get(&crate_key));
        match range {
            None => {
                return Err(MemoryRegistryError::NoRange(crate_key));
            }
            Some(r) if !r.contains(id) => {
                return Err(MemoryRegistryError::OutOfRange(crate_key, id));
            }
            Some(_) => {
                // OK, continue
            }
        }

        // 2. Check already registered
        let existing = MEMORY_REGISTRY.with_borrow(|map| map.get(&id));
        if let Some(existing) = existing {
            if existing.label.as_ref() != label {
                return Err(MemoryRegistryError::AlreadyRegistered(
                    id,
                    existing.label.to_string(),
                    label.to_string(),
                ));
            }
            // idempotent case
            return Ok(());
        }

        // 3. Insert
        MEMORY_REGISTRY.with_borrow_mut(|map| {
            map.insert(id, MemoryRegistryEntry::new(label));
        });

        Ok(())
    }

    /// Reserve a block of memory IDs for a crate.
    ///
    /// Pure domain/model-level function, no logging or unwrap.
    pub fn reserve_range(crate_name: &str, start: u8, end: u8) -> Result<(), MemoryRegistryError> {
        if start > end {
            // Slightly overloaded use of OutOfRange, but matches existing semantics.
            return Err(MemoryRegistryError::OutOfRange(
                crate_name.to_string(),
                start,
            ));
        }

        let crate_key = crate_name.to_string();

        // 1. Check for conflicts (existing ranges)
        let conflict = MEMORY_RANGES.with_borrow(|ranges| {
            if ranges.contains_key(&crate_key) {
                return Some(MemoryRegistryError::DuplicateRange(crate_key.clone()));
            }

            for entry in ranges.iter() {
                let other_crate = entry.key();
                let other_range = entry.value();

                if !(end < other_range.start || start > other_range.end) {
                    return Some(MemoryRegistryError::Overlap(
                        crate_key.clone(),
                        start,
                        end,
                        other_crate.clone(),
                        other_range.start,
                        other_range.end,
                    ));
                }
            }

            None
        });

        if let Some(err) = conflict {
            return Err(err);
        }

        // 2. Insert
        MEMORY_RANGES.with_borrow_mut(|ranges| {
            let range = MemoryRange::new(crate_name, start, end);
            ranges.insert(crate_name.to_string(), range);
        });

        Ok(())
    }

    #[must_use]
    pub fn get(id: u8) -> Option<MemoryRegistryEntry> {
        MEMORY_REGISTRY.with_borrow(|map| map.get(&id))
    }

    #[must_use]
    pub fn export() -> MemoryRegistryView {
        MEMORY_REGISTRY.with_borrow(|map| {
            map.iter()
                .map(|entry| (*entry.key(), entry.value()))
                .collect()
        })
    }

    #[must_use]
    pub fn export_ranges() -> Vec<(String, MemoryRange)> {
        MEMORY_RANGES.with_borrow(|ranges| {
            ranges
                .iter()
                .map(|e| (e.key().clone(), e.value()))
                .collect()
        })
    }

    pub fn clear() {
        MEMORY_REGISTRY.with_borrow_mut(StableBTreeMap::clear);
        MEMORY_RANGES.with_borrow_mut(StableBTreeMap::clear);
    }
}
