use crate::registry::{
    MemoryRange, MemoryRegistry, MemoryRegistryEntry, MemoryRegistryError, MemoryRegistryView,
    drain_pending_ranges, drain_pending_registrations,
};

///
/// MemoryRegistrySummary
/// Summary of memory registry state after initialization.
///

#[derive(Debug)]
pub struct MemoryRegistrySummary {
    pub ranges: Vec<(String, MemoryRange)>,
    pub entries: MemoryRegistryView,
}

///
/// MemoryRegistryOps
/// Ops wrapper around the global memory registry.
///

pub struct MemoryRegistryOps;

impl MemoryRegistryOps {
    /// Initialise registered memory segments and ranges.
    ///
    /// - Optionally reserves an initial range for the current crate.
    /// - Applies all deferred range reservations.
    /// - Applies all deferred registrations (sorted by ID).
    pub fn init_memory(
        initial_range: Option<(&str, u8, u8)>,
    ) -> Result<MemoryRegistrySummary, MemoryRegistryError> {
        if let Some((crate_name, start, end)) = initial_range {
            MemoryRegistry::reserve_range(crate_name, start, end)?;
        }

        let mut ranges = drain_pending_ranges();
        ranges.sort_by_key(|(_, start, _)| *start);
        for (crate_name, start, end) in ranges {
            MemoryRegistry::reserve_range(crate_name, start, end)?;
        }

        let mut regs = drain_pending_registrations();
        regs.sort_by_key(|(id, _, _)| *id);
        for (id, crate_name, label) in regs {
            MemoryRegistry::register(id, crate_name, label)?;
        }

        Ok(MemoryRegistrySummary {
            ranges: MemoryRegistry::export_ranges(),
            entries: MemoryRegistry::export(),
        })
    }

    #[must_use]
    pub fn export() -> MemoryRegistryView {
        MemoryRegistry::export()
    }

    #[must_use]
    pub fn export_ranges() -> Vec<(String, MemoryRange)> {
        MemoryRegistry::export_ranges()
    }

    #[must_use]
    pub fn get(id: u8) -> Option<MemoryRegistryEntry> {
        MemoryRegistry::get(id)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{defer_register, defer_reserve_range, reset_for_tests};

    #[test]
    fn init_memory_applies_initial_and_pending() {
        reset_for_tests();
        defer_reserve_range("crate_b", 5, 6);
        defer_register(5, "crate_b", "B5");

        let summary =
            MemoryRegistryOps::init_memory(Some(("crate_a", 1, 3))).expect("init should succeed");

        assert_eq!(summary.ranges.len(), 2);
        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].0, 5);
        assert_eq!(summary.entries[0].1.label.as_ref(), "B5");
    }

    #[test]
    fn init_memory_is_idempotent_for_same_initial_range() {
        reset_for_tests();

        MemoryRegistryOps::init_memory(Some(("crate_a", 1, 3))).expect("first init should succeed");
        MemoryRegistryOps::init_memory(Some(("crate_a", 1, 3)))
            .expect("second init should succeed");
    }

    #[test]
    fn init_memory_returns_error_on_conflict() {
        reset_for_tests();
        defer_reserve_range("crate_a", 1, 3);
        defer_reserve_range("crate_b", 3, 4);

        let err = MemoryRegistryOps::init_memory(None).unwrap_err();
        assert!(matches!(
            err,
            MemoryRegistryError::Overlap(_, _, _, _, _, _)
        ));
    }
}
