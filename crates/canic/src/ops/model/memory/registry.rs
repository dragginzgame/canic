pub use crate::model::memory::registry::{MemoryRange, MemoryRegistryEntry};

use crate::{
    CRATE_NAME, Error, ThisError, log,
    log::Topic,
    model::memory::{
        CANIC_MEMORY_MAX, CANIC_MEMORY_MIN,
        registry::{
            MemoryRegistry, MemoryRegistryError, drain_pending_ranges, drain_pending_registrations,
        },
    },
    ops::model::memory::MemoryOpsError,
};

///
/// MemoryRegistryDto
/// DTO view of the memory registry
///

pub type MemoryRegistryDto = Vec<(u8, MemoryRegistryEntry)>;

///
/// MemoryRegistryOpsError
///
/// Map model-level memory registry errors into the crate-level Error.
/// This keeps the mapping at the ops boundary instead of inside the model.
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryOpsError {
    #[error(transparent)]
    MemoryRegistryError(#[from] MemoryRegistryError),
}

impl From<MemoryRegistryOpsError> for Error {
    fn from(err: MemoryRegistryOpsError) -> Self {
        MemoryOpsError::from(err).into()
    }
}

///
/// MemoryRegistryOps
/// Ops wrapper around the global memory registry.
///
pub struct MemoryRegistryOps;

impl MemoryRegistryOps {
    /// Initialise all registered memory segments and ranges.
    ///
    /// - Reserves the internal canic range.
    /// - Applies all deferred range reservations.
    /// - Applies all deferred registrations (sorted by ID).
    /// - Emits summary logs per range.
    pub fn init_memory() -> Result<(), Error> {
        // 1. reserve internal range
        MemoryRegistry::reserve_range(CRATE_NAME, CANIC_MEMORY_MIN, CANIC_MEMORY_MAX)?;

        // 2. flush all pending ranges
        let mut ranges = drain_pending_ranges();
        // deterministic order (optional)
        ranges.sort_by_key(|(_, start, _)| *start);
        for (crate_name, start, end) in ranges {
            MemoryRegistry::reserve_range(crate_name, start, end)?;
        }

        // 3. flush all pending registrations
        let mut regs = drain_pending_registrations();
        regs.sort_by_key(|(id, _, _)| *id);
        for (id, crate_name, label) in regs {
            MemoryRegistry::register(id, crate_name, label)?;
        }

        // 4. log summary
        Self::log_summary();

        Ok(())
    }

    fn log_summary() {
        let ranges = MemoryRegistry::export_ranges();
        let entries = MemoryRegistry::export();

        for (crate_name, range) in ranges {
            let count = entries.iter().filter(|(id, _)| range.contains(*id)).count();

            log!(
                Topic::Memory,
                Info,
                "💾 memory.range: {} [{}-{}] ({}/{} slots used)",
                crate_name,
                range.start,
                range.end,
                count,
                range.end - range.start + 1,
            );
        }
    }

    #[must_use]
    pub fn export() -> MemoryRegistryDto {
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
