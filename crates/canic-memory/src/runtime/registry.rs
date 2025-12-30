use crate::registry::{
    MemoryRange, MemoryRegistry, MemoryRegistryEntry, MemoryRegistryError, drain_pending_ranges,
    drain_pending_registrations,
};

///
/// MemoryRegistryInitSummary
///
/// Substrate-level summary of registry state after initialization.
/// This is intended for diagnostics and testing only.
/// It is NOT a stable API contract or external view.
///

#[derive(Debug)]
pub struct MemoryRegistryInitSummary {
    pub ranges: Vec<(String, MemoryRange)>,
    pub entries: Vec<(u8, MemoryRegistryEntry)>,
}

///
/// MemoryRegistryRuntime
///
/// Substrate runtime controller responsible for initializing the
/// global memory registry.
///
/// This type performs mechanical coordination only:
/// - ordering
/// - conflict detection
/// - idempotent initialization
///
/// It encodes no application semantics.
///
pub struct MemoryRegistryRuntime;

impl MemoryRegistryRuntime {
    /// Initialize the memory registry.
    ///
    /// - Optionally reserves an initial range for the caller.
    /// - Applies all deferred range reservations.
    /// - Applies all deferred ID registrations.
    ///
    /// This function is idempotent for the same initial range.
    pub fn init(
        initial_range: Option<(&str, u8, u8)>,
    ) -> Result<MemoryRegistryInitSummary, MemoryRegistryError> {
        // Reserve the caller's initial range first (if provided)
        if let Some((crate_name, start, end)) = initial_range {
            MemoryRegistry::reserve_range(crate_name, start, end)?;
        }

        // Apply deferred range reservations deterministically
        let mut ranges = drain_pending_ranges();
        ranges.sort_by_key(|(_, start, _)| *start);
        for (crate_name, start, end) in ranges {
            MemoryRegistry::reserve_range(&crate_name, start, end)?;
        }

        // Apply deferred registrations deterministically
        let mut regs = drain_pending_registrations();
        regs.sort_by_key(|(id, _, _)| *id);
        for (id, crate_name, label) in regs {
            MemoryRegistry::register(id, &crate_name, &label)?;
        }

        Ok(MemoryRegistryInitSummary {
            ranges: MemoryRegistry::export_ranges(),
            entries: MemoryRegistry::export(),
        })
    }

    /// Snapshot all registry entries.
    #[must_use]
    pub fn snapshot_entries() -> Vec<(u8, MemoryRegistryEntry)> {
        MemoryRegistry::export()
    }

    /// Snapshot all reserved memory ranges.
    #[must_use]
    pub fn snapshot_ranges() -> Vec<(String, MemoryRange)> {
        MemoryRegistry::export_ranges()
    }

    /// Retrieve a single registry entry by ID.
    #[must_use]
    pub fn get(id: u8) -> Option<MemoryRegistryEntry> {
        MemoryRegistry::get(id)
    }
}

//
// TESTS
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{defer_register, defer_reserve_range, reset_for_tests};

    #[test]
    fn init_applies_initial_and_pending() {
        reset_for_tests();
        defer_reserve_range("crate_b", 5, 6);
        defer_register(5, "crate_b", "B5");

        let summary =
            MemoryRegistryRuntime::init(Some(("crate_a", 1, 3))).expect("init should succeed");

        assert_eq!(summary.ranges.len(), 2);
        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].0, 5);
        assert_eq!(summary.entries[0].1.label, "B5");
    }

    #[test]
    fn init_is_idempotent_for_same_initial_range() {
        reset_for_tests();

        MemoryRegistryRuntime::init(Some(("crate_a", 1, 3))).expect("first init should succeed");
        MemoryRegistryRuntime::init(Some(("crate_a", 1, 3))).expect("second init should succeed");
    }

    #[test]
    fn init_returns_error_on_conflict() {
        reset_for_tests();
        defer_reserve_range("crate_a", 1, 3);
        defer_reserve_range("crate_b", 3, 4);

        let err = MemoryRegistryRuntime::init(None).unwrap_err();
        assert!(matches!(err, MemoryRegistryError::Overlap { .. }));
    }
}
