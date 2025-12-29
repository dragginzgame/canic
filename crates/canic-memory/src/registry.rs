//! NOTE: All stable registry access is TLS-thread-local.
//! This ensures atomicity on the IC’s single-threaded execution model.
use crate::{impl_storable_bounded, manager::MEMORY_MANAGER};
use candid::CandidType;
use canic_cdk::{
    structures::{
        BTreeMap as StableBTreeMap, DefaultMemoryImpl,
        memory::{MemoryId, VirtualMemory},
    },
    types::BoundedString256,
    utils::time::now_secs,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// Reserved for the registry system itself
///
pub const MEMORY_REGISTRY_ID: u8 = 0;
pub const MEMORY_RANGES_ID: u8 = 1;

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

// public as it gets called from macros
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

// public as it gets called from macros
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

    #[error("crate `{0}` key too long ({1} bytes), max 256")]
    CrateKeyTooLong(String, usize),

    #[error("crate `{0}` already has a reserved range")]
    DuplicateRange(String),

    #[error("crate `{0}` provided invalid range {1}-{2} (start > end)")]
    InvalidRange(String, u8, u8),

    #[error("label for crate `{0}` too long ({1} bytes), max 256")]
    LabelTooLong(String, usize),

    #[error("crate `{0}` attempted to register ID {1}, but it is outside its allowed ranges")]
    OutOfRange(String, u8),

    #[error("crate `{0}` range {1}-{2} overlaps with crate `{3}` range {4}-{5}")]
    Overlap(String, u8, u8, String, u8, u8),

    #[error("crate `{0}` has not reserved any memory range")]
    NoRange(String),
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
    pub(crate) fn try_new(
        crate_name: &str,
        start: u8,
        end: u8,
    ) -> Result<Self, MemoryRegistryError> {
        let crate_key = BoundedString256::try_new(crate_name).map_err(|_| {
            MemoryRegistryError::CrateKeyTooLong(crate_name.to_string(), crate_name.len())
        })?;

        Ok(Self {
            crate_key,
            start,
            end,
            created_at: now_secs(),
        })
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
    pub(crate) fn try_new(crate_name: &str, label: &str) -> Result<Self, MemoryRegistryError> {
        let label = BoundedString256::try_new(label)
            .map_err(|_| MemoryRegistryError::LabelTooLong(crate_name.to_string(), label.len()))?;

        Ok(Self {
            label,
            created_at: now_secs(),
        })
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
        let entry = MemoryRegistryEntry::try_new(crate_name, label)?;
        MEMORY_REGISTRY.with_borrow_mut(|map| {
            map.insert(id, entry);
        });

        Ok(())
    }

    /// Reserve a block of memory IDs for a crate.
    ///
    /// Pure domain/model-level function, no logging or unwrap.
    pub fn reserve_range(crate_name: &str, start: u8, end: u8) -> Result<(), MemoryRegistryError> {
        if start > end {
            return Err(MemoryRegistryError::InvalidRange(
                crate_name.to_string(),
                start,
                end,
            ));
        }

        let crate_key = crate_name.to_string();

        // 1. Check for conflicts (existing ranges)
        let conflict = MEMORY_RANGES.with_borrow(|ranges| {
            if let Some(existing) = ranges.get(&crate_key) {
                if existing.start == start && existing.end == end {
                    return None;
                }

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
        let range = MemoryRange::try_new(crate_name, start, end)?;
        MEMORY_RANGES.with_borrow_mut(|ranges| {
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
}

#[cfg(test)]
pub(crate) fn reset_for_tests() {
    MEMORY_REGISTRY.with_borrow_mut(StableBTreeMap::clear);
    MEMORY_RANGES.with_borrow_mut(StableBTreeMap::clear);
    PENDING_REGISTRATIONS.with(|q| q.borrow_mut().clear());
    PENDING_RANGES.with(|q| q.borrow_mut().clear());
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_range_happy_path_and_reject_overlap() {
        reset_for_tests();
        MemoryRegistry::reserve_range("crate_a", 10, 20).unwrap();

        // Overlap with existing should error
        let err = MemoryRegistry::reserve_range("crate_b", 15, 25).unwrap_err();
        assert!(matches!(
            err,
            MemoryRegistryError::Overlap(_, _, _, _, _, _)
        ));

        // Disjoint should succeed
        MemoryRegistry::reserve_range("crate_b", 30, 40).unwrap();

        let ranges = MemoryRegistry::export_ranges();
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn reserve_range_rejects_invalid_order() {
        reset_for_tests();
        let err = MemoryRegistry::reserve_range("crate_a", 5, 4).unwrap_err();
        assert!(matches!(err, MemoryRegistryError::InvalidRange(_, _, _)));
        assert!(MemoryRegistry::export_ranges().is_empty());
    }

    #[test]
    fn register_id_requires_range_and_checks_bounds() {
        reset_for_tests();
        MemoryRegistry::reserve_range("crate_a", 1, 3).unwrap();

        // Out of range
        let err = MemoryRegistry::register(5, "crate_a", "Foo").unwrap_err();
        assert!(matches!(err, MemoryRegistryError::OutOfRange(_, _)));

        // Happy path
        MemoryRegistry::register(2, "crate_a", "Foo").unwrap();

        // Idempotent same label
        MemoryRegistry::register(2, "crate_a", "Foo").unwrap();

        // Different label should error
        let err = MemoryRegistry::register(2, "crate_a", "Bar").unwrap_err();
        assert!(matches!(
            err,
            MemoryRegistryError::AlreadyRegistered(_, _, _)
        ));

        let view = MemoryRegistry::export();
        assert_eq!(view.len(), 1);
        assert_eq!(view[0].0, 2);
    }

    #[test]
    fn pending_queues_drain_in_order() {
        reset_for_tests();
        defer_reserve_range("crate_a", 1, 2);
        defer_reserve_range("crate_b", 3, 4);
        defer_register(1, "crate_a", "A1");
        defer_register(3, "crate_b", "B3");

        let ranges = drain_pending_ranges();
        assert_eq!(ranges, vec![("crate_a", 1, 2), ("crate_b", 3, 4)]);
        let regs = drain_pending_registrations();
        assert_eq!(regs, vec![(1, "crate_a", "A1"), (3, "crate_b", "B3")]);

        // queues are empty after drain
        assert!(drain_pending_ranges().is_empty());
        assert!(drain_pending_registrations().is_empty());
    }

    #[test]
    fn reserve_range_rejects_too_long_crate_key() {
        reset_for_tests();

        let crate_name = "a".repeat(257);
        let err = MemoryRegistry::reserve_range(&crate_name, 1, 2).unwrap_err();
        assert!(matches!(err, MemoryRegistryError::CrateKeyTooLong(_, 257)));
    }

    #[test]
    fn register_rejects_too_long_label() {
        reset_for_tests();
        MemoryRegistry::reserve_range("crate_a", 1, 3).unwrap();

        let label = "a".repeat(257);
        let err = MemoryRegistry::register(2, "crate_a", &label).unwrap_err();
        assert!(matches!(err, MemoryRegistryError::LabelTooLong(_, 257)));
    }
}
