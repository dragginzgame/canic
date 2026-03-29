//! Runtime memory registry primitives.
//! Owns TLS setup for memory registry initialization.

use crate::{
    CRATE_NAME, InternalError,
    dto::memory::MemoryRegistryEntry,
    ops::runtime::RuntimeOpsError,
    storage::stable::{CANIC_MEMORY_MAX, CANIC_MEMORY_MIN},
};
use canic_memory::{
    registry::MemoryRegistryError,
    runtime::{
        init_eager_tls,
        registry::{MemoryRegistryInitSummary as RawInitSummary, MemoryRegistryRuntime},
        run_registered_eager_init,
    },
};
use thiserror::Error as ThisError;

///
/// MemoryRegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryOpsError {
    // this error comes from the canic-memory crate
    #[error(transparent)]
    Registry(#[from] MemoryRegistryError),
}

impl From<MemoryRegistryOpsError> for InternalError {
    fn from(err: MemoryRegistryOpsError) -> Self {
        RuntimeOpsError::MemoryRegistryOps(err).into()
    }
}

///
/// MemoryRangeSnapshot
///

#[derive(Clone, Debug)]
pub struct MemoryRangeSnapshot {
    pub crate_name: String,
    pub start: u8,
    pub end: u8,
}

///
/// MemoryRegistryInitSummary
///

#[derive(Clone, Debug)]
pub struct MemoryRegistryInitSummary {
    pub ranges: Vec<MemoryRangeSnapshot>,
    pub entries: Vec<MemoryRegistryEntry>,
}

impl MemoryRegistryInitSummary {
    fn from_raw(summary: RawInitSummary) -> Self {
        let ranges = summary
            .ranges
            .into_iter()
            .map(|(crate_name, range)| MemoryRangeSnapshot {
                crate_name,
                start: range.start,
                end: range.end,
            })
            .collect();

        let entries = summary
            .entries
            .into_iter()
            .map(|(id, entry)| MemoryRegistryEntry {
                id,
                crate_name: entry.crate_name,
                label: entry.label,
            })
            .collect();

        Self { ranges, entries }
    }
}

///
/// MemoryRegistryOps
///

pub struct MemoryRegistryOps;

impl MemoryRegistryOps {
    // Run eager TLS touches before the registry initializes stable-memory slots.
    pub fn init_eager_tls() {
        init_eager_tls();
    }

    // Run registered eager-init hooks before the registry commits deferred items.
    pub fn run_registered_eager_init() {
        run_registered_eager_init();
    }

    // Initialize the stable-memory registry for this crate and summarize the layout.
    pub(crate) fn init_registry() -> Result<MemoryRegistryInitSummary, InternalError> {
        let summary =
            MemoryRegistryRuntime::init(Some((CRATE_NAME, CANIC_MEMORY_MIN, CANIC_MEMORY_MAX)))
                .map_err(MemoryRegistryOpsError::from)?;

        Ok(MemoryRegistryInitSummary::from_raw(summary))
    }

    // Run the full synchronous Canic memory bootstrap and return the committed layout.
    pub fn bootstrap_registry() -> Result<MemoryRegistryInitSummary, InternalError> {
        Self::init_eager_tls();
        Self::run_registered_eager_init();
        Self::init_registry()
    }

    #[cfg(target_arch = "wasm32")]
    #[must_use]
    pub fn is_initialized() -> bool {
        MemoryRegistryRuntime::is_initialized()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn ensure_bootstrap() -> Result<(), InternalError> {
        if Self::is_initialized() {
            return Ok(());
        }

        let _ = Self::bootstrap_registry()?;
        Ok(())
    }

    #[must_use]
    pub fn snapshot_entries() -> Vec<MemoryRegistryEntry> {
        MemoryRegistryRuntime::snapshot_entries()
            .into_iter()
            .map(|(id, entry)| MemoryRegistryEntry {
                id,
                crate_name: entry.crate_name,
                label: entry.label,
            })
            .collect()
    }
}
