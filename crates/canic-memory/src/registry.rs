use crate::ThisError;
use std::{cell::RefCell, collections::BTreeMap};

///
/// MemoryRange
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryRange {
    pub start: u8,
    pub end: u8,
}

impl MemoryRange {
    #[must_use]
    pub const fn contains(&self, id: u8) -> bool {
        id >= self.start && id <= self.end
    }
}

///
/// MemoryRegistryEntry
///

#[derive(Clone, Debug)]
pub struct MemoryRegistryEntry {
    pub crate_name: String,
    pub label: String,
}

///
/// MemoryRegistryError
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryError {
    #[error(
        "memory range overlap: crate '{existing_crate}' [{existing_start}-{existing_end}]
conflicts with crate '{new_crate}' [{new_start}-{new_end}]"
    )]
    Overlap {
        existing_crate: String,
        existing_start: u8,
        existing_end: u8,
        new_crate: String,
        new_start: u8,
        new_end: u8,
    },

    #[error("memory id {0} is already registered; each memory id must be globally unique")]
    DuplicateId(u8),
}

//
// Internal global state (substrate-level, single-threaded)
//

thread_local! {
    static RESERVED_RANGES: RefCell<Vec<(String, MemoryRange)>> = const { RefCell::new(Vec::new()) };
    static REGISTRY: RefCell<BTreeMap<u8, MemoryRegistryEntry>> = const { RefCell::new(BTreeMap::new()) };

    // Deferred registrations (used before init)
    static PENDING_RANGES: RefCell<Vec<(String, u8, u8)>> = const { RefCell::new(Vec::new()) };
    static PENDING_REGISTRATIONS: RefCell<Vec<(u8, String, String)>> = const { RefCell::new(Vec::new()) };
}

///
/// MemoryRegistry
///
/// Canonical substrate registry for stable memory IDs.
///
pub struct MemoryRegistry;

impl MemoryRegistry {
    /// Reserve a memory range for a crate.
    pub fn reserve_range(crate_name: &str, start: u8, end: u8) -> Result<(), MemoryRegistryError> {
        let range = MemoryRange { start, end };

        RESERVED_RANGES.with_borrow(|ranges| {
            for (existing_crate, existing_range) in ranges {
                if ranges_overlap(*existing_range, range) {
                    if existing_crate == crate_name
                        && existing_range.start == start
                        && existing_range.end == end
                    {
                        // Allow exact duplicate reservations for idempotent init.
                        return Ok(());
                    }
                    return Err(MemoryRegistryError::Overlap {
                        existing_crate: existing_crate.clone(),
                        existing_start: existing_range.start,
                        existing_end: existing_range.end,
                        new_crate: crate_name.to_string(),
                        new_start: start,
                        new_end: end,
                    });
                }
            }

            Ok(())
        })?;

        RESERVED_RANGES.with_borrow_mut(|ranges| {
            ranges.push((crate_name.to_string(), range));
        });

        Ok(())
    }

    /// Register a memory ID.
    pub fn register(id: u8, crate_name: &str, label: &str) -> Result<(), MemoryRegistryError> {
        REGISTRY.with_borrow(|reg| {
            if reg.contains_key(&id) {
                return Err(MemoryRegistryError::DuplicateId(id));
            }
            Ok(())
        })?;

        REGISTRY.with_borrow_mut(|reg| {
            reg.insert(
                id,
                MemoryRegistryEntry {
                    crate_name: crate_name.to_string(),
                    label: label.to_string(),
                },
            );
        });

        Ok(())
    }

    /// Export all registered entries (canonical snapshot).
    #[must_use]
    pub fn export() -> Vec<(u8, MemoryRegistryEntry)> {
        REGISTRY.with_borrow(|reg| reg.iter().map(|(k, v)| (*k, v.clone())).collect())
    }

    /// Export all reserved ranges.
    #[must_use]
    pub fn export_ranges() -> Vec<(String, MemoryRange)> {
        RESERVED_RANGES.with_borrow(std::clone::Clone::clone)
    }

    /// Retrieve a single registry entry.
    #[must_use]
    pub fn get(id: u8) -> Option<MemoryRegistryEntry> {
        REGISTRY.with_borrow(|reg| reg.get(&id).cloned())
    }
}

//
// Deferred registration helpers (used before runtime init)
//

pub fn defer_reserve_range(crate_name: &str, start: u8, end: u8) {
    PENDING_RANGES.with_borrow_mut(|ranges| {
        ranges.push((crate_name.to_string(), start, end));
    });
}

pub fn defer_register(id: u8, crate_name: &str, label: &str) {
    PENDING_REGISTRATIONS.with_borrow_mut(|regs| {
        regs.push((id, crate_name.to_string(), label.to_string()));
    });
}

#[must_use]
pub fn drain_pending_ranges() -> Vec<(String, u8, u8)> {
    PENDING_RANGES.with_borrow_mut(std::mem::take)
}

#[must_use]
pub fn drain_pending_registrations() -> Vec<(u8, String, String)> {
    PENDING_REGISTRATIONS.with_borrow_mut(std::mem::take)
}

//
// Test-only helpers
//

#[cfg(test)]
pub fn reset_for_tests() {
    RESERVED_RANGES.with_borrow_mut(Vec::clear);
    REGISTRY.with_borrow_mut(BTreeMap::clear);
    PENDING_RANGES.with_borrow_mut(Vec::clear);
    PENDING_REGISTRATIONS.with_borrow_mut(Vec::clear);
}

//
// Internal helpers
//

const fn ranges_overlap(a: MemoryRange, b: MemoryRange) -> bool {
    a.start <= b.end && b.start <= a.end
}
