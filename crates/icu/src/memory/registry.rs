use crate::{
    Error, Log,
    cdk::structures::{
        BTreeMap as StableBTreeMap, DefaultMemoryImpl,
        memory::{MemoryId, VirtualMemory},
    },
    impl_storable_bounded, log,
    memory::{
        ICU_MEMORY_MAX, ICU_MEMORY_MIN, MEMORY_MANAGER, MEMORY_RANGES_ID, MEMORY_REGISTRY_ID,
        MemoryError,
    },
    types::BoundedString64,
    utils::time::now_secs,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

// MEMORY_REGISTRY
thread_local! {
    static MEMORY_REGISTRY: RefCell<StableBTreeMap<u8, MemoryRegistryEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|this| {
                this.get(MemoryId::new(MEMORY_REGISTRY_ID))
            }),
        ));
}

// MEMORY_RANGE
thread_local! {
    static MEMORY_RANGES: RefCell<StableBTreeMap<String, MemoryRange, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|mgr| {
                mgr.get(MemoryId::new(MEMORY_RANGES_ID))
            }),
        ));
}

//
// TLS_INITIALIZERS
//
// Holds a list of closures that, when called, will "touch" each thread_local!
// static and force its initialization. This ensures that every TLS value
// that calls `icu_memory!` has actually executed and enqueued its
// registration, instead of lazily initializing much later at first use.
//
thread_local! {
    pub static TLS_INITIALIZERS: RefCell<Vec<fn()>> = const {
        RefCell::new(Vec::new())
    };
}

//
// TLS_PENDING_REGISTRATIONS
//
// Queue of memory registrations produced during TLS initialization via
// `icu_memory!`. Each entry is (id, crate_name, label).
// These are deferred until `flush_pending_registrations()` is called,
// which validates and inserts them into the global MemoryRegistry.
//
thread_local! {
    pub static TLS_PENDING_REGISTRATIONS: RefCell<Vec<(u8, &'static str, &'static str)>> = const {
        RefCell::new(Vec::new())
    };
}

///
/// Called during `canister_init` / `post_upgrade` to process all deferred registrations.
///
/// Panics if any registration fails.
///

pub fn force_init_all_tls() {
    TLS_INITIALIZERS.with(|v| {
        for f in v.borrow().iter() {
            f(); // forces init
        }
    });

    // reserve internal icu range
    MemoryRegistry::reserve_range(ICU_MEMORY_MIN, ICU_MEMORY_MAX, "icu").unwrap();

    // sort the queue, and drain it to register
    TLS_PENDING_REGISTRATIONS.with(|q| {
        let mut regs = q.borrow_mut();

        regs.sort_by_key(|(id, _, _)| *id);
        for (id, crate_name, label) in regs.drain(..) {
            MemoryRegistry::register(id, crate_name, label).unwrap();
        }
    });

    // summary logs: one per range
    MEMORY_RANGES.with_borrow(|ranges| {
        MEMORY_REGISTRY.with_borrow(|registry| {
            for entry in ranges.iter() {
                let crate_name = entry.key();
                let range = entry.value();

                let count = registry.iter().filter(|e| range.contains(*e.key())).count();

                log!(
                    Log::Info,
                    "💾 memory.range: {} [{}-{}] ({}/{} used)",
                    crate_name,
                    range.start,
                    range.end,
                    count,
                    range.end - range.start,
                );
            }
        });
    });
}

///
/// MemoryRegistryError
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryError {
    #[error("ID {0} is already registered with type {1}, tried to register type {2}")]
    AlreadyRegistered(u8, String, String),

    #[error("crate `{0}` attempted to register ID {1}, but it is outside its allowed ranges")]
    OutOfRange(String, u8),

    #[error("crate `{0}` range {1}-{2} overlaps with crate `{3}` range {4}-{5}")]
    Overlap(String, u8, u8, String, u8, u8),
}

///
/// MemoryRange
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryRange {
    pub start: u8,
    pub end: u8,
    pub crate_name: BoundedString64,
    pub created_at: u64,
}

impl MemoryRange {
    #[must_use]
    pub fn new(start: u8, end: u8, crate_name: &str) -> Self {
        Self {
            start,
            end,
            crate_name: BoundedString64::new(crate_name),
            created_at: now_secs(),
        }
    }

    #[must_use]
    pub fn contains(&self, id: u8) -> bool {
        (self.start..=self.end).contains(&id)
    }
}

impl_storable_bounded!(MemoryRange, 128, false);

///
/// MemoryRegistryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MemoryRegistryEntry {
    pub label: BoundedString64,
    pub created_at: u64,
}

impl MemoryRegistryEntry {
    #[must_use]
    pub fn new(label: &str) -> Self {
        Self {
            label: BoundedString64::new(label),
            created_at: now_secs(),
        }
    }
}

impl_storable_bounded!(MemoryRegistryEntry, 128, false);

///
/// MemoryRegistry
///

pub struct MemoryRegistry;

pub type MemoryRegistryView = Vec<(u8, MemoryRegistryEntry)>;

impl MemoryRegistry {
    #[must_use]
    pub fn is_empty() -> bool {
        MEMORY_REGISTRY.with_borrow(|map| map.is_empty())
    }

    /// Register an ID, enforcing crate’s allowed range.
    pub fn register(id: u8, crate_name: &str, label: &str) -> Result<(), Error> {
        // immutable borrow first: check ranges and existing registry entry
        let allowed = MEMORY_RANGES.with_borrow(|ranges| {
            ranges
                .get(&crate_name.to_string())
                .is_some_and(|r| r.contains(id))
        });
        if !allowed {
            return Err(MemoryError::from(MemoryRegistryError::OutOfRange(
                crate_name.to_string(),
                id,
            )))?;
        }

        // check already registered
        let existing = MEMORY_REGISTRY.with_borrow(|map| map.get(&id));
        if let Some(existing) = existing {
            if existing.label.as_ref() != label {
                return Err(MemoryError::from(MemoryRegistryError::AlreadyRegistered(
                    id,
                    existing.label.to_string(),
                    label.to_string(),
                )))?;
            }
            return Ok(()); // idempotent case
        }

        // now borrow mutably for insertion
        MEMORY_REGISTRY.with_borrow_mut(|map| {
            map.insert(id, MemoryRegistryEntry::new(label));
        });

        Ok(())
    }

    /// Reserve a block of memory IDs for a crate.
    pub fn reserve_range(start: u8, end: u8, crate_name: &str) -> Result<(), Error> {
        if start > end {
            return Err(MemoryError::from(MemoryRegistryError::OutOfRange(
                crate_name.to_string(),
                start,
            )))?;
        }

        // immutable borrow first
        let conflict = MEMORY_RANGES.with_borrow(|ranges| {
            if ranges.contains_key(&crate_name.to_string()) {
                return Some(MemoryRegistryError::AlreadyRegistered(
                    start,
                    crate_name.to_string(),
                    "range already exists".to_string(),
                ));
            }

            for entry in ranges.iter() {
                let other_crate = entry.key();
                let other_range = entry.value();

                if !(end < other_range.start || start > other_range.end) {
                    return Some(MemoryRegistryError::Overlap(
                        crate_name.to_string(),
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
            return Err(MemoryError::from(err))?;
        }

        // now borrow mutably once for insertion
        MEMORY_RANGES.with_borrow_mut(|ranges| {
            let range = MemoryRange::new(start, end, crate_name);
            ranges.insert(crate_name.to_string(), range);
        });

        Ok(())
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

    pub fn clear() {
        MEMORY_REGISTRY.with_borrow_mut(StableBTreeMap::clear);
        MEMORY_RANGES.with_borrow_mut(StableBTreeMap::clear);
    }
}
