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
/// MemoryRangeEntry
///

#[derive(Clone, Debug)]
pub struct MemoryRangeEntry {
    pub owner: String,
    pub range: MemoryRange,
}

///
/// MemoryRangeSnapshot
///

#[derive(Clone, Debug)]
pub struct MemoryRangeSnapshot {
    pub owner: String,
    pub range: MemoryRange,
    pub entries: Vec<(u8, MemoryRegistryEntry)>,
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

    #[error("memory range is invalid: start={start} end={end}")]
    InvalidRange { start: u8, end: u8 },

    #[error("memory id {0} is already registered; each memory id must be globally unique")]
    DuplicateId(u8),

    #[error("memory id {id} has no reserved range for crate '{crate_name}'")]
    NoReservedRange { crate_name: String, id: u8 },

    #[error(
        "memory id {id} reserved to crate '{owner}' [{owner_start}-{owner_end}], not '{crate_name}'"
    )]
    IdOwnedByOther {
        crate_name: String,
        id: u8,
        owner: String,
        owner_start: u8,
        owner_end: u8,
    },

    #[error("memory id {id} is outside reserved ranges for crate '{crate_name}'")]
    IdOutOfRange { crate_name: String, id: u8 },

    #[error(
        "memory id {id} is reserved for stable-structures internals and cannot be used by application code"
    )]
    ReservedInternalId { id: u8 },
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
        if start > end {
            return Err(MemoryRegistryError::InvalidRange { start, end });
        }
        validate_range_excludes_reserved_internal_id(start, end)?;

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
        validate_non_internal_id(id)?;
        validate_registration_range(crate_name, id)?;

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

    /// Export all reserved ranges with explicit owners.
    #[must_use]
    pub fn export_range_entries() -> Vec<MemoryRangeEntry> {
        RESERVED_RANGES.with_borrow(|ranges| {
            ranges
                .iter()
                .map(|(owner, range)| MemoryRangeEntry {
                    owner: owner.clone(),
                    range: *range,
                })
                .collect()
        })
    }

    /// Export registry entries grouped by reserved range.
    #[must_use]
    pub fn export_ids_by_range() -> Vec<MemoryRangeSnapshot> {
        let mut ranges = RESERVED_RANGES.with_borrow(std::clone::Clone::clone);
        let entries = REGISTRY.with_borrow(std::clone::Clone::clone);

        ranges.sort_by_key(|(_, range)| range.start);

        ranges
            .into_iter()
            .map(|(owner, range)| {
                let entries = entries
                    .iter()
                    .filter(|(id, _)| range.contains(**id))
                    .map(|(id, entry)| (*id, entry.clone()))
                    .collect();

                MemoryRangeSnapshot {
                    owner,
                    range,
                    entries,
                }
            })
            .collect()
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

pub fn defer_reserve_range(
    crate_name: &str,
    start: u8,
    end: u8,
) -> Result<(), MemoryRegistryError> {
    if start > end {
        return Err(MemoryRegistryError::InvalidRange { start, end });
    }
    validate_range_excludes_reserved_internal_id(start, end)?;

    // Queue range reservations for runtime init to apply deterministically.
    PENDING_RANGES.with_borrow_mut(|ranges| {
        ranges.push((crate_name.to_string(), start, end));
    });

    Ok(())
}

pub fn defer_register(id: u8, crate_name: &str, label: &str) -> Result<(), MemoryRegistryError> {
    validate_non_internal_id(id)?;

    // Queue ID registrations for runtime init to apply after ranges are reserved.
    PENDING_REGISTRATIONS.with_borrow_mut(|regs| {
        regs.push((id, crate_name.to_string(), label.to_string()));
    });

    Ok(())
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

const INTERNAL_RESERVED_MEMORY_ID: u8 = u8::MAX;

const fn validate_non_internal_id(id: u8) -> Result<(), MemoryRegistryError> {
    if id == INTERNAL_RESERVED_MEMORY_ID {
        return Err(MemoryRegistryError::ReservedInternalId { id });
    }
    Ok(())
}

const fn validate_range_excludes_reserved_internal_id(
    _start: u8,
    end: u8,
) -> Result<(), MemoryRegistryError> {
    if end == INTERNAL_RESERVED_MEMORY_ID {
        return Err(MemoryRegistryError::ReservedInternalId {
            id: INTERNAL_RESERVED_MEMORY_ID,
        });
    }
    Ok(())
}

fn validate_registration_range(crate_name: &str, id: u8) -> Result<(), MemoryRegistryError> {
    let mut has_range = false;
    let mut owner_match = false;
    let mut owner_for_id: Option<(String, MemoryRange)> = None;

    RESERVED_RANGES.with_borrow(|ranges| {
        for (owner, range) in ranges {
            if owner == crate_name {
                has_range = true;
                if range.contains(id) {
                    owner_match = true;
                    break;
                }
            }

            if owner_for_id.is_none() && range.contains(id) {
                owner_for_id = Some((owner.clone(), *range));
            }
        }
    });

    if owner_match {
        return Ok(());
    }

    if !has_range {
        return Err(MemoryRegistryError::NoReservedRange {
            crate_name: crate_name.to_string(),
            id,
        });
    }

    if let Some((owner, range)) = owner_for_id {
        return Err(MemoryRegistryError::IdOwnedByOther {
            crate_name: crate_name.to_string(),
            id,
            owner,
            owner_start: range.start,
            owner_end: range.end,
        });
    }

    Err(MemoryRegistryError::IdOutOfRange {
        crate_name: crate_name.to_string(),
        id,
    })
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_in_range() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 1, 3).expect("reserve range");
        MemoryRegistry::register(2, "crate_a", "slot").expect("register in range");
    }

    #[test]
    fn rejects_unreserved() {
        reset_for_tests();

        let err = MemoryRegistry::register(2, "crate_a", "slot").expect_err("missing range");
        assert!(matches!(err, MemoryRegistryError::NoReservedRange { .. }));
    }

    #[test]
    fn rejects_other_owner() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 1, 3).expect("reserve range A");
        MemoryRegistry::reserve_range("crate_b", 4, 6).expect("reserve range B");

        let err = MemoryRegistry::register(2, "crate_b", "slot").expect_err("owned by other");
        assert!(matches!(err, MemoryRegistryError::IdOwnedByOther { .. }));
    }

    #[test]
    fn export_ids_by_range_groups_entries() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 1, 3).expect("reserve range A");
        MemoryRegistry::reserve_range("crate_b", 4, 6).expect("reserve range B");
        MemoryRegistry::register(1, "crate_a", "a1").expect("register a1");
        MemoryRegistry::register(5, "crate_b", "b5").expect("register b5");

        let snapshots = MemoryRegistry::export_ids_by_range();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].entries.len(), 1);
        assert_eq!(snapshots[1].entries.len(), 1);
    }

    #[test]
    fn rejects_internal_reserved_id_on_register() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 1, 254).expect("reserve range");
        let err = MemoryRegistry::register(u8::MAX, "crate_a", "slot")
            .expect_err("reserved id should be rejected");
        assert!(matches!(
            err,
            MemoryRegistryError::ReservedInternalId { .. }
        ));
    }

    #[test]
    fn rejects_internal_reserved_id_on_range_reservation() {
        reset_for_tests();

        let err = MemoryRegistry::reserve_range("crate_a", 250, u8::MAX)
            .expect_err("reserved internal id must not be reservable");
        assert!(matches!(
            err,
            MemoryRegistryError::ReservedInternalId { .. }
        ));
    }

    #[test]
    fn rejects_internal_reserved_id_on_deferred_register() {
        reset_for_tests();

        let err = defer_register(u8::MAX, "crate_a", "slot")
            .expect_err("reserved id should fail before init");
        assert!(matches!(
            err,
            MemoryRegistryError::ReservedInternalId { .. }
        ));
    }

    #[test]
    fn rejects_internal_reserved_id_on_deferred_range_reservation() {
        reset_for_tests();

        let err = defer_reserve_range("crate_a", 240, u8::MAX)
            .expect_err("reserved id should fail before init");
        assert!(matches!(
            err,
            MemoryRegistryError::ReservedInternalId { .. }
        ));
    }
}
