use crate::{
    Error, Log,
    cdk::structures::{
        BTreeMap as StableBTreeMap, DefaultMemoryImpl,
        memory::{MemoryId, VirtualMemory},
    },
    impl_storable_bounded, log,
    memory::{MEMORY_MANAGER, MEMORY_REGISTRY_MEMORY_ID, MemoryError},
    thread_local_register,
    types::BoundedString64,
    utils::time::now_secs,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap};
use thiserror::Error as ThisError;

// MEMORY_REGISTRY
thread_local_register! {
    static MEMORY_REGISTRY: RefCell<StableBTreeMap<u8, MemoryRegistryEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|this| {
                this.get(MemoryId::new(MEMORY_REGISTRY_MEMORY_ID))
            }),
        ));
}

// MEMORY_RANGES
thread_local_register! {
    static MEMORY_RANGES: RefCell<BTreeMap<String, MemoryRange>> = const {
        RefCell::new(BTreeMap::new())
    };
}

// TLS_REGISTRARS
// so that other TLS can enqueue their registrations without breaking rust
thread_local! {
    pub static TLS_REGISTRARS: RefCell<Vec<fn()>> = const { RefCell::new(Vec::new()) };
}

// PENDING_REGISTRATIONS
thread_local! {
    pub static PENDING_REGISTRATIONS: RefCell<Vec<(u8, &'static str, &'static str)>> = const {
        RefCell::new(Vec::new())
    };
}

///
/// Called during `canister_init` / `post_upgrade` to process all deferred registrations.
///
/// Panics if any registration fails.
///

pub fn force_init_all_tls() {
    TLS_REGISTRARS.with(|v| {
        for f in v.borrow().iter() {
            f(); // forces init
        }
    });

    flush_pending_registrations();
}

pub fn flush_pending_registrations() {
    PENDING_REGISTRATIONS.with(|q| {
        // Drain the queue
        for (id, crate_name, label) in q.borrow_mut().drain(..) {
            match MemoryRegistry::register(id, crate_name, label) {
                Ok(_) => {
                    crate::log!(
                        crate::Log::Ok,
                        "📦 Registered memory ID {id} for crate `{crate_name}` (label `{label}`)"
                    );
                }
                Err(err) => {
                    panic!(
                        "❌ memory registration failed for crate `{crate_name}` id {id} (label `{label}`): {err}"
                    );
                }
            }
        }
    });
}

///
/// MemoryRegistryError
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryError {
    #[error("ID {0} is already registered with type {1}, tried to register type {2}")]
    AlreadyRegistered(u8, String, String),

    #[error("memory id {0} is reserved")]
    Reserved(u8),

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
    pub crate_name: String,
    pub created_at: u64,
}

impl MemoryRange {
    #[must_use]
    pub fn new(start: u8, end: u8, crate_name: &str) -> Self {
        Self {
            start,
            end,
            crate_name: crate_name.to_string(),
            created_at: now_secs(),
        }
    }

    #[must_use]
    pub fn contains(&self, id: u8) -> bool {
        (self.start..=self.end).contains(&id)
    }
}

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

    #[must_use]
    pub const fn is_reserved(id: u8) -> bool {
        id == MEMORY_REGISTRY_MEMORY_ID
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
            if ranges.contains_key(crate_name) {
                return Some(MemoryRegistryError::AlreadyRegistered(
                    start,
                    crate_name.to_string(),
                    "range already exists".to_string(),
                ));
            }

            for (other_crate, other_range) in ranges.iter() {
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

            log!(
                Log::Info,
                "🧩 Reserved memory range for crate `{crate_name}`: {start} → {end}",
            );
        });

        Ok(())
    }

    #[must_use]
    pub fn export_ranges() -> Vec<(String, MemoryRange)> {
        MEMORY_RANGES
            .with_borrow(|ranges| ranges.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    /// Register an ID, enforcing crate’s allowed range.
    pub fn register(id: u8, crate_name: &str, label: &str) -> Result<(), Error> {
        if Self::is_reserved(id) {
            return Err(MemoryError::from(MemoryRegistryError::Reserved(id)))?;
        }

        // immutable borrow first: check ranges and existing registry entry
        let allowed = MEMORY_RANGES.with_borrow(|ranges| {
            ranges
                .get(crate_name)
                .map(|r| r.contains(id))
                .unwrap_or(false)
        });

        if !allowed {
            return Err(MemoryError::from(MemoryRegistryError::OutOfRange(
                crate_name.to_string(),
                id,
            )))?;
        }

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

        log!(Log::Info, "🔖 memory.register: {id} ({label}@{crate_name})");

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

    pub fn clear() {
        MEMORY_REGISTRY.with_borrow_mut(StableBTreeMap::clear);
        MEMORY_RANGES.with_borrow_mut(BTreeMap::clear);
    }
}
