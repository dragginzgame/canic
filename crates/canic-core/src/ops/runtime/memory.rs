use crate::{
    CRATE_NAME, Error, ThisError,
    dto::memory::{MemoryRegistryEntryView, MemoryRegistryView},
    log,
    log::Topic,
    model::memory::{CANIC_MEMORY_MAX, CANIC_MEMORY_MIN},
    ops::runtime::RuntimeOpsError,
};
use canic_memory::{
    registry::{MemoryRange, MemoryRegistryError},
    runtime::{
        init_eager_tls,
        registry::{MemoryRegistryInitSummary, MemoryRegistryRuntime},
    },
};

///
/// MemoryOpsError
///

#[derive(Debug, ThisError)]
pub enum MemoryOpsError {
    #[error(transparent)]
    Registry(#[from] MemoryRegistryError),
}

impl From<MemoryOpsError> for Error {
    fn from(err: MemoryOpsError) -> Self {
        RuntimeOpsError::MemoryOpsError(err).into()
    }
}

///
/// MemoryOps
///
/// Platform-level runtime initialization for canic-memory.
///
/// This is a single-step, idempotent operation that:
/// - eagerly initializes TLS
/// - applies deferred memory range registrations
///
/// Must be called during lifecycle bootstrap.
///

pub struct MemoryOps;

impl MemoryOps {
    pub fn init() -> Result<(), Error> {
        // Ensure TLS-backed globals are initialized
        init_eager_tls();

        // Initialize the memory registry with CANIC's reserved range
        let summary =
            MemoryRegistryRuntime::init(Some((CRATE_NAME, CANIC_MEMORY_MIN, CANIC_MEMORY_MAX)))
                .map_err(MemoryOpsError::from)?;

        Self::log_summary(&summary);

        Ok(())
    }

    pub fn init_memory() -> Result<(), Error> {
        Self::init()
    }

    #[must_use]
    pub fn export_view() -> MemoryRegistryView {
        let entries = MemoryRegistryRuntime::snapshot_entries()
            .into_iter()
            .map(|(id, entry)| MemoryRegistryEntryView {
                id,
                crate_name: entry.crate_name,
                label: entry.label,
            })
            .collect();

        MemoryRegistryView { entries }
    }

    #[must_use]
    pub fn export_ranges() -> Vec<(String, MemoryRange)> {
        MemoryRegistryRuntime::snapshot_ranges()
    }

    #[must_use]
    pub fn get(id: u8) -> Option<MemoryRegistryEntryView> {
        MemoryRegistryRuntime::get(id).map(|entry| MemoryRegistryEntryView {
            id,
            crate_name: entry.crate_name,
            label: entry.label,
        })
    }

    fn log_summary(summary: &MemoryRegistryInitSummary) {
        if !crate::log::is_ready() {
            return;
        }

        for (crate_name, range) in &summary.ranges {
            let used = summary
                .entries
                .iter()
                .filter(|(id, _)| range.contains(*id))
                .count();

            log!(
                Topic::Memory,
                Info,
                "💾 memory.range: {} [{}-{}] ({}/{} slots used)",
                crate_name,
                range.start,
                range.end,
                used,
                range.end - range.start + 1,
            );
        }
    }
}
