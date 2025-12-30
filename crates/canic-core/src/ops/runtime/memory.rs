pub use canic_memory::{MemoryRange, MemoryRegistryEntry, MemoryRegistryView};

use crate::{
    CRATE_NAME, Error, ThisError, log,
    log::Topic,
    model::memory::{CANIC_MEMORY_MAX, CANIC_MEMORY_MIN},
    ops::runtime::RuntimeOpsError,
};
use canic_memory::{
    MemoryRegistryError,
    ops::{MemoryRegistryOps as BaseRegistryOps, MemoryRegistrySummary},
};

///
/// MemoryRegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryOpsError {
    #[error(transparent)]
    MemoryRegistryOpsError(#[from] MemoryRegistryError),
}

impl From<MemoryRegistryOpsError> for Error {
    fn from(err: MemoryRegistryOpsError) -> Self {
        RuntimeOpsError::from(err).into()
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
        let summary =
            BaseRegistryOps::init_memory(Some((CRATE_NAME, CANIC_MEMORY_MIN, CANIC_MEMORY_MAX)))
                .map_err(MemoryRegistryOpsError::from)?;

        Self::log_summary(&summary);

        Ok(())
    }

    fn log_summary(summary: &MemoryRegistrySummary) {
        if !crate::log::is_ready() {
            // During early init, logging may not be ready; avoid accidental traps.
            return;
        }

        let entries = &summary.entries;

        for (crate_name, range) in &summary.ranges {
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
    pub fn export() -> MemoryRegistryView {
        BaseRegistryOps::export()
    }

    #[must_use]
    pub fn export_ranges() -> Vec<(String, MemoryRange)> {
        BaseRegistryOps::export_ranges()
    }

    #[must_use]
    pub fn get(id: u8) -> Option<MemoryRegistryEntry> {
        BaseRegistryOps::get(id)
    }
}
